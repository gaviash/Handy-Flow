#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::apple_intelligence;
use crate::audio_feedback::{play_feedback_sound, play_feedback_sound_blocking, SoundType};
use crate::audio_toolkit::VadPolicy;
use crate::features::{feature_target, Feature, Reasoning};
use crate::managers::audio::{recording_notice_code, AudioRecordingManager};
use crate::managers::history::HistoryManager;
use crate::managers::model::ModelManager;
use crate::managers::transcription::policy::{live_decode, LiveDecode, ModelTraits};
use crate::managers::transcription::StreamWorkKind;
use crate::managers::transcription::{Transcription, TranscriptionManager};
use crate::notices::{self, Notice, NoticeCode, NoticeSource};
use crate::settings::{get_settings, AppSettings, OverlayStyle, APPLE_INTELLIGENCE_PROVIDER_ID};
use crate::shortcut;
use crate::tray::{set_tray_state, TrayIconState};
use crate::utils::{
    self, show_processing_overlay, show_recording_overlay, show_transcribing_overlay,
};
use crate::TranscriptionCoordinator;
use log::{debug, error, warn};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tauri_specta::Event;

/// How a dictation ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionOutcome {
    /// The text was typed.
    Typed,
    /// Nothing was heard, so nothing was typed.
    Empty,
    /// The microphone sent only digital silence (e.g. muted).
    Silent,
    /// Transcribing or pasting failed.
    Failed,
}

/// Emitted when a dictation finishes (not on cancel); onboarding uses it to measure speed.
#[derive(Clone, Debug, Serialize, Deserialize, Type, tauri_specta::Event)]
pub struct TranscriptionCompleteEvent {
    pub outcome: TranscriptionOutcome,
    pub audio_seconds: f64,
    pub wait_seconds: f64,
}

fn emit_transcription_complete(
    app: &AppHandle,
    outcome: TranscriptionOutcome,
    sample_count: usize,
    stop_time: Instant,
) {
    let event = TranscriptionCompleteEvent {
        outcome,
        audio_seconds: sample_count as f64
            / crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE as f64,
        wait_seconds: stop_time.elapsed().as_secs_f64(),
    };
    if let Err(e) = event.emit(app) {
        warn!("Failed to emit transcription complete: {e}");
    }
}

/// Nothing to type: `Silent` if the microphone sent only zeros, else `Empty`.
fn empty_outcome(input_silent: bool) -> TranscriptionOutcome {
    if input_silent {
        TranscriptionOutcome::Silent
    } else {
        TranscriptionOutcome::Empty
    }
}

/// Why a dictation typed nothing, when the user should be told: a microphone
/// that failed or sent only digital silence (#1899), or at least a second of
/// speech that came out empty. A short tap (padded to 1.25 s) stays quiet.
fn empty_notice(
    input_silent: bool,
    recorder_failed: bool,
    sample_count: usize,
) -> Option<NoticeCode> {
    if recorder_failed {
        Some(NoticeCode::MicrophoneLost)
    } else if input_silent {
        Some(NoticeCode::MicrophoneSilent)
    } else if sample_count > crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE as usize * 5 / 4 {
        // Over the 1.25 s a short take is padded to: real speech.
        Some(NoticeCode::NothingRecognized)
    } else {
        None
    }
}

/// The notice after a successful paste: a skipped rewrite, a microphone that
/// failed mid-take (only the start was typed), or a failed rewrite.
fn typed_notice(
    rewrite_skipped: Option<NoticeCode>,
    recorder_failed: bool,
    post_process_failed: bool,
) -> Option<NoticeCode> {
    rewrite_skipped
        .or(recorder_failed.then_some(NoticeCode::MicrophoneLost))
        .or(post_process_failed.then_some(NoticeCode::PostProcessFailed))
}

/// Ends a dictation that has nothing to type, saying why when it isn't a tap.
fn finish_without_text(
    app: &AppHandle,
    input_silent: bool,
    recorder_failed: bool,
    sample_count: usize,
    stop_time: Instant,
) {
    if let Some(code) = empty_notice(input_silent, recorder_failed, sample_count) {
        warn!("Nothing was typed: {code:?}");
        notices::raise(app, Notice::new(code, NoticeSource::Dictation));
    } else {
        utils::hide_recording_overlay(app);
    }
    set_tray_state(app, TrayIconState::Idle);
    emit_transcription_complete(app, empty_outcome(input_silent), sample_count, stop_time);
}

const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// Deadline for the whole post-processing step, retries included. The HTTP
/// client only bounds connecting, so an endpoint that accepts the request and
/// never answers would otherwise hold the dictation until a proxy gives up.
const POST_PROCESS_TIMEOUT: Duration = Duration::from_secs(60);

/// The rewrite's whole budget: the setting, or more for a long dictation
/// (7 s + 60 ms a word), so a 45 s dictation isn't pasted raw at 8 s.
fn rewrite_timeout(setting_secs: u64, transcription: &str) -> Duration {
    let words = transcription.split_whitespace().count() as u64;
    Duration::from_secs(setting_secs.max(1))
        .max(Duration::from_millis(7_000 + 60 * words))
        .min(POST_PROCESS_TIMEOUT)
}

/// Bumped per recording session, so a late paste from an earlier session
/// doesn't reset the tray or hide the overlay of a newer one.
static RECORDING_SESSION: AtomicU64 = AtomicU64::new(0);

/// Drop guard that finishes the transcription pipeline, including immediate
/// model unloading on early exits.
struct FinishGuard(AppHandle, Arc<TranscriptionManager>);
impl Drop for FinishGuard {
    fn drop(&mut self) {
        // Processing is over: disarm Escape, unless a newer recording armed it.
        let recording = self
            .0
            .try_state::<Arc<AudioRecordingManager>>()
            .is_some_and(|rm| rm.is_recording());
        if !recording {
            shortcut::unregister_cancel_shortcut(&self.0);
        }
        self.1.maybe_unload_immediately("transcription session");
        if let Some(c) = self.0.try_state::<TranscriptionCoordinator>() {
            c.notify_processing_finished();
        }
        // The pipeline just freed its large transient buffers (captured PCM,
        // WAV copy, engine scratch); hand the cached pages back to the OS so
        // they don't sit in malloc arenas until they get swapped out (#1792).
        crate::memory::trim_freed_memory();
    }
}

// Shortcut Action Trait
pub trait ShortcutAction: Send + Sync {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
}

// Transcribe Action
struct TranscribeAction {
    post_process: bool,
    /// Handy Adaptive (F123): the speech is an instruction for the selection.
    instruct: bool,
}

/// F121–F123: applies `instruction` to `text` with the rewrite provider.
async fn transform_text(
    settings: &AppSettings,
    instruction: &str,
    text: &str,
    context: Option<&crate::context::LlmContext>,
    report: Option<u64>,
) -> Option<String> {
    let (provider, key, model, reasoning) = rewrite_target(settings)?;
    let message = crate::adaptive::transform_user_message(instruction, text, context);
    crate::context::record_sent(report, &message);
    let call = crate::llm_client::send_chat_completion_with_schema(
        &provider,
        key,
        &model,
        message.clone(),
        Some(crate::adaptive::TRANSFORM_PROMPT.to_string()),
        None,
        rewrite_reasoning(&provider.id, reasoning),
    );
    let mut reply = tokio::time::timeout(Duration::from_secs(12), call)
        .await
        .unwrap_or_else(|_| Err("timed out".into()));
    if !matches!(&reply, Ok(Some(out)) if !crate::adaptive::is_refusal(text, out)) {
        warn!(
            "Transformation failed or refused: {:?}",
            reply.as_ref().err()
        );
        reply = Err("refused".into());
        let spare = ask_fallback(
            settings,
            &provider.id,
            &model,
            message,
            crate::adaptive::TRANSFORM_PROMPT,
        );
        if let Ok(Some(r)) = tokio::time::timeout(Duration::from_secs(8), spare).await {
            reply = r;
        }
    }
    match reply {
        Ok(Some(out)) if !crate::adaptive::is_refusal(text, &out) => {
            non_blank_post_processed_text(strip_invisible_chars(strip_think_block(&out)))
        }
        _ => None,
    }
}

/// System prompt for the Handy Adaptive framing, or None for the legacy
/// `${output}` substitution. A user prompt without `${output}` expects the
/// dictation as a separate `<dictation>` message, so it gets the house framing too.
fn house_framing(prompt_id: &str, prompt: &str) -> Option<String> {
    if prompt_id == crate::adaptive::HOUSE_PROMPT_ID {
        Some(crate::adaptive::HOUSE_PROMPT.to_string())
    } else if !prompt.contains("${output}") {
        Some(prompt.to_string())
    } else {
        None
    }
}

fn rewrite_target(settings: &AppSettings) -> Option<crate::features::Target> {
    if settings.features.rewrite.provider_id.is_empty() {
        return None;
    }
    feature_target(settings, Feature::Rewrite)
        .or_else(|| feature_target(settings, Feature::RewriteFallback))
}

// Preserve the default request shape for rewrite providers that never got an Off field.
fn rewrite_reasoning(provider_id: &str, reasoning: Reasoning) -> Option<Reasoning> {
    (reasoning != Reasoning::Off
        || matches!(provider_id, "custom" | "openrouter" | "cloudflare")
        || provider_id.starts_with("custom-"))
    .then_some(reasoning)
}

/// Groq ↔ Cloudflare: when the main provider fails, the fallback provider set
/// in the app answers the same request. None when there's no usable fallback.
async fn ask_fallback(
    settings: &AppSettings,
    failed_id: &str,
    failed_model: &str,
    user: String,
    system: &str,
) -> Option<Result<Option<String>, String>> {
    let (provider, key, model, reasoning) = feature_target(settings, Feature::RewriteFallback)?;
    // The same model as the main one (a common setup): one more try still
    // beats pasting raw after an empty reply, a 5xx or a dropped connection.
    if provider.id == failed_id && model == failed_model {
        warn!("Main provider '{failed_id}' failed; trying the same model once more");
    } else {
        warn!(
            "Main provider '{failed_id}' failed; asking fallback '{}'",
            provider.id
        );
    }
    Some(
        crate::llm_client::send_chat_completion_with_schema(
            &provider,
            key,
            &model,
            user,
            Some(system.to_string()),
            None,
            rewrite_reasoning(&provider.id, reasoning),
        )
        .await,
    )
}

/// The selection a transformation may replace, or the reason it can't (F121).
fn selection_allowed(capture: &crate::context::Capture) -> bool {
    !capture.sensitive && !capture.excluded && !capture.terminal
}

fn selection_notice(capture: &crate::context::Capture) -> NoticeCode {
    if capture.sensitive || capture.excluded {
        NoticeCode::PrivacyRewriteSkipped
    } else {
        NoticeCode::SelectionUnavailable
    }
}

/// Ctrl+C is the fallback for fields UI Automation can't read. When it sees
/// the caret and no selection, nothing is selected: Ctrl+C would interrupt a
/// terminal's command or copy the whole line in an editor.
fn may_copy_selection(field: Option<&crate::context::Field>) -> bool {
    field.map_or(true, |f| !f.caret_known)
}

fn transformable_selection(app: &AppHandle, capture: &crate::context::Capture) -> Option<String> {
    // Ctrl+C would interrupt a terminal's running command.
    if !selection_allowed(capture) {
        return None;
    }
    let selection = capture
        .field
        .as_ref()
        .map(|f| f.selection.clone())
        .filter(|s| !s.trim().is_empty())
        // Google Docs, Word: UI Automation can't see the selection.
        .or_else(|| {
            may_copy_selection(capture.field.as_ref())
                .then(|| crate::clipboard::copy_selection(app))
                .flatten()
        })?;
    (selection.chars().count() <= crate::context::SELECTION_MAX).then_some(selection)
}

/// F121/F122: Polish or a saved transformation of the selected text, on a key press.
struct TransformAction {
    /// `None`: Polish; `Some(i)`: saved transformation `i` (0-based).
    slot: Option<usize>,
}

impl ShortcutAction for TransformAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        let (app, slot) = (app.clone(), self.slot);
        std::thread::spawn(move || {
            let settings = get_settings(&app);
            let instruction = match slot {
                None => crate::adaptive::POLISH_INSTRUCTION.to_string(),
                Some(i) => match settings.transforms.get(i) {
                    Some(t) if !t.instruction.trim().is_empty() => t.instruction.clone(),
                    _ => return,
                },
            };
            let capture = crate::context::capture(
                true,
                &settings.context_excluded_apps,
                &settings.context_excluded_domains,
            );
            let Some(selection) = transformable_selection(&app, &capture) else {
                notices::raise(
                    &app,
                    Notice::new(selection_notice(&capture), NoticeSource::Dictation),
                );
                return;
            };
            let context = crate::context::enriched_context(
                &app,
                &settings,
                &capture,
                &selection,
                &settings.transform_context,
            );
            let report = crate::context::set_report(crate::context::ContextReport {
                kind: if slot.is_some() {
                    crate::context::ReportKind::Transform
                } else {
                    crate::context::ReportKind::Polish
                },
                capture: Some(capture),
                ..Default::default()
            });
            show_processing_overlay(&app);
            let result = tauri::async_runtime::block_on(transform_text(
                &settings,
                &instruction,
                &selection,
                context.as_ref(),
                Some(report),
            ));
            match result {
                // Pasting over the selection replaces it; the app's undo reverts it.
                Some(text) => {
                    let result = utils::paste(text, app.clone());
                    if let Err(e) = &result {
                        error!("Failed to paste the transformation: {e}");
                    }
                    match transform_paste_notice(&result) {
                        Some(code) => {
                            notices::raise(&app, Notice::new(code, NoticeSource::Dictation))
                        }
                        None => utils::hide_recording_overlay(&app),
                    }
                }
                None => notices::raise(
                    &app,
                    Notice::new(NoticeCode::PostProcessFailed, NoticeSource::Dictation),
                ),
            }
        });
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {}
}

/// The notice for a transformation's paste: an admin window rejects the keys
/// without an error, so `Blocked` is a failure too.
fn transform_paste_notice(result: &Result<utils::PasteOutcome, String>) -> Option<NoticeCode> {
    match result {
        Ok(utils::PasteOutcome::Pasted) => None,
        Ok(utils::PasteOutcome::Blocked) => Some(NoticeCode::PasteBlockedElevated),
        Err(_) => Some(NoticeCode::PasteFailed),
    }
}

/// Field name for structured output JSON schema
const TRANSCRIPTION_FIELD: &str = "transcription";

/// Strip invisible Unicode characters that some LLMs may insert
fn strip_invisible_chars(s: &str) -> String {
    s.replace(['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'], "")
}

/// Strip a leading `<think>...</think>` block. Some endpoints can't disable
/// reasoning, and some local servers put the reasoning text into `content`
/// instead of a separate field — without this the user would get the model's
/// chain of thought pasted along with the cleaned transcription.
pub(crate) fn strip_think_block(mut content: &str) -> &str {
    while let Some(thought) = content.trim_start().strip_prefix("<think>") {
        let Some((_, visible)) = thought.split_once("</think>") else {
            return "";
        };
        content = visible.trim_start();
    }
    content
}

/// Build a system prompt from the user's prompt template.
/// Removes `${output}` placeholder since the transcription is sent as the user message.
fn build_system_prompt(prompt_template: &str) -> String {
    prompt_template.replace("${output}", "").trim().to_string()
}

/// Returns `true` when a transcription has no meaningful content to
/// post-process (empty or whitespace-only). Used to skip the post-processing
/// LLM call when nothing was actually transcribed, which would otherwise make
/// the model reply with an error message such as "you need to provide the
/// transcription".
fn is_blank_transcription(transcription: &str) -> bool {
    transcription.trim().is_empty()
}

fn non_blank_post_processed_text(text: String) -> Option<String> {
    if is_blank_transcription(&text) {
        warn!(
            "Post-processing returned an empty response. Falling back to original transcription."
        );
        None
    } else {
        Some(text)
    }
}

async fn complete_unless_cancelled<F, C>(operation: F, is_cancelled: C) -> Option<F::Output>
where
    F: Future,
    C: Fn() -> bool,
{
    tokio::pin!(operation);

    loop {
        if is_cancelled() {
            return None;
        }

        if let Ok(result) =
            tokio::time::timeout(CANCELLATION_POLL_INTERVAL, operation.as_mut()).await
        {
            return Some(result);
        }
    }
}

fn discard_cancelled_wav(path: &std::path::Path) {
    if let Err(err) = std::fs::remove_file(path) {
        if err.kind() != std::io::ErrorKind::NotFound {
            error!(
                "Failed to remove cancelled recording {}: {}",
                path.display(),
                err
            );
        }
    }
}

async fn discard_cancelled_take(hm: &HistoryManager, id: Option<i64>, path: &std::path::Path) {
    if let Some(id) = id {
        if let Err(err) = hm.delete_entry(id).await {
            error!("Failed to remove cancelled history entry: {}", err);
        }
    }
    discard_cancelled_wav(path);
}

fn failed_instruction_for_history(sensitive: bool, text: &str) -> Option<String> {
    (!sensitive).then(|| text.to_string())
}

fn should_use_streaming_overlay(style: OverlayStyle, is_streaming: bool) -> bool {
    style == OverlayStyle::Live && is_streaming
}

/// What became of a transcription's post-processing.
#[derive(Debug, PartialEq, Eq)]
enum PostProcessOutcome {
    /// There was no speech to process.
    Skipped,
    Done(String),
    /// The user asked for it, but it couldn't run or produced nothing, so the
    /// transcript goes out as it is.
    Failed,
    /// The rewrite came back anomalous (§6 guard 3): the cleaned transcript
    /// goes out, without a notice for an isolated case.
    Rejected,
}

async fn post_process_transcription(
    settings: &AppSettings,
    transcription: &str,
    context: Option<&crate::context::LlmContext>,
    report: Option<u64>,
) -> PostProcessOutcome {
    if is_blank_transcription(transcription) {
        debug!("Post-processing skipped because the transcription is empty");
        return PostProcessOutcome::Skipped;
    }

    let Some((provider, api_key, model, reasoning)) = rewrite_target(settings) else {
        debug!("Post-processing enabled but no usable model is selected");
        return PostProcessOutcome::Failed;
    };

    let selected_prompt_id = match &settings.post_process_selected_prompt_id {
        Some(id) => id.clone(),
        None => {
            debug!("Post-processing skipped because no prompt is selected");
            return PostProcessOutcome::Failed;
        }
    };

    let prompt = match settings
        .post_process_prompts
        .iter()
        .find(|prompt| prompt.id == selected_prompt_id)
    {
        Some(prompt) => prompt.prompt.clone(),
        None => {
            debug!(
                "Post-processing skipped because prompt '{}' was not found",
                selected_prompt_id
            );
            return PostProcessOutcome::Failed;
        }
    };

    if prompt.trim().is_empty() {
        debug!("Post-processing skipped because the selected prompt is empty");
        return PostProcessOutcome::Failed;
    }

    debug!(
        "Starting LLM post-processing with provider '{}' (model: {})",
        provider.id, model
    );

    let reasoning = rewrite_reasoning(&provider.id, reasoning);

    // Handy Adaptive framing: instructions in the system message, the
    // dictation (and context) framed as data in the user message, plain-text reply.
    if let Some(system_prompt) = house_framing(&selected_prompt_id, &prompt)
        .filter(|_| provider.id != APPLE_INTELLIGENCE_PROVIDER_ID)
    {
        let message = crate::adaptive::house_user_message(transcription, context);
        crate::context::record_sent(report, &message);
        let primary = async {
            let mut reply = crate::llm_client::send_chat_completion_with_schema(
                &provider,
                api_key.clone(),
                &model,
                message.clone(),
                Some(system_prompt.clone()),
                None,
                reasoning,
            )
            .await;
            // Groq's free tier caps each model's tokens per day: once 120b is out
            // of quota, 20b (its own quota) still rewrites better than no rewrite.
            if model == crate::adaptive::GROQ_MAIN_MODEL
                && reply.as_ref().is_err_and(|e| e.contains("status 429"))
            {
                warn!(
                    "Rewrite model is rate-limited; retrying with {}",
                    crate::adaptive::GROQ_SPARE_MODEL
                );
                reply = crate::llm_client::send_chat_completion_with_schema(
                    &provider,
                    api_key,
                    crate::adaptive::GROQ_SPARE_MODEL,
                    message.clone(),
                    Some(system_prompt.clone()),
                    None,
                    reasoning,
                )
                .await;
            }
            reply
        };
        // Leave the fallback ~3 s of the overall post-processing timeout.
        let budget = rewrite_timeout(settings.post_process_timeout_secs, transcription)
            .saturating_sub(Duration::from_secs(3))
            .max(Duration::from_secs(2));
        let mut reply = tokio::time::timeout(budget, primary)
            .await
            .unwrap_or_else(|_| Err("timed out".into()));
        if !matches!(reply, Ok(Some(_))) {
            warn!(
                "Rewrite failed for provider '{}': {:?}",
                provider.id,
                reply.as_ref().err()
            );
            if let Some(spare) =
                ask_fallback(settings, &provider.id, &model, message, &system_prompt).await
            {
                reply = spare;
            }
        }
        return match reply {
            Ok(Some(content)) => {
                let mut text = strip_invisible_chars(strip_think_block(&content));
                // F59: the tone's sure effects; a terminal keeps its text.
                if let Some(c) = context.filter(|c| !c.terminal) {
                    text = crate::adaptive::apply_tone(&text, c.tone);
                }
                let text = crate::adaptive::finish_rewrite(&text);
                let email = context.is_some_and(|c| c.content == crate::context::Content::Email);
                PostProcessOutcome::Done(if email {
                    crate::adaptive::letter_form(&text)
                } else {
                    text
                })
            }
            Ok(None) => PostProcessOutcome::Failed,
            Err(e) => {
                warn!("Rewrite failed for provider '{}': {}", provider.id, e);
                PostProcessOutcome::Failed
            }
        };
    }

    if provider.supports_structured_output {
        debug!("Using structured outputs for provider '{}'", provider.id);

        let system_prompt = build_system_prompt(&prompt);
        let user_content = transcription.to_string();

        // Handle Apple Intelligence separately since it uses native Swift APIs
        if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                if !apple_intelligence::check_apple_intelligence_availability() {
                    debug!(
                        "Apple Intelligence selected but not currently available on this device"
                    );
                    return PostProcessOutcome::Failed;
                }

                let token_limit = model.trim().parse::<i32>().unwrap_or(0);
                return match apple_intelligence::process_text_with_system_prompt(
                    &system_prompt,
                    &user_content,
                    token_limit,
                ) {
                    Ok(result) => {
                        if result.trim().is_empty() {
                            debug!("Apple Intelligence returned an empty response");
                            PostProcessOutcome::Failed
                        } else {
                            let result = strip_invisible_chars(&result);
                            debug!(
                                "Apple Intelligence post-processing succeeded. Output length: {} chars",
                                result.len()
                            );
                            PostProcessOutcome::Done(result)
                        }
                    }
                    Err(err) => {
                        error!("Apple Intelligence post-processing failed: {}", err);
                        PostProcessOutcome::Failed
                    }
                };
            }

            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                debug!("Apple Intelligence provider selected on unsupported platform");
                return PostProcessOutcome::Failed;
            }
        }

        // Define JSON schema for transcription output
        let json_schema = serde_json::json!({
            "type": "object",
            "properties": {
                (TRANSCRIPTION_FIELD): {
                    "type": "string",
                    "description": "The cleaned and processed transcription text"
                }
            },
            "required": [TRANSCRIPTION_FIELD],
            "additionalProperties": false
        });

        match crate::llm_client::send_chat_completion_with_schema(
            &provider,
            api_key.clone(),
            &model,
            user_content,
            Some(system_prompt),
            Some(json_schema),
            reasoning,
        )
        .await
        {
            Ok(Some(content)) => {
                // Parse the JSON response to extract the transcription field
                let content = strip_think_block(&content);
                match serde_json::from_str::<serde_json::Value>(content) {
                    Ok(json) => {
                        if let Some(transcription_value) =
                            json.get(TRANSCRIPTION_FIELD).and_then(|t| t.as_str())
                        {
                            let result = strip_invisible_chars(transcription_value);
                            debug!(
                                "Structured output post-processing succeeded for provider '{}'. Output length: {} chars",
                                provider.id,
                                result.len()
                            );
                            return PostProcessOutcome::Done(result);
                        } else {
                            error!("Structured output response missing 'transcription' field");
                            return PostProcessOutcome::Done(strip_invisible_chars(content));
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse structured output JSON: {}. Returning raw content.",
                            e
                        );
                        return PostProcessOutcome::Done(strip_invisible_chars(content));
                    }
                }
            }
            Ok(None) => {
                error!("LLM API response has no content");
                return PostProcessOutcome::Failed;
            }
            Err(e) => {
                warn!(
                    "Structured output failed for provider '{}': {}. Falling back to legacy mode.",
                    provider.id, e
                );
                // Fall through to legacy mode below
            }
        }
    }

    // Legacy mode: Replace ${output} variable in the prompt with the actual text
    let processed_prompt = prompt.replace("${output}", transcription);
    debug!("Processed prompt length: {} chars", processed_prompt.len());

    match crate::llm_client::send_chat_completion(
        &provider,
        api_key,
        &model,
        processed_prompt,
        reasoning,
    )
    .await
    {
        Ok(Some(content)) => {
            let content = strip_invisible_chars(strip_think_block(&content));
            debug!(
                "LLM post-processing succeeded for provider '{}'. Output length: {} chars",
                provider.id,
                content.len()
            );
            PostProcessOutcome::Done(content)
        }
        Ok(None) => {
            error!("LLM API response has no content");
            PostProcessOutcome::Failed
        }
        Err(e) => {
            error!(
                "LLM post-processing failed for provider '{}': {}. Falling back to original transcription.",
                provider.id,
                e
            );
            PostProcessOutcome::Failed
        }
    }
}

/// Spec §6 rule 5: the learned names are applied locally only when no
/// rewrite came out, for a plain dictation, never in a private field.
/// R1: a password, excluded or unchecked field is never watched, so none of
/// its text reaches the learning validator or the judge.
fn observable(sensitive: bool, excluded: bool, unchecked: bool) -> bool {
    !sensitive && !excluded && !unchecked
}

/// Only a known password field or an excluded app/site stops the rewrite:
/// a check that couldn't be made (unknown site, unanswered password check)
/// must not leave the dictation raw.
fn rewrite_allowed(requested: bool, sensitive: bool, excluded: bool, snippet: bool) -> bool {
    requested && !sensitive && !excluded && !snippet
}

fn rewrite_skip_notice(requested: bool, excluded: bool, snippet: bool) -> Option<NoticeCode> {
    (requested && !snippet && excluded).then_some(NoticeCode::PrivacyRewriteSkipped)
}

fn learned_fallback(rewrote: bool, instruct: bool, snippet: bool, private: bool) -> bool {
    !rewrote && !instruct && !snippet && !private
}

pub(crate) struct ProcessedTranscription {
    pub final_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
    /// `final_text` is the bare transcript because post-processing failed.
    pub post_process_failed: bool,
}

pub(crate) async fn process_transcription_output(
    app: &AppHandle,
    transcription: &str,
    post_process: bool,
    context: Option<&crate::context::LlmContext>,
    report: Option<u64>,
) -> ProcessedTranscription {
    let settings = get_settings(app);
    let mut final_text = transcription.to_string();
    let mut post_processed_text: Option<String> = None;
    let mut post_process_prompt: Option<String> = None;
    let mut post_process_failed = false;

    if post_process {
        // A reply that's blank once cleaned is no better than none.
        let timeout = rewrite_timeout(settings.post_process_timeout_secs, &final_text);
        let outcome = match tokio::time::timeout(
            timeout,
            post_process_transcription(&settings, &final_text, context, report),
        )
        .await
        {
            Ok(PostProcessOutcome::Done(text)) => {
                non_blank_post_processed_text(text).map_or(PostProcessOutcome::Failed, |text| {
                    // §6 guard 3: a verifiably anomalous rewrite falls back.
                    // House rewrite only: a user prompt may translate or summarize.
                    let house = settings.post_process_selected_prompt_id.as_deref()
                        == Some(crate::adaptive::HOUSE_PROMPT_ID);
                    match house
                        .then(|| crate::adaptive::output_anomaly(&final_text, &text))
                        .flatten()
                    {
                        Some(anomaly) => {
                            warn!("Rewrite rejected ({anomaly:?}); using the cleaned transcript");
                            PostProcessOutcome::Rejected
                        }
                        None => PostProcessOutcome::Done(text),
                    }
                })
            }
            Ok(outcome) => outcome,
            Err(_) => {
                warn!(
                    "Post-processing timed out after {:?}; pasting the original transcription",
                    timeout
                );
                PostProcessOutcome::Failed
            }
        };
        match outcome {
            PostProcessOutcome::Skipped | PostProcessOutcome::Rejected => {}
            PostProcessOutcome::Failed => post_process_failed = true,
            PostProcessOutcome::Done(processed_text) => {
                post_processed_text = Some(processed_text.clone());
                final_text = processed_text;

                if let Some(prompt_id) = &settings.post_process_selected_prompt_id {
                    if let Some(prompt) = settings
                        .post_process_prompts
                        .iter()
                        .find(|prompt| &prompt.id == prompt_id)
                    {
                        post_process_prompt = Some(prompt.prompt.clone());
                    }
                }
            }
        }
    }

    ProcessedTranscription {
        final_text,
        post_processed_text,
        post_process_prompt,
        post_process_failed,
    }
}

impl ShortcutAction for TranscribeAction {
    fn start(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        let start_time = Instant::now();
        debug!("TranscribeAction::start called for binding: {}", binding_id);

        // Load model in the background
        let tm = app.state::<Arc<TranscriptionManager>>();
        let rm = app.state::<Arc<AudioRecordingManager>>();

        // Load ASR model and VAD model in parallel
        let kickoff_started = Instant::now();
        // An empty selection has nothing to load; the check below says so.
        let selected_model = get_settings(app).selected_model;
        let load_started = !selected_model.is_empty() && tm.initiate_model_load(true);
        let rm_clone = Arc::clone(&rm);
        std::thread::spawn(move || {
            if let Err(e) = rm_clone.preload_vad() {
                debug!("VAD pre-load failed: {}", e);
            }
        });
        let kickoff_elapsed = kickoff_started.elapsed();

        // Don't open the mic if nothing can transcribe the recording. A load
        // started above reports its own failure; one already running doesn't.
        if !tm.is_model_loaded() {
            if selected_model.is_empty() {
                warn!("Not starting recording: no model is selected");
                tm.raise_no_model_selected();
                return;
            }
            if let Err(e) = app
                .state::<Arc<ModelManager>>()
                .get_model_path(&selected_model)
            {
                warn!("Not starting recording: no model can transcribe it ({})", e);
                if !load_started {
                    tm.raise_model_load_failed(&selected_model, &e);
                }
                return;
            }
        }

        // F98: a new dictation closes the previous observation, which learns
        // before this dictation's rewrite.
        crate::learning::end_observation();
        let binding_id = binding_id.to_string();
        RECORDING_SESSION.fetch_add(1, Ordering::SeqCst);
        let tray_started = Instant::now();
        set_tray_state(app, TrayIconState::Recording);
        let tray_elapsed = tray_started.elapsed();

        // Get the microphone mode to determine audio feedback timing
        let plan_started = Instant::now();
        let settings = get_settings(app);
        let is_always_on = settings.always_on_microphone;

        let selected_model_info = app
            .state::<Arc<ModelManager>>()
            .get_model_info(&settings.selected_model);

        // Use the app-facing model capability as the single pre-recording source
        // for live streaming decisions. Unknown support is represented as false
        // until the model registry is updated by discovery or runtime load.
        let model_supports_streaming = selected_model_info
            .as_ref()
            .map(|m| m.supports_streaming)
            .unwrap_or(false);
        let vad_policy = if !settings.vad_enabled {
            VadPolicy::Disabled
        } else if model_supports_streaming {
            VadPolicy::Streaming
        } else {
            VadPolicy::Offline
        };
        // Before the model loads, whether it keeps its accuracy on long audio
        // is unknown, so assume not: the worker decides again with the loaded
        // model, and a wrong guess costs only an idle worker.
        let expected_model = ModelTraits {
            streaming: model_supports_streaming,
            long_form: false,
        };
        if live_decode(expected_model, settings.long_form_chunking) != LiveDecode::None {
            tm.start_stream();
        }
        let plan_elapsed = plan_started.elapsed();

        // Sizing the overlay follows the same advertised capability. A model that
        // doesn't stream (or whose capability is not known yet) gets the compact
        // pill instead of an oversized transparent live window.
        let overlay_started = Instant::now();
        match settings.overlay_style {
            OverlayStyle::Live if model_supports_streaming => utils::show_streaming_overlay(app),
            OverlayStyle::Live | OverlayStyle::Minimal => show_recording_overlay(app),
            OverlayStyle::None => {} // show_overlay_state no-ops on None anyway
        }
        // Everything above runs before capture can begin, so each span here is
        // added keypress->capture latency.
        debug!(
            "start-path pre-recording steps: model_kickoff={:?} tray={:?} settings+stream_plan={:?} overlay={:?}",
            kickoff_elapsed,
            tray_elapsed,
            plan_elapsed,
            overlay_started.elapsed()
        );
        debug!("Microphone mode - always_on: {}", is_always_on);

        let mut recording_error: Option<anyhow::Error> = None;
        let recording_start_time = Instant::now();
        match rm.try_start_recording(&binding_id, vad_policy) {
            Ok(readiness) => {
                debug!(
                    "Recording request accepted in {:?}; waiting for first microphone samples",
                    recording_start_time.elapsed()
                );
                let generation = readiness.generation();
                let app_clone = app.clone();
                let rm_clone = Arc::clone(&rm);
                std::thread::spawn(move || {
                    if !readiness.wait() {
                        debug!("Microphone readiness wait ended without receiving samples");
                        return;
                    }

                    // Development-only preview hook for evaluating the brief
                    // arming animation on hardware that normally starts too fast
                    // to make it visible.
                    #[cfg(debug_assertions)]
                    if let Ok(delay_ms) = std::env::var("HANDY_DEBUG_MIC_READY_DELAY_MS")
                        .unwrap_or_default()
                        .parse::<u64>()
                    {
                        let delay_ms = delay_ms.min(10_000);
                        if delay_ms > 0 {
                            debug!("Delaying microphone-ready cue by {delay_ms}ms for UI preview");
                            std::thread::sleep(Duration::from_millis(delay_ms));
                        }
                    }

                    if !rm_clone.is_recording_readiness_current(generation) {
                        debug!("Microphone became ready for an inactive recording");
                        return;
                    }

                    debug!("Microphone is receiving samples; recording is ready");
                    utils::emit_recording_ready(&app_clone);

                    // The start chime is a readiness cue, so it must follow the
                    // first real input callback rather than Stream::play() or a
                    // fixed delay. The helper returns immediately when feedback
                    // is disabled; mute still follows the same readiness point.
                    if rm_clone.is_recording_readiness_current(generation) {
                        play_feedback_sound_blocking(&app_clone, SoundType::Start);
                    }
                    if rm_clone.is_recording_readiness_current(generation) {
                        rm_clone.apply_mute();
                    }
                });
            }
            Err(e) => {
                debug!("Failed to start recording: {}", e);
                recording_error = Some(e);
            }
        }

        if recording_error.is_none() {
            // Dynamically register the cancel shortcut in a separate task to avoid deadlock
            shortcut::register_cancel_shortcut(app);
        } else {
            // Starting failed (for example due to blocked microphone permissions).
            // Revert UI state so we don't stay stuck in the recording overlay.
            tm.cancel_stream();
            set_tray_state(app, TrayIconState::Idle);
            if let Some(err) = recording_error {
                notices::raise(
                    app,
                    Notice::new(recording_notice_code(&err), NoticeSource::Dictation)
                        .with_detail(format!("{err:#}")),
                );
            }
        }

        debug!(
            "TranscribeAction::start completed in {:?}",
            start_time.elapsed()
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        // Prevent a slow microphone from emitting a ready event or start chime
        // after the user has already requested stop.
        app.state::<Arc<AudioRecordingManager>>()
            .invalidate_recording_readiness();

        // Handy Adaptive (F03/S48): Escape stays armed while the dictation is
        // processed; FinishGuard disarms it once the pipeline is done.

        let stop_time = Instant::now();
        debug!("TranscribeAction::stop called for binding: {}", binding_id);

        // Handy Adaptive (D3, F30): the focused field at release is the
        // destination. Read it now, alongside the transcription. The WAV save
        // waits only for the password check (F79: nothing kept).
        let (sensitive_tx, sensitive_rx) = std::sync::mpsc::channel::<bool>();
        // Handy Adaptive (spec §7): the names around the cursor, for an ASR
        // model that takes context. Never from a password, an excluded field
        // or an unchecked one.
        let (screen_tx, screen_rx) =
            std::sync::mpsc::channel::<Option<crate::asr_context::AsrScreen>>();
        // The speech API never hears a password or an excluded app's field.
        let (private_tx, private_rx) = std::sync::mpsc::channel::<bool>();
        let ctx_settings = get_settings(app);
        let capture_task = tauri::async_runtime::spawn_blocking(move || {
            let capture = crate::context::capture(
                ctx_settings.context_enabled,
                &ctx_settings.context_excluded_apps,
                &ctx_settings.context_excluded_domains,
            );
            let _ = sensitive_tx.send(capture.sensitive);
            // Windows: a password check that didn't happen counts as private.
            let unchecked = capture.unchecked();
            let _ = private_tx.send(capture.sensitive || capture.excluded || unchecked);
            let screen = (!capture.sensitive && !capture.excluded && !unchecked).then(|| {
                crate::asr_context::AsrScreen {
                    // Spec R10 / Wispr E140: the window's names bias the ASR.
                    terms: capture
                        .names
                        .iter()
                        .chain(&capture.terms)
                        .cloned()
                        .collect(),
                }
            });
            let _ = screen_tx.send(screen);
            capture
        });

        let ah = app.clone();
        let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
        let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());
        let hm = Arc::clone(&app.state::<Arc<HistoryManager>>());

        set_tray_state(app, TrayIconState::Transcribing);
        // Stop should give immediate visual feedback. Live streaming can keep
        // the larger panel, but it still switches from listening to a working
        // spinner while the stream finalizes. Non-streaming paths use the
        // compact transcribing pill (None no-ops in show_*).
        let style = get_settings(app).overlay_style;
        // Capture this before finalizing the stream so every later working state
        // targets the same overlay that was shown for this transcription.
        let use_streaming_overlay = should_use_streaming_overlay(style, tm.is_streaming());
        if use_streaming_overlay {
            tm.emit_stream_working(StreamWorkKind::Transcribing);
        } else {
            show_transcribing_overlay(app);
        }

        // Unmute before playing audio feedback so the stop sound is audible
        rm.remove_mute();

        // Play audio feedback for recording stop
        play_feedback_sound(app, SoundType::Stop);

        let binding_id = binding_id.to_string(); // Clone binding_id for the async task
        let post_process = self.post_process;
        let instruct = self.instruct;
        let cancel_generation = rm.cancel_generation();

        tauri::async_runtime::spawn(async move {
            let _guard = FinishGuard(ah.clone(), Arc::clone(&tm));
            debug!(
                "Starting async transcription task for binding: {}",
                binding_id
            );

            let stop_recording_time = Instant::now();
            if let Some(samples) = rm.stop_recording(&binding_id, cancel_generation) {
                // Read now: by the time transcribing ends, another take may
                // have replaced it.
                let input_silent = rm.last_input_silent();
                let recorder_failed = rm.last_recorder_failed();
                debug!(
                    "Recording stopped and samples retrieved in {:?}, sample count: {}",
                    stop_recording_time.elapsed(),
                    samples.len()
                );

                if rm.was_cancelled_since(cancel_generation) {
                    debug!("Transcription operation cancelled after recording stop");
                    tm.cancel_stream();
                    utils::hide_recording_overlay(&ah);
                    set_tray_state(&ah, TrayIconState::Idle);
                    return;
                }

                if samples.is_empty() {
                    debug!("Recording produced no audio samples; skipping persistence");
                    // Tear down any streaming worker so its channel doesn't leak
                    // and block the next start_stream.
                    tm.cancel_stream();
                    finish_without_text(&ah, input_silent, recorder_failed, 0, stop_time);
                } else {
                    // Save WAV concurrently with transcription
                    let sample_count = samples.len();
                    let file_name = format!(
                        "handy-{}-{}.wav",
                        chrono::Utc::now().timestamp_micros(),
                        RECORDING_SESSION.load(Ordering::SeqCst)
                    );
                    let wav_path = hm.recordings_dir().join(&file_name);
                    let wav_path_for_verify = wav_path.clone();
                    let samples_for_wav = samples.clone();
                    let wav_handle = tauri::async_runtime::spawn_blocking(move || {
                        // A capture always answers within its deadline.
                        if sensitive_rx
                            .recv_timeout(Duration::from_secs(3))
                            .unwrap_or(false)
                        {
                            return Ok(false);
                        }
                        crate::audio_toolkit::save_wav_file(&wav_path, &samples_for_wav)
                            .map(|()| true)
                    });

                    // Transcribe concurrently with WAV save: the live decode's
                    // result when it has one (all audio was already fed to it),
                    // otherwise a batch run of the samples. A cancel is surfaced
                    // as an error, so it never starts a batch run.
                    // Spec §7: only a model that takes context, with the screen
                    // option on, waits for the capture, and at most capture_wait_ms.
                    let asr = get_settings(&ah).asr_context;
                    // Handy Adaptive: speech by API (Groq Whisper…), local as fallback.
                    let mut cloud = crate::cloud_asr::configured(&get_settings(&ah));
                    // Unknown in time counts as private: the local model transcribes.
                    if cloud.is_some()
                        && private_rx
                            .recv_timeout(Duration::from_millis(1500))
                            .unwrap_or(true)
                    {
                        log::info!("Speech API skipped: password, excluded or unread field");
                        cloud = None;
                    }
                    let mut screen_waited = None;
                    // Kept for the local model when the speech API fails.
                    let mut screen_kept = None;
                    let wants_context = match &cloud {
                        Some(_) => asr.enabled,
                        None => tm.asr_context_wanted(),
                    };
                    if asr.screen && get_settings(&ah).learning_enabled && wants_context {
                        let wait = Duration::from_millis(asr.capture_wait_ms);
                        let screen = screen_rx.recv_timeout(wait).ok();
                        screen_waited = Some(screen.is_some());
                        screen_kept = screen.flatten();
                        tm.set_asr_screen(screen_kept.clone());
                    }
                    let transcription_time = Instant::now();
                    let mut cloud_report = None;
                    let mut cloud_text = None;
                    let mut cloud_cancelled = false;
                    if let Some(c) = cloud.as_ref().filter(|_| !samples.is_empty()) {
                        let language = crate::cloud_asr::language(&get_settings(&ah));
                        let label = format!("{} {}", c.provider.id, c.model);
                        let (prompt, mut report) = tm.cloud_context(&label, language.clone());
                        // Escape stops the call at once, without a local run.
                        match complete_unless_cancelled(
                            crate::cloud_asr::transcribe(
                                c,
                                &samples,
                                language.as_deref(),
                                prompt.as_deref(),
                                get_settings(&ah).translate_to_english,
                            ),
                            || rm.was_cancelled_since(cancel_generation),
                        )
                        .await
                        {
                            None => cloud_cancelled = true,
                            Some(Ok(raw)) => {
                                log::info!(
                                    "Speech API '{label}' transcribed {:.1}s of audio in {:?}",
                                    samples.len() as f64 / 16_000.0,
                                    transcription_time.elapsed()
                                );
                                let (t, echo) =
                                    tm.finish_cloud_text(raw, language, prompt.as_deref());
                                report.echo_dropped = echo;
                                if t.text.trim().is_empty() {
                                    // Speech may still be there: the local model decides.
                                    warn!("Speech API '{label}' returned no text; using the local model");
                                    report.cloud_error = Some("empty result".into());
                                    tm.set_asr_screen(screen_kept.clone());
                                } else {
                                    // The live decode, if any, is no longer needed.
                                    tm.cancel_stream();
                                    cloud_text = Some(t);
                                }
                            }
                            Some(Err(e)) => {
                                warn!("Speech API '{label}' failed ({e}); using the local model");
                                tm.set_asr_screen(screen_kept.clone());
                                report.cloud_error = Some(e);
                            }
                        }
                        cloud_report = Some(report);
                    }
                    let transcription_result = match cloud_text {
                        Some(t) => Ok(t),
                        None if cloud_cancelled => {
                            tm.cancel_stream();
                            Err(anyhow::anyhow!("cancelled during the speech API call"))
                        }
                        None => tm.transcribe_recording(samples),
                    };
                    // This dictation's screen never reaches the next one.
                    tm.set_asr_screen(None);
                    // F114: what the speech model was given, for Settings → Context.
                    let asr_report = match cloud_report {
                        // A failed API call keeps its error; the local run's
                        // report says what the local model got.
                        Some(r) if r.cloud_error.is_some() => tm
                            .take_asr_report()
                            .map(|mut l| {
                                l.cloud_error = r.cloud_error.clone();
                                l
                            })
                            .or(Some(r)),
                        Some(r) => Some(r),
                        None => tm.take_asr_report(),
                    }
                    .map(|mut r| {
                        r.screen_waited = screen_waited;
                        r
                    });

                    // Await WAV save and verify
                    let wav_saved = match wav_handle.await {
                        Ok(Ok(false)) => false,
                        Ok(Ok(true)) => {
                            match crate::audio_toolkit::verify_wav_file(
                                &wav_path_for_verify,
                                sample_count,
                            ) {
                                Ok(()) => true,
                                Err(e) => {
                                    error!("WAV verification failed: {}", e);
                                    false
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            error!("Failed to save WAV file: {}", e);
                            false
                        }
                        Err(e) => {
                            error!("WAV save task panicked: {}", e);
                            false
                        }
                    };

                    if rm.was_cancelled_since(cancel_generation) {
                        debug!("Transcription operation cancelled before output handling");
                        discard_cancelled_wav(&wav_path_for_verify);
                        utils::hide_recording_overlay(&ah);
                        set_tray_state(&ah, TrayIconState::Idle);
                        return;
                    }

                    match transcription_result {
                        Ok(Transcription {
                            text: transcription,
                            language,
                        }) => {
                            debug!(
                                "Transcription completed in {:?}: '{}'",
                                transcription_time.elapsed(),
                                utils::redact_text(&transcription)
                            );

                            let capture = capture_task.await.unwrap_or_default();
                            if rm.was_cancelled_since(cancel_generation) {
                                discard_cancelled_wav(&wav_path_for_verify);
                                utils::hide_recording_overlay(&ah);
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            }
                            // F79: a password field gets the raw transcript, nothing sent or kept.
                            let sensitive = capture.sensitive;
                            let excluded = capture.excluded;
                            // Windows: a password check that didn't happen counts as private.
                            let unchecked = capture.unchecked();
                            let (field, terminal) = (capture.field.clone(), capture.terminal);
                            let (screen_names, content) = (capture.names.clone(), capture.content);
                            let ctx_settings = get_settings(&ah);
                            // F120: a trigger said on its own inserts its snippet as is.
                            let snippet = (!sensitive)
                                .then(|| {
                                    crate::adaptive::snippet_for(
                                        &transcription,
                                        &ctx_settings.snippets,
                                    )
                                })
                                .flatten();
                            let rewrite_skipped =
                                rewrite_skip_notice(post_process, excluded, snippet.is_some());
                            let post_process = rewrite_allowed(
                                post_process,
                                sensitive,
                                excluded,
                                snippet.is_some(),
                            );
                            // Handy Adaptive: the rewrite's context, as the user allows it.
                            let llm_context = crate::context::enriched_context(
                                &ah,
                                &ctx_settings,
                                &capture,
                                &transcription,
                                &ctx_settings.rewrite_context,
                            );
                            // F109: where the dictation went, for History.
                            let app_label = crate::context::app_label(&capture);
                            // F123: a spoken instruction gets the selection's context.
                            let transform_context = instruct
                                .then(|| {
                                    let selected = capture
                                        .field
                                        .as_ref()
                                        .map(|f| f.selection.clone())
                                        .unwrap_or_default();
                                    crate::context::enriched_context(
                                        &ah,
                                        &ctx_settings,
                                        &capture,
                                        &selected,
                                        &ctx_settings.transform_context,
                                    )
                                })
                                .flatten();
                            let report_id =
                                crate::context::set_report(crate::context::ContextReport {
                                    kind: if instruct {
                                        crate::context::ReportKind::Instruction
                                    } else {
                                        crate::context::ReportKind::Dictation
                                    },
                                    capture: Some(capture.clone()),
                                    asr: asr_report,
                                    ..Default::default()
                                });

                            if post_process {
                                // The pill names the register it writes in
                                // (kind of app and tone, never any text).
                                if let Some(c) = llm_context.as_ref() {
                                    let _ = ah.emit_to(
                                        "recording_overlay",
                                        "rewrite-register",
                                        (c.content, c.tone),
                                    );
                                }
                                if use_streaming_overlay {
                                    tm.emit_stream_working(StreamWorkKind::Polishing);
                                } else {
                                    show_processing_overlay(&ah);
                                }
                            }
                            // F123: a spoken instruction transforms the selection; nothing
                            // else is pasted, and the result is not joined or observed.
                            let (field, instruct_result) = if instruct {
                                let Some(selection) = transformable_selection(&ah, &capture) else {
                                    if rm.was_cancelled_since(cancel_generation) {
                                        discard_cancelled_wav(&wav_path_for_verify);
                                        utils::hide_recording_overlay(&ah);
                                        set_tray_state(&ah, TrayIconState::Idle);
                                        return;
                                    }
                                    if let Some(text) =
                                        failed_instruction_for_history(sensitive, &transcription)
                                    {
                                        if let Err(err) = hm.save_entry(
                                            file_name,
                                            text,
                                            post_process,
                                            None,
                                            None,
                                            language,
                                            excluded || unchecked,
                                        ) {
                                            error!(
                                                "Failed to save instruction to history: {}",
                                                err
                                            );
                                        }
                                    }
                                    notices::raise(
                                        &ah,
                                        Notice::new(
                                            selection_notice(&capture),
                                            NoticeSource::Dictation,
                                        ),
                                    );
                                    set_tray_state(&ah, TrayIconState::Idle);
                                    return;
                                };
                                show_processing_overlay(&ah);
                                // Escape stops the wait at once, so the next
                                // dictation isn't queued behind a slow model.
                                (
                                    None,
                                    complete_unless_cancelled(
                                        transform_text(
                                            &ctx_settings,
                                            &transcription,
                                            &selection,
                                            transform_context.as_ref(),
                                            Some(report_id),
                                        ),
                                        || rm.was_cancelled_since(cancel_generation),
                                    )
                                    .await
                                    .flatten(),
                                )
                            } else {
                                (field, None)
                            };
                            if rm.was_cancelled_since(cancel_generation) {
                                discard_cancelled_wav(&wav_path_for_verify);
                                utils::hide_recording_overlay(&ah);
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            }
                            if instruct && instruct_result.is_none() {
                                if let Some(text) =
                                    failed_instruction_for_history(sensitive, &transcription)
                                {
                                    if let Err(err) = hm.save_entry(
                                        file_name,
                                        text,
                                        post_process,
                                        None,
                                        None,
                                        language,
                                        excluded || unchecked,
                                    ) {
                                        error!("Failed to save instruction to history: {}", err);
                                    }
                                }
                                notices::raise(
                                    &ah,
                                    Notice::new(
                                        NoticeCode::PostProcessFailed,
                                        NoticeSource::Dictation,
                                    ),
                                );
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            }
                            let post_process = post_process && !instruct;
                            let Some(processed) = complete_unless_cancelled(
                                process_transcription_output(
                                    &ah,
                                    instruct_result
                                        .as_deref()
                                        .or(snippet.as_deref())
                                        .unwrap_or(&transcription),
                                    post_process,
                                    llm_context.as_ref(),
                                    Some(report_id),
                                ),
                                || rm.was_cancelled_since(cancel_generation),
                            )
                            .await
                            else {
                                debug!("Transcription operation cancelled during output handling");
                                discard_cancelled_wav(&wav_path_for_verify);
                                utils::hide_recording_overlay(&ah);
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            };

                            if rm.was_cancelled_since(cancel_generation) {
                                debug!("Transcription operation cancelled before paste");
                                discard_cancelled_wav(&wav_path_for_verify);
                                utils::hide_recording_overlay(&ah);
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            }

                            // Spec §6 rule 5: no rewrite came out (not asked, failed,
                            // skipped, rejected, timed out), so the confirmed learned
                            // names are applied locally. A plain dictation only, never
                            // a private field.
                            let processed = if learned_fallback(
                                processed.post_processed_text.is_some(),
                                instruct,
                                snippet.is_some(),
                                sensitive || excluded || unchecked,
                            ) {
                                ProcessedTranscription {
                                    final_text: crate::learning::apply_learned(
                                        &ah,
                                        &processed.final_text,
                                    ),
                                    ..processed
                                }
                            } else {
                                processed
                            };

                            // Handy Adaptive: the observation records what the user kept.
                            let observed_app = app_label.clone();
                            // What the user said, before any rewrite: it keeps the
                            // learned words they used alive (A3).
                            let spoken = transcription.clone();
                            let mut history_id = None;
                            // Save to history if WAV was saved
                            if wav_saved && !sensitive {
                                match hm.save_entry(
                                    file_name,
                                    transcription,
                                    post_process,
                                    processed.post_processed_text.clone(),
                                    processed.post_process_prompt.clone(),
                                    language,
                                    excluded || unchecked,
                                ) {
                                    Ok(entry) if !app_label.is_empty() => {
                                        history_id = Some(entry.id);
                                        if let Err(err) = hm.set_entry_app(entry, app_label) {
                                            error!("Failed to record the dictation's app: {}", err);
                                        }
                                    }
                                    Ok(entry) => history_id = Some(entry.id),
                                    Err(err) => error!("Failed to save history entry: {}", err),
                                }
                                // The spellings the rewrite was given, to check later.
                                if let (Some(id), Some(c), true) =
                                    (history_id, llm_context.as_ref(), post_process)
                                {
                                    let terms: Vec<String> =
                                        c.terms.iter().chain(&c.names).cloned().collect();
                                    if let Err(err) = hm.set_context_terms(id, &terms) {
                                        error!("Failed to record the rewrite's terms: {}", err);
                                    }
                                }
                            }

                            // The user's hook only changes what gets pasted;
                            // history above keeps the text as produced.
                            let Some(hooked) = complete_unless_cancelled(
                                crate::transcription_hook::apply(&ah, processed.final_text),
                                || rm.was_cancelled_since(cancel_generation),
                            )
                            .await
                            else {
                                debug!("Transcription operation cancelled during hook");
                                discard_cancelled_take(&hm, history_id, &wav_path_for_verify).await;
                                utils::hide_recording_overlay(&ah);
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            };
                            if rm.was_cancelled_since(cancel_generation) {
                                discard_cancelled_take(&hm, history_id, &wav_path_for_verify).await;
                                utils::hide_recording_overlay(&ah);
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            }
                            // EX-1: fit the text to the cursor it goes to; a private
                            // field gets it as produced.
                            let known_names: Vec<String> = screen_names
                                .into_iter()
                                .chain(crate::learning::plain_terms(&ah))
                                .chain(ctx_settings.custom_words.iter().cloned())
                                .chain(Some(ctx_settings.user_name.trim().to_string()))
                                .filter(|n| !n.is_empty())
                                .collect();
                            let joined = crate::junction::join(
                                &hooked,
                                &crate::junction::JoinContext {
                                    field: field
                                        .as_ref()
                                        .filter(|_| !(sensitive || excluded || unchecked)),
                                    style: content
                                        .map(|c| ctx_settings.styles.tone(c.style()))
                                        .unwrap_or_default(),
                                    content,
                                    known_names: &known_names,
                                },
                            );
                            let (final_text, spacing_decided) =
                                (joined.text, joined.spacing_decided);

                            if final_text.is_empty() {
                                finish_without_text(
                                    &ah,
                                    input_silent,
                                    recorder_failed,
                                    sample_count,
                                    stop_time,
                                );
                            } else {
                                let ah_clone = ah.clone();
                                let paste_time = Instant::now();
                                let post_process_failed = processed.post_process_failed;
                                let rm_for_paste = Arc::clone(&rm);
                                let hm_for_paste = Arc::clone(&hm);
                                let wav_path_for_paste = wav_path_for_verify.clone();
                                // F80: the pasted text is watched for the user's corrections.
                                let observed = field
                                    .clone()
                                    .filter(|_| observable(sensitive, excluded, unchecked))
                                    .map(|f| (f, final_text.clone(), history_id, observed_app));
                                // What was pasted counts the learned words it holds.
                                let kept = final_text.clone();
                                // Taken before the paste thread exists, and so
                                // before the next session can start.
                                let session = RECORDING_SESSION.load(Ordering::SeqCst);
                                // Off the main thread: paste sleeps between steps, and a
                                // paste into Handy's own windows needs the main thread free.
                                std::thread::Builder::new()
                                    .name("paste".into())
                                    .spawn(move || {
                                        if rm_for_paste.was_cancelled_since(cancel_generation) {
                                            debug!(
                                                "Transcription operation cancelled before paste"
                                            );
                                            tauri::async_runtime::block_on(discard_cancelled_take(
                                                &hm_for_paste,
                                                history_id,
                                                &wav_path_for_paste,
                                            ));
                                            utils::hide_recording_overlay(&ah_clone);
                                            set_tray_state(&ah_clone, TrayIconState::Idle);
                                            return;
                                        }

                                        // A new session may have taken over the
                                        // UI while this paste ran.
                                        let owns_ui =
                                            || RECORDING_SESSION.load(Ordering::SeqCst) == session;
                                        // F33: no automatic paste into an excluded target;
                                        // the text stays in History.
                                        let pasted = if excluded {
                                            Err(None)
                                        } else {
                                            if sensitive {
                                                utils::paste_secret(final_text, ah_clone.clone())
                                            } else if spacing_decided {
                                                utils::paste_spaced(final_text, ah_clone.clone())
                                            } else {
                                                utils::paste(final_text, ah_clone.clone())
                                            }
                                            .map_err(Some)
                                        };
                                        if rm_for_paste.was_cancelled_since(cancel_generation) {
                                            tauri::async_runtime::block_on(discard_cancelled_take(
                                                &hm_for_paste,
                                                history_id,
                                                &wav_path_for_paste,
                                            ));
                                            utils::hide_recording_overlay(&ah_clone);
                                            set_tray_state(&ah_clone, TrayIconState::Idle);
                                            return;
                                        }
                                        let (outcome, notice) = match pasted {
                                            Err(None) => (
                                                TranscriptionOutcome::Failed,
                                                Some(Notice::new(
                                                    NoticeCode::PasteSkippedExcluded,
                                                    NoticeSource::Dictation,
                                                )),
                                            ),
                                            Ok(utils::PasteOutcome::Pasted) => {
                                                debug!(
                                                    "Text pasted successfully in {:?}",
                                                    paste_time.elapsed()
                                                );
                                                crate::learning::touch_used(
                                                    &ah_clone, &spoken, &kept,
                                                );
                                                if let Some((f, text, id, app)) = &observed {
                                                    crate::learning::observe(
                                                        &ah_clone,
                                                        f,
                                                        text,
                                                        *id,
                                                        app.clone(),
                                                        terminal,
                                                    );
                                                }
                                                // Leave a newer session's notice alone.
                                                if owns_ui() {
                                                    notices::clear(
                                                        &ah_clone,
                                                        NoticeSource::Dictation,
                                                    );
                                                }
                                                let notice = typed_notice(
                                                    rewrite_skipped,
                                                    recorder_failed,
                                                    post_process_failed,
                                                )
                                                .map(|code| {
                                                    Notice::new(code, NoticeSource::Dictation)
                                                });
                                                (TranscriptionOutcome::Typed, notice)
                                            }
                                            Ok(utils::PasteOutcome::Blocked) => (
                                                TranscriptionOutcome::Failed,
                                                Some(Notice::new(
                                                    NoticeCode::PasteBlockedElevated,
                                                    NoticeSource::Dictation,
                                                )),
                                            ),
                                            Err(Some(e)) => {
                                                error!("Failed to paste transcription: {}", e);
                                                (
                                                    TranscriptionOutcome::Failed,
                                                    Some(Notice::new(
                                                        NoticeCode::PasteFailed,
                                                        NoticeSource::Dictation,
                                                    )),
                                                )
                                            }
                                        };
                                        match (owns_ui(), notice) {
                                            (true, Some(notice)) => {
                                                notices::raise(&ah_clone, notice)
                                            }
                                            (true, None) => {
                                                if get_settings(&ah_clone).paste_method
                                                    == crate::settings::PasteMethod::None
                                                {
                                                    utils::hide_recording_overlay(&ah_clone);
                                                } else {
                                                    crate::overlay::show_pasted_overlay(&ah_clone);
                                                }
                                            }
                                            (false, Some(notice)) => {
                                                notices::raise_without_overlay(&ah_clone, notice)
                                            }
                                            (false, None) => {}
                                        }
                                        if owns_ui() {
                                            set_tray_state(&ah_clone, TrayIconState::Idle);
                                        }
                                        emit_transcription_complete(
                                            &ah_clone,
                                            outcome,
                                            sample_count,
                                            stop_time,
                                        );
                                    })
                                    .map(|_| ())
                                    .unwrap_or_else(|e| {
                                        error!("Failed to start the paste thread: {:?}", e);
                                        utils::hide_recording_overlay(&ah);
                                        set_tray_state(&ah, TrayIconState::Idle);
                                        emit_transcription_complete(
                                            &ah,
                                            TranscriptionOutcome::Failed,
                                            sample_count,
                                            stop_time,
                                        );
                                    });
                            }
                        }
                        Err(err) => {
                            if rm.was_cancelled_since(cancel_generation) {
                                debug!(
                                    "Transcription operation cancelled after transcription error"
                                );
                                discard_cancelled_wav(&wav_path_for_verify);
                                utils::hide_recording_overlay(&ah);
                                set_tray_state(&ah, TrayIconState::Idle);
                                return;
                            }

                            error!("Transcription failed: {}", err);
                            notices::raise(
                                &ah,
                                Notice::new(
                                    NoticeCode::TranscriptionFailed,
                                    NoticeSource::Dictation,
                                )
                                .with_detail(err.to_string()),
                            );
                            // Save entry with empty text so user can retry
                            if wav_saved {
                                if let Err(save_err) = hm.save_entry(
                                    file_name,
                                    String::new(),
                                    post_process,
                                    None,
                                    None,
                                    None,
                                    true,
                                ) {
                                    error!("Failed to save failed history entry: {}", save_err);
                                }
                            }
                            set_tray_state(&ah, TrayIconState::Idle);
                            emit_transcription_complete(
                                &ah,
                                TranscriptionOutcome::Failed,
                                sample_count,
                                stop_time,
                            );
                        }
                    }
                }
            } else {
                debug!("No samples retrieved from recording stop");
                // Tear down any streaming worker so its channel doesn't leak.
                tm.cancel_stream();
                utils::hide_recording_overlay(&ah);
                set_tray_state(&ah, TrayIconState::Idle);
                if !rm.was_cancelled_since(cancel_generation) {
                    emit_transcription_complete(&ah, TranscriptionOutcome::Empty, 0, stop_time);
                }
            }
        });

        debug!(
            "TranscribeAction::stop completed in {:?}",
            stop_time.elapsed()
        );
    }
}

// Cancel Action
struct CancelAction;

impl ShortcutAction for CancelAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        utils::cancel_current_operation(app);
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on stop for cancel
    }
}

// Test Action
struct TestAction;

impl ShortcutAction for TestAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Started - {} (App: {})", // Changed "Pressed" to "Started" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Stopped - {} (App: {})", // Changed "Released" to "Stopped" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }
}

// Static Action Map
pub static ACTION_MAP: Lazy<HashMap<String, Arc<dyn ShortcutAction>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "transcribe".to_string(),
        Arc::new(TranscribeAction {
            post_process: false,
            instruct: false,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_with_post_process".to_string(),
        Arc::new(TranscribeAction {
            post_process: true,
            instruct: false,
        }) as Arc<dyn ShortcutAction>,
    );
    // Handy Adaptive (F121–F123).
    map.insert(
        "instruct".to_string(),
        Arc::new(TranscribeAction {
            post_process: true,
            instruct: true,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "polish".to_string(),
        Arc::new(TransformAction { slot: None }) as Arc<dyn ShortcutAction>,
    );
    for i in 0..crate::adaptive::MAX_TRANSFORMS {
        map.insert(
            format!("transform_{}", i + 1),
            Arc::new(TransformAction { slot: Some(i) }) as Arc<dyn ShortcutAction>,
        );
    }
    map.insert(
        "cancel".to_string(),
        Arc::new(CancelAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "test".to_string(),
        Arc::new(TestAction) as Arc<dyn ShortcutAction>,
    );
    map
});

#[cfg(test)]
mod tests {

    #[test]
    fn a_long_dictation_gets_a_longer_rewrite_budget() {
        use std::time::Duration;
        let words = |n: usize| vec!["mot"; n].join(" ");
        // Short: the setting.
        assert_eq!(
            super::rewrite_timeout(8, &words(10)),
            Duration::from_secs(8)
        );
        // 150 words (~45 s of speech): 7 s + 60 ms a word.
        assert_eq!(
            super::rewrite_timeout(8, &words(150)),
            Duration::from_secs(16)
        );
        // A longer setting still wins; never past the hard cap.
        assert_eq!(
            super::rewrite_timeout(20, &words(150)),
            Duration::from_secs(20)
        );
        assert_eq!(
            super::rewrite_timeout(8, &words(5000)),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn ctrl_c_only_when_ui_automation_cannot_see_the_selection() {
        use crate::context::Field;
        let field = |caret_known| Field {
            caret_known,
            ..Default::default()
        };
        // No field, or a field without a known caret (Google Docs): copy.
        assert!(super::may_copy_selection(None));
        assert!(super::may_copy_selection(Some(&field(false))));
        // UI Automation sees the caret and no selection: nothing is selected;
        // Ctrl+C would interrupt a terminal or copy the whole line.
        assert!(!super::may_copy_selection(Some(&field(true))));
    }

    #[test]
    fn a_blocked_transformation_paste_is_reported() {
        use crate::notices::NoticeCode;
        use crate::utils::PasteOutcome;
        assert_eq!(
            super::transform_paste_notice(&Ok(PasteOutcome::Pasted)),
            None
        );
        assert_eq!(
            super::transform_paste_notice(&Ok(PasteOutcome::Blocked)),
            Some(NoticeCode::PasteBlockedElevated)
        );
        assert_eq!(
            super::transform_paste_notice(&Err("x".into())),
            Some(NoticeCode::PasteFailed)
        );
    }

    #[test]
    fn a_partial_take_after_a_microphone_failure_says_so() {
        use crate::notices::NoticeCode::*;
        assert_eq!(super::typed_notice(None, true, false), Some(MicrophoneLost));
        assert_eq!(super::typed_notice(None, true, true), Some(MicrophoneLost));
        assert_eq!(
            super::typed_notice(None, false, true),
            Some(PostProcessFailed)
        );
        assert_eq!(super::typed_notice(None, false, false), None);
    }

    #[test]
    fn an_empty_result_always_says_why() {
        use crate::notices::NoticeCode::*;
        let second = crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE as usize;
        assert_eq!(super::empty_notice(false, true, 0), Some(MicrophoneLost));
        assert_eq!(super::empty_notice(true, true, 0), Some(MicrophoneLost));
        assert_eq!(super::empty_notice(true, false, 0), Some(MicrophoneSilent));
        assert_eq!(
            super::empty_notice(false, false, 2 * second),
            Some(NothingRecognized)
        );
        // A tap or a cough: no notice, even once padded to 1.25 s.
        assert_eq!(super::empty_notice(false, false, second / 2), None);
        assert_eq!(super::empty_notice(false, false, second * 5 / 4), None);
    }

    #[test]
    fn lot_1_3_cancelled_wav_is_removed() {
        let path = std::env::temp_dir().join(format!("handy-cancel-{}.wav", std::process::id()));
        std::fs::write(&path, b"test").unwrap();
        super::discard_cancelled_wav(&path);
        assert!(!path.exists());
    }

    #[test]
    fn lot_1_3_failed_instruction_is_kept_in_history() {
        assert_eq!(
            super::failed_instruction_for_history(false, "ma consigne"),
            Some("ma consigne".to_string())
        );
        assert_eq!(super::failed_instruction_for_history(true, "secret"), None);
    }

    #[test]
    fn every_transformation_slot_has_an_action() {
        for i in 1..=crate::adaptive::MAX_TRANSFORMS {
            assert!(
                super::ACTION_MAP.contains_key(&format!("transform_{i}")),
                "transform_{i}"
            );
        }
    }
    use super::{
        complete_unless_cancelled, house_framing, is_blank_transcription, learned_fallback,
        non_blank_post_processed_text, observable, post_process_transcription,
        should_use_streaming_overlay, strip_invisible_chars, strip_think_block, transform_text,
        PostProcessOutcome,
    };

    #[test]
    fn a_private_field_is_never_observed() {
        assert!(observable(false, false, false));
        assert!(!observable(true, false, false));
        assert!(!observable(false, true, false));
        // The password check didn't answer: private too (spec R1).
        assert!(!observable(false, false, true));
    }

    #[test]
    fn rewrite_is_skipped_only_for_password_or_excluded_targets() {
        assert!(!super::rewrite_allowed(true, false, true, false));
        assert!(!super::rewrite_allowed(true, true, false, false));
        // Unknown site or unanswered password check: the rewrite still runs.
        assert!(super::rewrite_allowed(true, false, false, false));
        assert_eq!(
            super::rewrite_skip_notice(true, true, false),
            Some(super::NoticeCode::PrivacyRewriteSkipped)
        );
        assert_eq!(super::rewrite_skip_notice(true, false, false), None);
        assert_eq!(super::rewrite_skip_notice(false, true, false), None);
    }

    #[test]
    fn transformation_runs_when_checks_could_not_be_made() {
        let mut capture = crate::context::Capture::default();
        assert!(super::selection_allowed(&capture));
        capture.excluded = true;
        assert!(!super::selection_allowed(&capture));
        assert_eq!(
            super::selection_notice(&capture),
            super::NoticeCode::PrivacyRewriteSkipped
        );
    }

    #[test]
    fn fallback_runs_after_a_rejected_rewrite() {
        // No rewrite text came out (failed, skipped, rejected, timed out, not asked).
        assert!(learned_fallback(false, false, false, false));
        // A rewrite came out: the LLM already checked the context.
        assert!(!learned_fallback(true, false, false, false));
    }

    #[test]
    fn no_fallback_for_a_password_field() {
        assert!(!learned_fallback(false, false, false, true));
        // Nor for an instruction or a snippet.
        assert!(!learned_fallback(false, true, false, false));
        assert!(!learned_fallback(false, false, true, false));
    }

    #[test]
    fn prompts_without_output_placeholder_get_house_framing() {
        let house = crate::adaptive::HOUSE_PROMPT_ID;
        assert_eq!(
            house_framing(house, "ignored").as_deref(),
            Some(crate::adaptive::HOUSE_PROMPT)
        );
        assert_eq!(
            house_framing("mine", "Rewrite it.").as_deref(),
            Some("Rewrite it.")
        );
        assert_eq!(house_framing("mine", "Fix: ${output}"), None);
    }
    use crate::settings::{get_default_settings, AppSettings, OverlayStyle};
    use std::future;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    /// Settings that run unstructured post-processing against `base_url`.
    fn post_process_settings(base_url: &str) -> AppSettings {
        let mut settings = get_default_settings();
        let provider = settings
            .post_process_providers
            .iter_mut()
            .find(|provider| provider.id == settings.post_process_provider_id)
            .expect("the default provider exists");
        provider.base_url = base_url.to_string();
        provider.supports_structured_output = false;
        let provider_id = provider.id.clone();
        settings
            .post_process_models
            .insert(provider_id.clone(), "model".to_string());
        settings
            .post_process_api_keys
            .insert(provider_id, "key".into());
        crate::features::migrate(&mut settings);
        settings.post_process_selected_prompt_id =
            settings.post_process_prompts.first().map(|p| p.id.clone());
        settings
    }

    /// Answers one chat completion request with `body`, and returns its URL.
    fn serve_once(body: &'static str) -> String {
        serve_status("200 OK", body)
    }

    fn serve_status(status: &'static str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 8192];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{address}")
    }

    fn run_post_process(settings: &AppSettings, transcription: &str) -> PostProcessOutcome {
        tauri::async_runtime::block_on(post_process_transcription(
            settings,
            transcription,
            None,
            None,
        ))
    }

    #[test]
    fn blank_speech_is_skipped_but_missing_setup_failed() {
        let settings = post_process_settings("http://127.0.0.1:1");
        assert_eq!(
            run_post_process(&settings, "  "),
            PostProcessOutcome::Skipped
        );

        let mut settings = post_process_settings("http://127.0.0.1:1");
        settings.features.rewrite.model.clear();
        settings.features.rewrite_fallback = Default::default();
        assert_eq!(
            run_post_process(&settings, "hello"),
            PostProcessOutcome::Failed
        );
    }

    /// Main provider rate-limited (429); Cloudflare set up as the fallback.
    fn with_fallback(fallback_body: &'static str) -> AppSettings {
        let mut settings = post_process_settings(&serve_status(
            "429 Too Many Requests",
            r#"{"error":{"message":"rate limit"}}"#,
        ));
        let url = serve_once(fallback_body);
        settings
            .post_process_provider_mut("cloudflare")
            .expect("cloudflare is a default provider")
            .base_url = url;
        settings
            .post_process_api_keys
            .insert("cloudflare".into(), "key".into());
        settings
    }

    #[test]
    fn the_fallback_provider_answers_when_the_main_one_fails() {
        let settings = with_fallback(r#"{"choices":[{"message":{"content":"Relais OK."}}]}"#);
        assert_eq!(
            run_post_process(&settings, "relais ok"),
            PostProcessOutcome::Done("Relais OK.".to_string())
        );
    }

    #[test]
    fn fallback_answers_when_the_rewrite_key_is_missing() {
        let mut settings = with_fallback(r#"{"choices":[{"message":{"content":"Relais OK."}}]}"#);
        settings
            .post_process_api_keys
            .remove(&settings.features.rewrite.provider_id);
        assert_eq!(
            run_post_process(&settings, "relais ok"),
            PostProcessOutcome::Done("Relais OK.".to_string())
        );
    }

    #[test]
    fn a_fallback_without_a_key_is_skipped() {
        let mut settings = with_fallback(r#"{"choices":[{"message":{"content":"x"}}]}"#);
        settings
            .post_process_api_keys
            .insert("cloudflare".into(), String::new());
        assert_eq!(
            run_post_process(&settings, "relais ok"),
            PostProcessOutcome::Failed
        );
    }

    #[test]
    fn a_transformation_falls_back_and_a_refusal_keeps_the_text() {
        let settings = with_fallback(r#"{"choices":[{"message":{"content":"Texte poli."}}]}"#);
        assert_eq!(
            tauri::async_runtime::block_on(transform_text(
                &settings,
                "Polis",
                "texte poli",
                None,
                None
            )),
            Some("Texte poli.".to_string())
        );
        let refusal =
            r#"{"choices":[{"message":{"content":"I'm sorry, but I can't help with that."}}]}"#;
        let mut settings = post_process_settings(&serve_once(refusal));
        settings.features.rewrite_fallback = Default::default();
        assert_eq!(
            tauri::async_runtime::block_on(transform_text(
                &settings,
                "Polis",
                "texte poli",
                None,
                None
            )),
            None
        );
    }

    #[test]
    fn a_post_processing_reply_is_done() {
        let url = serve_once(r#"{"choices":[{"message":{"content":"polished"}}]}"#);
        assert_eq!(
            run_post_process(&post_process_settings(&url), "hello"),
            PostProcessOutcome::Done("polished".to_string())
        );
    }

    #[test]
    fn a_post_processing_reply_with_no_content_failed() {
        let url = serve_once(r#"{"choices":[]}"#);
        assert_eq!(
            run_post_process(&post_process_settings(&url), "hello"),
            PostProcessOutcome::Failed
        );
    }

    #[test]
    fn blank_transcription_is_detected() {
        assert!(is_blank_transcription(""));
        assert!(is_blank_transcription("   "));
        assert!(is_blank_transcription("\t\n  \r\n"));
    }

    #[test]
    fn non_blank_transcription_is_kept() {
        assert!(!is_blank_transcription("hello"));
        assert!(!is_blank_transcription("  hello  "));
    }

    #[test]
    fn blank_post_processed_text_is_rejected() {
        for text in ["", "   ", "\t\n\r\n", "\u{00A0}\u{2003}"] {
            assert_eq!(non_blank_post_processed_text(text.to_string()), None);
        }
    }

    #[test]
    fn cleaned_empty_post_processed_text_is_rejected() {
        for text in [
            "\u{200B}\u{200C}\u{200D}\u{FEFF}",
            "<think>reasoning only</think>",
            "<think>reasoning only</think>\n\u{200B} ",
        ] {
            let text = strip_invisible_chars(strip_think_block(text));
            assert_eq!(non_blank_post_processed_text(text), None);
        }
    }

    #[test]
    fn non_blank_post_processed_text_is_preserved() {
        for text in ["Cleaned text.", "  정리된 문장입니다.\n", "0"] {
            assert_eq!(
                non_blank_post_processed_text(text.to_string()),
                Some(text.to_string())
            );
        }
    }

    #[test]
    fn completed_operation_returns_its_output() {
        let result = tauri::async_runtime::block_on(complete_unless_cancelled(
            future::ready("done"),
            || false,
        ));

        assert_eq!(result, Some("done"));
    }

    #[test]
    fn pending_operation_stops_after_cancellation() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_for_thread = Arc::clone(&cancelled);
        let cancel_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10));
            cancelled_for_thread.store(true, Ordering::Release);
        });

        let result = tauri::async_runtime::block_on(complete_unless_cancelled(
            future::pending::<()>(),
            || cancelled.load(Ordering::Acquire),
        ));

        cancel_thread.join().unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn leading_think_block_is_stripped() {
        assert_eq!(
            strip_think_block("<think>pondering...</think>Cleaned text."),
            "Cleaned text."
        );
        assert_eq!(
            strip_think_block("  \n<think>multi\nline</think>\n  Cleaned text."),
            "Cleaned text."
        );
    }

    #[test]
    fn content_without_think_block_is_unchanged() {
        assert_eq!(strip_think_block("Cleaned text."), "Cleaned text.");
        assert_eq!(
            strip_think_block("Mentions <think> mid-sentence."),
            "Mentions <think> mid-sentence."
        );
        // Unclosed block: leave untouched rather than guess
        assert_eq!(strip_think_block("<think>never closed"), "");
    }

    #[test]
    fn live_overlay_uses_streaming_states_only_for_streaming_models() {
        assert!(should_use_streaming_overlay(OverlayStyle::Live, true));
        assert!(!should_use_streaming_overlay(OverlayStyle::Live, false));
        assert!(!should_use_streaming_overlay(OverlayStyle::Minimal, true));
        assert!(!should_use_streaming_overlay(OverlayStyle::None, true));
    }
}
