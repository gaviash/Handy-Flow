//! Handy Adaptive: transcription by an OpenAI-compatible speech API (Groq
//! Whisper, OpenAI, a custom server), with the same provider list and API keys
//! as the rewrite. The local model stays the fallback: a failed call never
//! loses the dictation.

use crate::settings::{AppSettings, PostProcessProvider};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::time::Duration;

/// Settings → Models → Transcription by API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct CloudAsrSettings {
    /// A post-processing provider id ("groq", "openai", "custom"); empty =
    /// the local model.
    pub provider_id: String,
    /// The model per provider, as the rewrite keeps it.
    pub models: std::collections::HashMap<String, String>,
    /// How long the API may take before the local model transcribes.
    pub timeout_secs: u64,
}

impl Default for CloudAsrSettings {
    fn default() -> Self {
        Self {
            provider_id: String::new(),
            models: default_models(),
            timeout_secs: 20,
        }
    }
}

/// Providers with a speech endpoint, and their suggested models (first =
/// default).
pub const PROVIDERS: &[(&str, &[&str])] = &[
    ("groq", &["whisper-large-v3", "whisper-large-v3-turbo"]),
    (
        "openai",
        &["whisper-1", "gpt-4o-transcribe", "gpt-4o-mini-transcribe"],
    ),
    ("custom", &[]),
];

fn default_models() -> std::collections::HashMap<String, String> {
    PROVIDERS
        .iter()
        .filter_map(|(id, models)| Some((id.to_string(), models.first()?.to_string())))
        .collect()
}

impl CloudAsrSettings {
    /// Bounds the settings page can't exceed (trust boundary: the IPC call).
    pub fn validate(&self) -> Result<(), String> {
        if !self.provider_id.is_empty() && !PROVIDERS.iter().any(|(id, _)| *id == self.provider_id)
        {
            return Err(format!("unknown speech provider '{}'", self.provider_id));
        }
        if !(3..=120).contains(&self.timeout_secs) {
            return Err("timeout_secs must be between 3 and 120".into());
        }
        Ok(())
    }

    #[cfg(all(test, target_os = "windows"))]
    pub fn model(&self) -> &str {
        self.models
            .get(&self.provider_id)
            .map(|m| m.trim())
            .unwrap_or("")
    }
}

/// A usable speech API: provider, key and model all set.
#[derive(Clone)]
pub struct CloudAsr {
    pub provider: PostProcessProvider,
    pub key: String,
    pub model: String,
    pub timeout: Duration,
}

/// The speech API to use, or `None` for the local model (none chosen, or no
/// key or model for it).
pub fn configured(settings: &AppSettings) -> Option<CloudAsr> {
    use crate::features::{feature_target, Feature};
    let Some((provider, key, model, _)) = feature_target(settings, Feature::TranscriptionApi)
    else {
        if !settings.features.transcription_api.provider_id.is_empty() {
            log::warn!("Speech API is not configured with a usable model; using the local model");
        }
        return None;
    };
    Some(CloudAsr {
        provider,
        key,
        model,
        timeout: Duration::from_secs(settings.cloud_asr.timeout_secs),
    })
}

/// The language to ask for: the selected one as an ISO-639-1 code, or `None`
/// to let the API detect it.
pub fn language(settings: &AppSettings) -> Option<String> {
    let l = settings.selected_language.trim();
    if l.is_empty() || l.eq_ignore_ascii_case("auto") {
        return None;
    }
    let code = l.split(['-', '_']).next()?.to_lowercase();
    (code.len() == 2 || code.len() == 3).then_some(code)
}

/// 16 kHz mono PCM as a 16-bit WAV file.
pub fn wav(samples: &[f32]) -> Result<Vec<u8>, String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = std::io::Cursor::new(Vec::with_capacity(44 + samples.len() * 2));
    {
        let mut w = hound::WavWriter::new(&mut out, spec).map_err(|e| e.to_string())?;
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
            w.write_sample(v).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
    }
    Ok(out.into_inner())
}

#[derive(Deserialize)]
struct Reply {
    text: String,
}

/// The provider's `error.message`, short, as `: …`; nothing else of the body
/// (it could echo what was sent).
pub(crate) fn error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
        .map(|m| format!(": {}", m.chars().take(120).collect::<String>()))
        .unwrap_or_default()
}

/// Transcribes `samples` with the speech API. `prompt` is Whisper's
/// conditioning text (spellings, expressions, previous text).
pub async fn transcribe(
    asr: &CloudAsr,
    samples: &[f32],
    language: Option<&str>,
    prompt: Option<&str>,
    translate: bool,
) -> Result<String, String> {
    // Whisper's translation endpoint writes English and takes no language.
    let (endpoint, language) = if translate {
        ("translations", None)
    } else {
        ("transcriptions", language)
    };
    let url = format!(
        "{}/audio/{endpoint}",
        asr.provider.base_url.trim_end_matches('/')
    );
    let file = reqwest::multipart::Part::bytes(wav(samples)?)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", file)
        .text("model", asr.model.clone())
        .text("response_format", "json")
        .text("temperature", "0");
    if let Some(l) = language {
        form = form.text("language", l.to_string());
    }
    if let Some(p) = prompt.filter(|p| !p.trim().is_empty()) {
        form = form.text("prompt", p.to_string());
    }
    let mut request = crate::llm_client::shared_http_client()?
        .post(&url)
        .header(
            reqwest::header::USER_AGENT,
            "Handy-Flow (+https://github.com/gaviash/Handy-Flow)",
        )
        .timeout(asr.timeout)
        .multipart(form);
    if !asr.key.is_empty() {
        request = request.bearer_auth(&asr.key);
    }
    let response = request.send().await.map_err(|e| {
        if e.is_timeout() {
            format!("timed out after {:?}", asr.timeout)
        } else {
            // Without the URL: a custom one may hold a token.
            format!("request failed: {}", e.without_url())
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        let mut body = response.text().await.unwrap_or_default();
        if !asr.key.is_empty() {
            body = body.replace(&asr.key, "[REDACTED]");
        }
        return Err(format!("HTTP {status}{}", error_message(&body)));
    }
    let reply: Reply = response
        .json()
        .await
        .map_err(|e| format!("unreadable reply: {e}"))?;
    Ok(reply.text.trim().to_string())
}

#[tauri::command]
#[specta::specta]
pub fn change_cloud_asr_setting(
    app: tauri::AppHandle,
    value: CloudAsrSettings,
) -> Result<(), String> {
    value.validate()?;
    let mut settings = crate::settings::get_settings(&app);
    settings.cloud_asr = value;
    crate::settings::write_settings(&app, settings);
    Ok(())
}

/// One transcription that never returns its own prompt: a copied head is cut
/// off; an answer that is all prompt is asked again once without it (said
/// alone, known words then come back as speech). `Err` only when the first
/// call fails; the second call failing gives no text.
pub async fn without_echo<F, Fut>(
    prompt: Option<&str>,
    mut call: F,
) -> Result<(String, crate::asr_context::Echo), String>
where
    F: FnMut(Option<String>) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    use crate::asr_context::{strip_echo, Echo};
    let first = call(prompt.map(str::to_string)).await?;
    let (text, echo) = strip_echo(&first, prompt);
    if echo != Echo::All {
        return Ok((text, echo));
    }
    Ok((call(None).await.unwrap_or_default(), Echo::All))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asr_context::Echo;

    /// Runs `without_echo` with canned replies; returns its result and the
    /// prompts the calls saw.
    fn run(
        prompt: Option<&str>,
        replies: Vec<Result<&str, &str>>,
    ) -> (Result<(String, Echo), String>, Vec<Option<String>>) {
        let seen = std::cell::RefCell::new(Vec::new());
        let replies = std::cell::RefCell::new(replies.into_iter());
        let out = tauri::async_runtime::block_on(without_echo(prompt, |p| {
            seen.borrow_mut().push(p);
            let r = replies.borrow_mut().next().expect("one call too many");
            std::future::ready(r.map(str::to_string).map_err(str::to_string))
        }));
        (out, seen.into_inner())
    }

    #[test]
    fn a_fully_copied_prompt_is_asked_again_without_it() {
        let p = Some("Vocabulaire : GitHub, Claude.");
        let (out, seen) = run(p, vec![Ok("Vocabulaire : GitHub, Claude."), Ok("Bonjour.")]);
        assert_eq!(out, Ok(("Bonjour.".into(), Echo::All)));
        assert_eq!(seen, vec![p.map(str::to_string), None]);

        let (out, seen) = run(p, vec![Ok("Bonjour.")]);
        assert_eq!(out, Ok(("Bonjour.".into(), Echo::None)));
        assert_eq!(seen.len(), 1);

        // Said alone, the words come back from the call without a prompt.
        let p = Some("Vocabulaire : Claude, Handy, Groq.");
        let (out, seen) = run(
            p,
            vec![Ok("Claude, Handy, Groq."), Ok("Claude, Handy, Groq.")],
        );
        assert_eq!(out, Ok(("Claude, Handy, Groq.".into(), Echo::All)));
        assert_eq!(seen.len(), 2);

        let full = "Vocabulaire : Claude, Handy, Groq.";
        assert_eq!(
            run(p, vec![Ok(full), Ok("")]).0,
            Ok((String::new(), Echo::All))
        );
        assert_eq!(
            run(p, vec![Ok(full), Err("timed out")]).0,
            Ok((String::new(), Echo::All))
        );
        let (out, seen) = run(p, vec![Err("HTTP 500")]);
        assert_eq!(out, Err("HTTP 500".into()));
        assert_eq!(seen.len(), 1);
    }

    #[test]
    fn a_copied_head_is_cut_without_a_second_call() {
        let p = Some("Vocabulaire : GitHub, Claude, Handy.");
        let (out, seen) = run(p, vec![Ok("Vocabulaire : GitHub, Claude. On y va.")]);
        assert_eq!(out, Ok(("On y va.".into(), Echo::Lead)));
        assert_eq!(seen.len(), 1);
    }

    #[test]
    fn escape_stops_the_second_call() {
        let p = Some("Vocabulaire : GitHub, Claude.");
        let mut first = true;
        let call = without_echo(p, |_| {
            let reply =
                std::mem::take(&mut first).then(|| "Vocabulaire : GitHub, Claude.".to_string());
            async move {
                match reply {
                    Some(r) => Ok(r),
                    None => std::future::pending().await,
                }
            }
        });
        let out =
            tauri::async_runtime::block_on(crate::actions::complete_unless_cancelled(call, || {
                true
            }));
        assert_eq!(out, None);
    }

    #[test]
    fn the_wav_is_16_khz_mono_16_bit() {
        let bytes = wav(&[0.0, 0.5, -1.0, 2.0]).unwrap();
        let r = hound::WavReader::new(std::io::Cursor::new(bytes)).unwrap();
        let spec = r.spec();
        assert_eq!(
            (spec.channels, spec.sample_rate, spec.bits_per_sample),
            (1, 16_000, 16)
        );
        let s: Vec<i16> = r.into_samples().map(Result::unwrap).collect();
        assert_eq!(s, vec![0, 16383, -32767, 32767]);
    }

    #[test]
    fn only_the_providers_error_message_is_kept() {
        assert_eq!(
            error_message(r#"{"error":{"message":"Rate limit reached","type":"x"}}"#),
            ": Rate limit reached"
        );
        assert_eq!(error_message("<html>Bearer sk-secret</html>"), "");
    }

    #[test]
    fn the_language_is_a_short_code_or_auto() {
        let mut s = crate::settings::get_default_settings();
        s.selected_language = "auto".into();
        assert_eq!(language(&s), None);
        s.selected_language = "fr".into();
        assert_eq!(language(&s).as_deref(), Some("fr"));
        s.selected_language = "zh-Hans".into();
        assert_eq!(language(&s).as_deref(), Some("zh"));
    }

    #[test]
    fn groq_whisper_needs_a_key_and_a_model() {
        let mut s = crate::settings::get_default_settings();
        assert!(configured(&s).is_none(), "local by default");
        s.cloud_asr.provider_id = "groq".into();
        crate::features::migrate(&mut s);
        s.post_process_api_keys.insert("groq".into(), String::new());
        assert!(configured(&s).is_none(), "no key");
        s.post_process_api_keys.insert("groq".into(), "k".into());
        let c = configured(&s).unwrap();
        assert_eq!(c.model, "whisper-large-v3");
        assert!(c.provider.base_url.contains("groq.com"));
        s.features.transcription_api.model.clear();
        assert!(configured(&s).is_none(), "no model");
        assert!(CloudAsrSettings {
            provider_id: "nope".into(),
            ..Default::default()
        }
        .validate()
        .is_err());
    }
}
