//! Handy Adaptive (spec §7): the context an ASR model gets, built from what
//! it accepts. Pure: no Tauri, no settings store — `transcription.rs` feeds it.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Whisper's prompt holds about 224 tokens; this stays well under it.
pub const PROMPT_MAX_CHARS: usize = 600;

/// What the loaded model accepts (transcribe-cpp features).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AsrCaps {
    pub vocabulary: bool,
    pub context_prompt: bool,
    pub initial_prompt: bool,
}

/// Everything the context may draw from, gathered by the caller.
#[derive(Debug, Clone, Default)]
pub struct AsrSources {
    pub custom_words: Vec<String>,
    /// Learned terms alone (no "heard as"), most relevant first.
    pub learned: Vec<String>,
    pub expressions: Vec<String>,
    pub app: Option<String>,
    pub before: Option<String>,
    pub language: Option<String>,
}

/// User settings (Settings → Learning → Transcription context).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct AsrContextSettings {
    pub enabled: bool,
    pub learned_words: bool,
    pub expressions: bool,
    /// The app and the text before the cursor; makes the ASR wait for the capture.
    pub screen: bool,
    pub max_terms: usize,
    pub capture_wait_ms: u64,
}

impl Default for AsrContextSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            learned_words: true,
            expressions: true,
            screen: false,
            max_terms: 50,
            capture_wait_ms: 150,
        }
    }
}

impl AsrContextSettings {
    /// Bounds the settings page can't exceed (trust boundary: the IPC call).
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=200).contains(&self.max_terms) {
            return Err("max_terms must be between 1 and 200".into());
        }
        if self.capture_wait_ms > 1000 {
            return Err("capture_wait_ms must be at most 1000".into());
        }
        Ok(())
    }
}

/// What the loaded model accepts, for the settings page (`None` = no model).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub struct AsrSupport {
    pub vocabulary: bool,
    pub prompt: bool,
}

/// Which prompt slot gets the text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AsrPrompt {
    #[default]
    None,
    /// `RunOptions::prompt` (Feature::ContextPrompt).
    Context(String),
    /// Whisper's initial prompt (Feature::InitialPrompt); never with vocabulary.
    Initial(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AsrContext {
    pub vocabulary: Vec<String>,
    pub prompt: AsrPrompt,
}

/// The context `caps` can take, from `src`, as `cfg` allows.
pub fn build(caps: AsrCaps, src: &AsrSources, cfg: &AsrContextSettings) -> AsrContext {
    if !cfg.enabled || caps == AsrCaps::default() {
        return AsrContext::default();
    }
    let mut terms: Vec<String> = Vec::new();
    let learned = src.learned.iter().filter(|_| cfg.learned_words);
    for t in src.custom_words.iter().chain(learned) {
        let t = t.trim();
        if !t.is_empty() && !terms.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            terms.push(t.to_string());
        }
    }
    terms.truncate(cfg.max_terms);
    let expressions: &[String] = if cfg.expressions {
        &src.expressions
    } else {
        &[]
    };
    let prompt_text = || prompt(&terms, expressions, src, cfg.screen);
    if caps.initial_prompt {
        return AsrContext {
            vocabulary: Vec::new(),
            prompt: prompt_text().map_or(AsrPrompt::None, AsrPrompt::Initial),
        };
    }
    AsrContext {
        vocabulary: if caps.vocabulary {
            terms.clone()
        } else {
            Vec::new()
        },
        prompt: if caps.context_prompt {
            prompt_text().map_or(AsrPrompt::None, AsrPrompt::Context)
        } else {
            AsrPrompt::None
        },
    }
}

/// One short conditioning text in the dictation's language, `None` if empty.
fn prompt(
    terms: &[String],
    expressions: &[String],
    src: &AsrSources,
    screen: bool,
) -> Option<String> {
    let fr = src.language.as_deref().is_none_or(|l| l.starts_with("fr"));
    let (vocab, expr, app, before) = if fr {
        (
            "Vocabulaire",
            "Expressions",
            "Application",
            "Texte précédent",
        )
    } else {
        ("Vocabulary", "Expressions", "App", "Previous text")
    };
    let mut parts = Vec::new();
    if !terms.is_empty() {
        parts.push(format!("{vocab} : {}.", terms.join(", ")));
    }
    if !expressions.is_empty() {
        let quoted: Vec<String> = expressions.iter().map(|e| format!("« {e} »")).collect();
        parts.push(format!("{expr} : {}.", quoted.join(", ")));
    }
    if screen {
        if let Some(a) = src.app.as_deref().filter(|a| !a.trim().is_empty()) {
            parts.push(format!("{app} : {a}."));
        }
        if let Some(b) = src.before.as_deref().filter(|b| !b.trim().is_empty()) {
            parts.push(format!("{before} : {}", b.trim()));
        }
    }
    if !fr {
        for p in parts.iter_mut() {
            *p = p.replacen(" : ", ": ", 1);
        }
    }
    let text = parts.join(" ");
    (!text.is_empty()).then(|| cut(&text, PROMPT_MAX_CHARS))
}

/// At most `max` chars, cut at a word.
fn cut(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max).collect();
    head.rsplit_once(' ').map_or(head.clone(), |(h, _)| {
        h.trim_end_matches([',', ' ']).to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> AsrSources {
        AsrSources {
            custom_words: vec!["Handy".into()],
            learned: vec!["log".into(), "Claude".into()],
            expressions: vec!["de manière claire et concise".into()],
            app: Some("slack.exe".into()),
            before: Some("on regarde le serveur".into()),
            language: Some("fr".into()),
        }
    }

    #[test]
    fn no_capability_builds_nothing() {
        assert_eq!(
            build(AsrCaps::default(), &src(), &AsrContextSettings::default()),
            AsrContext::default()
        );
    }

    #[test]
    fn vocabulary_lists_custom_then_learned_words_capped() {
        let cfg = AsrContextSettings {
            max_terms: 2,
            ..Default::default()
        };
        let caps = AsrCaps {
            vocabulary: true,
            ..Default::default()
        };
        let c = build(caps, &src(), &cfg);
        assert_eq!(c.vocabulary, vec!["Handy", "log"]);
        assert_eq!(c.prompt, AsrPrompt::None);
    }

    #[test]
    fn context_prompt_cites_terms_and_expressions_but_not_the_screen_by_default() {
        let caps = AsrCaps {
            context_prompt: true,
            ..Default::default()
        };
        let c = build(caps, &src(), &AsrContextSettings::default());
        let AsrPrompt::Context(p) = c.prompt else {
            panic!("context prompt expected")
        };
        assert!(
            p.contains("Claude") && p.contains("de manière claire et concise"),
            "{p}"
        );
        assert!(!p.contains("slack") && !p.contains("serveur"), "{p}");
        assert!(p.chars().count() <= PROMPT_MAX_CHARS);
        // With the screen option, the app and the text before the cursor too.
        let cfg = AsrContextSettings {
            screen: true,
            ..Default::default()
        };
        let AsrPrompt::Context(p) = build(caps, &src(), &cfg).prompt else {
            panic!()
        };
        assert!(
            p.contains("slack.exe") && p.contains("on regarde le serveur"),
            "{p}"
        );
    }

    #[test]
    fn whisper_initial_prompt_never_comes_with_vocabulary() {
        let caps = AsrCaps {
            vocabulary: true,
            initial_prompt: true,
            ..Default::default()
        };
        let c = build(caps, &src(), &AsrContextSettings::default());
        assert!(matches!(c.prompt, AsrPrompt::Initial(_)));
        assert!(c.vocabulary.is_empty());
    }

    #[test]
    fn disabled_or_sources_off_respected() {
        let caps = AsrCaps {
            context_prompt: true,
            vocabulary: true,
            ..Default::default()
        };
        let off = AsrContextSettings {
            enabled: false,
            ..Default::default()
        };
        assert_eq!(build(caps, &src(), &off), AsrContext::default());
        let cfg = AsrContextSettings {
            learned_words: false,
            expressions: false,
            ..Default::default()
        };
        let c = build(caps, &src(), &cfg);
        assert_eq!(c.vocabulary, vec!["Handy"]);
        // Nothing to say: no prompt at all.
        let empty = AsrSources::default();
        assert_eq!(
            build(caps, &empty, &AsrContextSettings::default()),
            AsrContext::default()
        );
    }

    #[test]
    fn settings_out_of_range_are_refused() {
        assert!(AsrContextSettings::default().validate().is_ok());
        let bad = |f: fn(&mut AsrContextSettings)| {
            let mut s = AsrContextSettings::default();
            f(&mut s);
            s.validate().is_err()
        };
        assert!(bad(|s| s.max_terms = 0));
        assert!(bad(|s| s.max_terms = 201));
        assert!(bad(|s| s.capture_wait_ms = 1001));
    }

    #[test]
    fn long_prompts_are_cut_at_a_word() {
        let mut s = src();
        s.learned = (0..200).map(|i| format!("terme{i}")).collect();
        let cfg = AsrContextSettings {
            max_terms: 200,
            ..Default::default()
        };
        let caps = AsrCaps {
            context_prompt: true,
            ..Default::default()
        };
        let AsrPrompt::Context(p) = build(caps, &s, &cfg).prompt else {
            panic!()
        };
        assert!(p.chars().count() <= PROMPT_MAX_CHARS);
        assert!(!p.ends_with("terme"), "{p}");
    }
}
