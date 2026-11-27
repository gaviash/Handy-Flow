//! Handy Adaptive (spec §7): the context an ASR model gets, built from what
//! it accepts. Pure: no Tauri, no settings store — `transcription.rs` feeds it.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Whisper's prompt holds about 224 tokens; this stays well under it.
pub const PROMPT_MAX_CHARS: usize = 600;
/// Whisper keeps only the last ~223 prompt tokens: past them the label and the
/// first (most wanted) terms would be dropped. Estimated, with margin.
const PROMPT_MAX_TOKENS: usize = 200;

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
    /// Names and terms seen around the cursor (spec R10, Wispr E140).
    pub screen_terms: Vec<String>,
    pub language: Option<String>,
}

/// What the screen gives the ASR: the names and terms around the cursor.
/// Never from a private capture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AsrScreen {
    pub terms: Vec<String>,
}

/// User settings (Settings → Learning → Transcription context). An old
/// `expressions` key is ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct AsrContextSettings {
    pub enabled: bool,
    pub learned_words: bool,
    /// The names read on screen; makes the ASR wait for the capture.
    pub screen: bool,
    pub max_terms: usize,
    pub capture_wait_ms: u64,
}

impl Default for AsrContextSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            learned_words: true,
            // Wispr uses the screen by default (E140).
            screen: true,
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
    let screen = src.screen_terms.iter().filter(|_| cfg.screen);
    // Spec R10: the user's words, then the screen's, then the learned ones.
    for t in src.custom_words.iter().chain(screen).chain(learned) {
        let t = t.trim();
        if !t.is_empty() && !terms.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            terms.push(t.to_string());
        }
    }
    terms.truncate(cfg.max_terms);
    let prompt_text = || prompt(&terms, src.language.as_deref());
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

/// The user's own words: their name first (the rewrite gets it, the speech
/// model didn't), then their custom words.
pub fn user_words(user_name: &str, custom_words: &[String]) -> Vec<String> {
    let name = user_name.trim();
    std::iter::once(name.to_string())
        .filter(|n| !n.is_empty())
        .chain(
            custom_words
                .iter()
                .filter(|w| !w.trim().eq_ignore_ascii_case(name))
                .cloned(),
        )
        .collect()
}

/// The vocabulary alone, in the dictation's language, `None` if empty. Anything
/// else (quoted phrases, previous text) gets read back as speech (plan 1, A).
fn prompt(terms: &[String], language: Option<&str>) -> Option<String> {
    let label = if language.is_none_or(|l| l.starts_with("fr")) {
        "Vocabulaire : "
    } else {
        "Vocabulary: "
    };
    // Whole terms only: a cut name ("Kasimir Zbig") would bias wrong.
    let mut list = String::new();
    let mut tokens = 0;
    for t in terms {
        let next = if list.is_empty() {
            t.clone()
        } else {
            format!("{list}, {t}")
        };
        tokens += estimated_tokens(t) + 1;
        if label.chars().count() + next.chars().count() + 1 > PROMPT_MAX_CHARS
            || tokens > PROMPT_MAX_TOKENS
        {
            break;
        }
        list = next;
    }
    (!list.is_empty()).then(|| format!("{label}{list}."))
}

/// shortcut: about 3 chars per token, 2 for code-like words (digits, `_`);
/// count real tokens if prompts still lose their head.
fn estimated_tokens(term: &str) -> usize {
    let per = if term.chars().any(|c| c.is_ascii_digit() || c == '_') {
        2
    } else {
        3
    };
    term.chars().count().div_ceil(per).max(1)
}

/// How much of a transcript was the prompt read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Echo {
    None,
    /// The prompt's head, then speech: the copy was cut off.
    Lead,
    /// Nothing but the prompt.
    All,
}

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// `text` without a copy of `prompt` at its head. A copy counts only when it
/// starts with the label ("Vocabulaire") and runs 3 words or more: without
/// the label, known words may really have been said.
pub fn strip_echo(text: &str, prompt: Option<&str>) -> (String, Echo) {
    let Some(prompt) = prompt else {
        return (text.to_string(), Echo::None);
    };
    if recites(text, prompt) {
        return (String::new(), Echo::All);
    }
    let in_prompt = format!(" {} ", words(prompt).join(" "));
    // Each word of the text with the byte where it ends.
    let mut spans = Vec::new();
    let mut start = None;
    for (i, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ' ')))
    {
        match (c.is_alphanumeric(), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                spans.push((text[s..i].to_lowercase(), i));
                start = None;
            }
            _ => {}
        }
    }
    let mut lead = 0;
    while lead < spans.len() {
        let run: Vec<&str> = spans[..=lead].iter().map(|(w, _)| w.as_str()).collect();
        if !in_prompt.contains(&format!(" {} ", run.join(" "))) {
            break;
        }
        lead += 1;
    }
    let labelled = spans
        .first()
        .is_some_and(|(w, _)| w == "vocabulaire" || w == "vocabulary");
    if lead < 3 || !labelled {
        return (text.to_string(), Echo::None);
    }
    let rest = text[spans[lead - 1].1..]
        .trim_start_matches(|c: char| c.is_whitespace() || ",;:.»«".contains(c));
    if rest.is_empty() {
        (String::new(), Echo::All)
    } else {
        (rest.to_string(), Echo::Lead)
    }
}

/// The prompt read back instead of speech: three words or more, recited in
/// the prompt's order.
fn recites(text: &str, prompt: &str) -> bool {
    // A run of the prompt's words in order: a real dictation often shares
    // the prompt's common words.
    let said = words(text);
    let run = format!(" {} ", said.join(" "));
    let contains = |s: &str| format!(" {} ", words(s).join(" ")).contains(&run);
    // Inside one item (a branch name, a phrase) it's speech: an echo recites
    // several items of the list.
    // A period splits items only between sentences: "fix.lesson.retries" is one.
    let items = prompt.replace(". ", "\n");
    said.len() >= 3
        && contains(prompt)
        && !items
            .trim_end_matches('.')
            .split([',', ';', ':', '«', '»', '\n'])
            .any(contains)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> AsrSources {
        AsrSources {
            custom_words: vec!["Handy".into()],
            learned: vec!["log".into(), "Claude".into()],
            screen_terms: Vec::new(),
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
    fn screen_terms_come_after_custom_words_and_before_learned() {
        let mut s = src();
        s.screen_terms = vec!["Kasimir Zbignowski".into(), "handy".into()];
        let caps = AsrCaps {
            vocabulary: true,
            ..Default::default()
        };
        let c = build(caps, &s, &AsrContextSettings::default());
        assert_eq!(
            c.vocabulary,
            vec!["Handy", "Kasimir Zbignowski", "log", "Claude"]
        );
    }

    #[test]
    fn screen_terms_are_left_out_when_screen_is_off() {
        let mut s = src();
        s.screen_terms = vec!["Kasimir Zbignowski".into()];
        let caps = AsrCaps {
            vocabulary: true,
            ..Default::default()
        };
        let cfg = AsrContextSettings {
            screen: false,
            ..Default::default()
        };
        assert_eq!(
            build(caps, &s, &cfg).vocabulary,
            vec!["Handy", "log", "Claude"]
        );
    }

    #[test]
    fn the_prompt_is_cut_at_a_whole_term() {
        let mut s = src();
        s.learned = (0..200).map(|i| format!("Nom{i} Prenom{i}")).collect();
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
        assert!(p.chars().count() <= PROMPT_MAX_CHARS, "{p}");
        let vocab = p
            .split(". ")
            .next()
            .unwrap()
            .trim_start_matches("Vocabulaire : ");
        for item in vocab.trim_end_matches('.').split(", ").skip(1) {
            assert_eq!(
                item.split(' ').count(),
                2,
                "cut inside a term: {item:?} in {p}"
            );
        }
    }

    #[test]
    fn context_prompt_cites_screen_terms_only_when_screen_is_on() {
        let mut s = src();
        s.screen_terms = vec!["Kasimir".into()];
        let caps = AsrCaps {
            context_prompt: true,
            ..Default::default()
        };
        let off = AsrContextSettings {
            screen: false,
            ..Default::default()
        };
        let AsrPrompt::Context(p) = build(caps, &s, &off).prompt else {
            panic!("context prompt expected")
        };
        assert_eq!(p, "Vocabulaire : Handy, log, Claude.");
        let AsrPrompt::Context(p) = build(caps, &s, &AsrContextSettings::default()).prompt else {
            panic!()
        };
        assert_eq!(p, "Vocabulaire : Handy, Kasimir, log, Claude.");
    }

    #[test]
    fn the_prompt_fits_whispers_token_window() {
        let mut s = src();
        s.learned = (0..200).map(|i| format!("req_{i}_ab12")).collect();
        let cfg = AsrContextSettings {
            max_terms: 200,
            ..Default::default()
        };
        let caps = AsrCaps {
            initial_prompt: true,
            ..Default::default()
        };
        let AsrPrompt::Initial(p) = build(caps, &s, &cfg).prompt else {
            panic!()
        };
        // The user's word stays first, and code-like terms stop early.
        assert!(p.starts_with("Vocabulaire : Handy, req_0_ab12"), "{p}");
        let n = p.matches("req_").count();
        assert!((10..40).contains(&n), "{n} terms: {p}");
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
    fn the_prompt_holds_only_vocabulary() {
        let caps = AsrCaps {
            initial_prompt: true,
            ..Default::default()
        };
        let AsrPrompt::Initial(p) = build(caps, &src(), &AsrContextSettings::default()).prompt
        else {
            panic!()
        };
        assert_eq!(p, "Vocabulaire : Handy, log, Claude.");
        assert!(!p.contains('«'));
    }

    #[test]
    fn an_english_dictation_gets_an_english_label() {
        let mut s = src();
        s.language = Some("en".into());
        let caps = AsrCaps {
            initial_prompt: true,
            ..Default::default()
        };
        let AsrPrompt::Initial(p) = build(caps, &s, &AsrContextSettings::default()).prompt else {
            panic!()
        };
        assert_eq!(p, "Vocabulary: Handy, log, Claude.");
    }

    #[test]
    fn no_term_means_no_prompt() {
        let caps = AsrCaps {
            initial_prompt: true,
            ..Default::default()
        };
        assert_eq!(
            build(caps, &AsrSources::default(), &AsrContextSettings::default()),
            AsrContext::default()
        );
    }

    #[test]
    fn a_term_too_long_alone_makes_no_prompt() {
        let s = AsrSources {
            custom_words: vec!["x".repeat(700)],
            ..Default::default()
        };
        let caps = AsrCaps {
            initial_prompt: true,
            ..Default::default()
        };
        assert_eq!(
            build(caps, &s, &AsrContextSettings::default()).prompt,
            AsrPrompt::None
        );
    }

    #[test]
    fn the_users_name_is_in_the_vocabulary() {
        let custom = vec!["Handy".to_string(), "Gavriel".to_string()];
        assert_eq!(user_words("Gavriel", &custom), vec!["Gavriel", "Handy"]);
        assert_eq!(user_words("  ", &custom), custom);
        assert_eq!(user_words("Ana", &[]), vec!["Ana"]);
    }

    #[test]
    fn a_copied_lead_is_removed_and_the_speech_stays() {
        let p = Some("Vocabulaire : GitHub, Claude, Handy, Groq, Qwen, ADHD, Cloudflare.");
        assert_eq!(
            strip_echo("Vocabulaire : GitHub, Claude. Je suis prêt.", p),
            ("Je suis prêt.".into(), Echo::Lead)
        );
        // Without the label it may be speech: kept whole.
        let said = "Claude, Handy, Groq, Qwen, ADHD, Cloudflare. On commence.";
        assert_eq!(strip_echo(said, p), (said.to_string(), Echo::None));
        assert_eq!(
            strip_echo("Vocabulaire : GitHub, Claude, Handy.", p),
            (String::new(), Echo::All)
        );
        let p = Some("Vocabulary: GitHub, Claude, Handy.");
        assert_eq!(
            strip_echo("Vocabulary: GitHub, Claude, so let's start.", p),
            ("so let's start.".into(), Echo::Lead)
        );
    }

    #[test]
    fn speech_that_opens_with_known_words_is_kept() {
        let p = Some("Vocabulaire : GitHub, Claude, Handy, Groq, Qwen.");
        for t in [
            "Claude, Handy et Groq sont prêts.",
            "Claude, Handy, Groq, Qwen : lequel ?",
            "Vocabulaire à revoir demain.",
            "log Claude",
        ] {
            assert_eq!(strip_echo(t, p), (t.to_string(), Echo::None));
        }
        assert_eq!(
            strip_echo("Vocabulaire : GitHub, Claude.", None).1,
            Echo::None
        );
    }

    #[test]
    fn a_recited_prompt_is_not_a_transcript() {
        let all = |t: &str, p: &str| strip_echo(t, Some(p)).1 == Echo::All;
        let p = "Vocabulaire : log, Claude, Handy. Expressions : « de manière claire ».";
        assert!(all("Vocabulaire : log, Claude, Handy.", p));
        assert!(!all("regarde le log de Claude dans le terminal", p));
        assert!(!all("log Claude", p), "too short to tell");
        // Common words also found in the text before the cursor are speech.
        let p = "Texte : je pense que tu es d'accord, je suis là. Vocabulaire : log.";
        assert!(!all("Je suis d'accord.", p));
        // Saying again a run of one item (a phrase of the text before the
        // cursor, a branch name) is speech: an echo recites several items.
        assert!(!all("je pense que tu es d'accord", p));
        let p = "Vocabulaire : fix/lesson-retries, Claude, Handy.";
        assert!(!all("fix lesson retries", p));
        assert!(all("fix lesson retries Claude Handy", p));
        // A dotted term is one item too.
        let p = "Vocabulaire : fix.lesson.retries, Claude.";
        assert!(!all("fix lesson retries", p));
    }

    #[test]
    fn old_settings_with_expressions_still_load() {
        let s: AsrContextSettings =
            serde_json::from_str(r#"{"expressions":true,"max_terms":12}"#).unwrap();
        assert_eq!(s.max_terms, 12);
        assert!(s.enabled && s.screen);
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
