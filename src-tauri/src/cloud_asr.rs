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

    pub fn model(&self) -> &str {
        self.models
            .get(&self.provider_id)
            .map(|m| m.trim())
            .unwrap_or("")
    }
}

/// A usable speech API: provider, key and model all set.
#[derive(Debug, Clone)]
pub struct CloudAsr {
    pub provider: PostProcessProvider,
    pub key: String,
    pub model: String,
    pub timeout: Duration,
}

/// The speech API to use, or `None` for the local model (none chosen, or no
/// key or model for it).
pub fn configured(settings: &AppSettings) -> Option<CloudAsr> {
    let cfg = &settings.cloud_asr;
    let id = cfg.provider_id.as_str();
    if id.is_empty() {
        return None;
    }
    let provider = settings.post_process_provider(id)?.clone();
    let key = settings
        .post_process_api_keys
        .get(id)
        .cloned()
        .unwrap_or_default();
    // A local custom server needs no key; a cloud provider does.
    if key.trim().is_empty() && id != "custom" {
        log::warn!("Speech API '{id}' has no API key; using the local model");
        return None;
    }
    let model = cfg.model().to_string();
    if model.is_empty() {
        log::warn!("Speech API '{id}' has no model; using the local model");
        return None;
    }
    Some(CloudAsr {
        provider,
        key,
        model,
        timeout: Duration::from_secs(cfg.timeout_secs),
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

/// Transcribes `samples` with the speech API. `prompt` is Whisper's
/// conditioning text (spellings, expressions, previous text).
pub async fn transcribe(
    asr: &CloudAsr,
    samples: &[f32],
    language: Option<&str>,
    prompt: Option<&str>,
) -> Result<String, String> {
    let url = format!(
        "{}/audio/transcriptions",
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
            "Handy/1.0 (+https://github.com/cjpais/Handy)",
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
            format!("request failed: {e}")
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        // The body is the provider's error message, never the audio.
        let body: String = response
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        return Err(format!("HTTP {status}: {body}"));
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

#[cfg(test)]
mod tests {
    use super::*;

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
        s.post_process_api_keys.insert("groq".into(), String::new());
        assert!(configured(&s).is_none(), "no key");
        s.post_process_api_keys.insert("groq".into(), "k".into());
        let c = configured(&s).unwrap();
        assert_eq!(c.model, "whisper-large-v3");
        assert!(c.provider.base_url.contains("groq.com"));
        s.cloud_asr.models.insert("groq".into(), " ".into());
        assert!(configured(&s).is_none(), "no model");
        assert!(CloudAsrSettings {
            provider_id: "nope".into(),
            ..Default::default()
        }
        .validate()
        .is_err());
    }
}
