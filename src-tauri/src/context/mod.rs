//! Handy Adaptive context (F20–F34): what the destination is, what the field
//! around the caret says, and the minimal, filtered part of it the LLM gets.
//! The reading itself is Windows-only (`windows.rs`); everything here is pure.

#[cfg(windows)]
mod windows;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;

/// What the target is for (F22). Independent of `Style` and the terminal flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Content {
    Email,
    WorkChat,
    PersonalChat,
    Ai,
    Document,
    Code,
    Other,
}

/// Style category (F58 picks a style per category at step 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Work,
    Email,
    Personal,
    Other,
}

/// Writing style chosen per style category (F58).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    Formal,
    Casual,
    VeryCasual,
    Excited,
    #[default]
    Default,
}

/// Tone per style category. Defaults (Q8): email formal, chats casual, the rest default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Styles {
    pub work: Tone,
    pub email: Tone,
    pub personal: Tone,
    pub other: Tone,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            work: Tone::Casual,
            email: Tone::Formal,
            personal: Tone::Casual,
            other: Tone::Default,
        }
    }
}

impl Styles {
    pub fn tone(&self, style: Style) -> Tone {
        match style {
            Style::Work => self.work,
            Style::Email => self.email,
            Style::Personal => self.personal,
            Style::Other => self.other,
        }
    }
}

impl Content {
    pub fn style(self) -> Style {
        match self {
            Content::Email => Style::Email,
            Content::WorkChat => Style::Work,
            Content::PersonalChat => Style::Personal,
            _ => Style::Other,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Rule {
    content: Content,
    #[serde(default)]
    processes: Vec<String>,
    #[serde(default)]
    hosts: Vec<String>,
}

/// Classification lists (F22), editable without recompiling: a
/// `context_rules.json` in the data folder replaces the built-in one.
#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    categories: Vec<Rule>,
    #[serde(default)]
    terminals: Vec<String>,
    #[serde(default)]
    browsers: Vec<String>,
}

const DEFAULT_RULES: &str = include_str!("rules.json");

static RULES: Lazy<Mutex<Rules>> = Lazy::new(|| {
    Mutex::new(serde_json::from_str(DEFAULT_RULES).expect("built-in context rules are valid"))
});

/// Loads `context_rules.json` from the data folder when it exists and parses.
pub fn load_rules(app: &tauri::AppHandle) {
    let Ok(path) = crate::portable::app_data_dir(app).map(|d| d.join("context_rules.json")) else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    match serde_json::from_str::<Rules>(&text) {
        Ok(rules) => *RULES.lock().unwrap_or_else(|e| e.into_inner()) = rules,
        Err(e) => log::warn!("Ignoring {}: {e}", path.display()),
    }
}

fn rules() -> Rules {
    RULES.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// Host match: exact or a subdomain (`mail.google.com` matches `google.com`).
fn host_matches(host: &str, pattern: &str) -> bool {
    host == pattern || host.ends_with(&format!(".{pattern}"))
}

/// (content, terminal) for a process (lowercase, no `.exe`) and optional host.
/// The site wins over the browser that shows it.
pub fn classify(process: &str, host: Option<&str>) -> (Content, bool) {
    let rules = rules();
    let terminal = rules.terminals.iter().any(|t| t == process);
    let by_host = host.and_then(|h| {
        rules
            .categories
            .iter()
            .find(|r| r.hosts.iter().any(|p| host_matches(h, p)))
    });
    let by_process = || {
        rules
            .categories
            .iter()
            .find(|r| r.processes.iter().any(|p| p == process))
    };
    let content = by_host
        .or_else(by_process)
        .map_or(Content::Other, |r| r.content);
    (content, terminal)
}

pub fn is_browser(process: &str) -> bool {
    rules().browsers.iter().any(|b| b == process)
}

/// `chrome.exe`, `C:\…\Code.exe` → `chrome`, `code`.
pub fn process_key(path: &str) -> String {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let lower = file.to_lowercase();
    lower.strip_suffix(".exe").unwrap_or(&lower).to_string()
}

/// Address-bar value → host (F21). `None` for anything that isn't a web
/// address: text being typed, `file:///`, internal pages, bare words.
pub fn normalize_host(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.chars().any(char::is_whitespace) {
        return None;
    }
    let rest = match raw.split_once("://") {
        Some((scheme, rest)) => {
            let scheme = scheme.to_ascii_lowercase();
            if scheme != "http" && scheme != "https" {
                return None;
            }
            rest
        }
        None => raw,
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.rsplit('@').next()?.split(':').next()?;
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    let valid = host.contains('.')
        && !host.starts_with('.')
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    (valid || host == "localhost").then_some(host)
}

/// F33: excluded process (exact) or domain (exact or subdomain).
pub fn is_excluded(process: &str, host: Option<&str>, apps: &[String], domains: &[String]) -> bool {
    apps.iter().any(|a| process_key(a.trim()) == process)
        || host.is_some_and(|h| {
            domains
                .iter()
                .map(|d| d.trim().trim_start_matches("*.").to_ascii_lowercase())
                .any(|d| !d.is_empty() && host_matches(h, &d))
        })
}

/// F24: a field that only shows its placeholder (or label) is empty.
pub fn is_placeholder(contents: &str, name: &str, help: &str) -> bool {
    let c = contents.trim();
    !c.is_empty() && (c == name.trim() || c == help.trim())
}

/// What was read from the focused field (F23).
#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct Field {
    pub before: String,
    pub after: String,
    pub selection: String,
    /// The caret position is known (TextPattern); otherwise `before` holds the
    /// whole content and is only general context.
    pub caret_known: bool,
}

/// One reading of the destination at release.
#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct Capture {
    pub process: String,
    pub title: String,
    pub host: Option<String>,
    pub content: Option<Content>,
    pub terminal: bool,
    /// Password or secure field (F32, F79): nothing else is read.
    pub sensitive: bool,
    /// Excluded app or domain (F33): no context, no automatic paste.
    pub excluded: bool,
    pub field: Option<Field>,
    /// Names next to email addresses on the page: recipients, quoted authors (F27).
    pub names: Vec<String>,
    /// Other spellings from the field, page and editor title (F40, F28).
    pub terms: Vec<String>,
    /// Characters of page text read to find `names` (F114); never kept or sent.
    pub page_chars: u32,
}

pub fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Last `max` chars before the caret, cut at a word boundary.
pub fn tail(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let cut: String = chars[chars.len() - max..].iter().collect();
    match cut.find(char::is_whitespace) {
        Some(i) => cut[i..].trim_start().to_string(),
        None => cut,
    }
}

/// First `max` chars after the caret, cut at a word boundary.
pub fn head(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    let cut: String = chars[..max].iter().collect();
    match cut.rfind(char::is_whitespace) {
        Some(i) => cut[..i].trim_end().to_string(),
        None => cut,
    }
}

static EMAIL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+").expect("valid regex"));
static CARD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b\d(?:[ -]?\d){12,18}\b").expect("valid regex"));
static IBAN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]{4}){2,7}(?: ?[A-Z0-9]{1,4})?\b").expect("valid regex")
});
static OTP: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d{6,8}\b").expect("valid regex"));
static TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Za-z0-9_\-]{24,}\b").expect("valid regex"));

/// F31: emails never leave; recognizable secrets are masked. This reduces the
/// risk, it doesn't guarantee detection: minimization stays the protection.
pub fn redact(s: &str) -> String {
    let s = EMAIL.replace_all(s, "[email]");
    let s = IBAN.replace_all(&s, "[iban]");
    let s = CARD.replace_all(&s, "[number]");
    let s = OTP.replace_all(&s, "[code]");
    TOKEN
        .replace_all(&s, |c: &regex::Captures| {
            let t = &c[0];
            let mixed =
                t.chars().any(|c| c.is_ascii_digit()) && t.chars().any(|c| c.is_ascii_alphabetic());
            if mixed {
                "[token]".to_string()
            } else {
                t.to_string()
            }
        })
        .into_owned()
}

/// F27: names written next to an email address — `Name <a@b>`, `Name (a@b)`,
/// `"Name" <a@b>`, `…, Name <a@b> a écrit :`. Up to four capitalized words.
pub fn names_near_emails(texts: &[String]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for text in texts {
        for m in EMAIL.find_iter(text) {
            let before = text[..m.start()].trim_end();
            let Some(before) = before.strip_suffix(['<', '(']) else {
                // Address shown alone: the name comes from its local part.
                if let Some(name) = name_from_address(m.as_str()) {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                continue;
            };
            let before = before.trim_end().trim_end_matches('"').trim_end();
            let words: Vec<&str> = before
                .rsplit(|c: char| c.is_whitespace())
                .map(|w| w.trim_start_matches('"'))
                .take_while(|w| w.chars().next().is_some_and(char::is_uppercase))
                .take(4)
                .collect();
            if words.is_empty() {
                continue;
            }
            let name = words.into_iter().rev().collect::<Vec<_>>().join(" ");
            if !name.contains('@') && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names.truncate(10);
    names
}

/// `adryen.martin@x.com` → `Adryen Martin`; role addresses and handles with
/// digits give nothing.
fn name_from_address(address: &str) -> Option<String> {
    const ROLES: &[&str] = &[
        "contact",
        "info",
        "support",
        "noreply",
        "no",
        "reply",
        "admin",
        "hello",
        "bonjour",
        "service",
        "team",
        "equipe",
        "sales",
        "mail",
        "newsletter",
        "notifications",
        "help",
    ];
    let local = address.split('@').next()?;
    let parts: Vec<&str> = local.split(['.', '_', '-']).collect();
    let ok = parts.len() <= 3
        && parts
            .iter()
            .all(|p| p.chars().count() >= 2 && p.chars().all(char::is_alphabetic))
        && !parts
            .iter()
            .any(|p| ROLES.contains(&p.to_lowercase().as_str()));
    ok.then(|| {
        parts
            .iter()
            .map(|p| {
                let mut c = p.chars();
                c.next().map_or(String::new(), |f| {
                    f.to_uppercase()
                        .chain(c.flat_map(char::to_lowercase))
                        .collect()
                })
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// The part of a capture the LLM may see (F31). `None` = nothing to send.
#[derive(Debug, Clone, Serialize, Type)]
pub struct LlmContext {
    pub content: Content,
    pub style: Style,
    pub tone: Tone,
    pub terminal: bool,
    pub before: String,
    pub after: String,
    pub selection: String,
    pub names: Vec<String>,
    /// Spellings to respect: page terms (F40) and the user's custom words.
    pub terms: Vec<String>,
    /// Handy Adaptive: the app, its window title (redacted) and web host.
    pub app: String,
    pub window: String,
    pub site: Option<String>,
    /// Past dictations like this one, as the user kept them (filled by actions).
    pub past: Vec<String>,
    /// The user's learned style rules (filled by actions).
    pub rules: Vec<String>,
}

/// Longest selection Polish and voice instructions transform (F121).
pub const SELECTION_MAX: usize = 8_000;

const BEFORE_CHARS: usize = 1500;
const AFTER_CHARS: usize = 500;
const WINDOW_CHARS: usize = 200;
const SELECTION_CHARS: usize = 300;

pub fn llm_context(capture: &Capture) -> Option<LlmContext> {
    if capture.sensitive || capture.excluded {
        return None;
    }
    let content = capture.content?;
    let (before, after, selection) = match &capture.field {
        Some(f) if f.caret_known => (
            redact(&tail(&f.before, BEFORE_CHARS)),
            redact(&head(&f.after, AFTER_CHARS)),
            redact(&head(&f.selection, SELECTION_CHARS)),
        ),
        _ => Default::default(),
    };
    Some(LlmContext {
        content,
        style: content.style(),
        tone: Tone::Default,
        terminal: capture.terminal,
        before,
        after,
        selection,
        names: capture.names.clone(),
        terms: capture.terms.clone(),
        app: capture.process.clone(),
        window: redact(&head(&capture.title, WINDOW_CHARS)),
        site: capture.host.clone(),
        past: Vec::new(),
        rules: Vec::new(),
    })
}

/// Words that open a sentence without being a name (fr/en), lowercased.
const STARTERS: &[&str] = &[
    "le",
    "la",
    "les",
    "un",
    "une",
    "des",
    "de",
    "du",
    "au",
    "aux",
    "ce",
    "cet",
    "cette",
    "ces",
    "je",
    "tu",
    "il",
    "elle",
    "on",
    "nous",
    "vous",
    "ils",
    "elles",
    "mon",
    "ma",
    "mes",
    "ton",
    "ta",
    "tes",
    "son",
    "sa",
    "ses",
    "notre",
    "votre",
    "leur",
    "leurs",
    "bonjour",
    "bonsoir",
    "salut",
    "merci",
    "cordialement",
    "bien",
    "bonne",
    "oui",
    "non",
    "mais",
    "et",
    "ou",
    "donc",
    "car",
    "si",
    "quand",
    "comment",
    "pourquoi",
    "voici",
    "voilà",
    "pour",
    "avec",
    "dans",
    "sur",
    "par",
    "en",
    "objet",
    "envoyé",
    "date",
    "hier",
    "demain",
    "aujourd'hui",
    "the",
    "a",
    "an",
    "you",
    "he",
    "she",
    "we",
    "they",
    "it",
    "this",
    "that",
    "these",
    "those",
    "hi",
    "hello",
    "dear",
    "thanks",
    "thank",
    "best",
    "regards",
    "yes",
    "no",
    "but",
    "and",
    "or",
    "so",
    "if",
    "when",
    "how",
    "why",
    "what",
    "subject",
    "sent",
    "from",
    "to",
    "for",
    "with",
    "in",
    "on",
    "at",
    "of",
    "my",
    "your",
    "our",
    "their",
    "please",
    "re",
    "tr",
    "fw",
    "cc",
    "cci",
    "bcc",
];

const CODE_EXTENSIONS: &[&str] = &[
    "py", "rs", "ts", "tsx", "js", "jsx", "json", "md", "go", "java", "kt", "cs", "cpp", "c", "h",
    "rb", "php", "sql", "yml", "yaml", "toml", "sh", "ps1", "html", "css", "vue", "swift",
];

/// A code identifier or file name: `compute_invoice_total`, `InvoiceAggregator`,
/// `billing_service.py`.
fn is_identifier(t: &str) -> bool {
    let letters = t.chars().any(char::is_alphabetic);
    let snake = t.contains('_') && letters && t.chars().all(|c| c.is_alphanumeric() || c == '_');
    let camel = t.chars().all(char::is_alphanumeric)
        && t.chars()
            .zip(t.chars().skip(1))
            .any(|(a, b)| a.is_lowercase() && b.is_uppercase());
    let file = t
        .rsplit_once('.')
        .is_some_and(|(stem, ext)| !stem.is_empty() && CODE_EXTENSIONS.contains(&ext));
    snake || camel || file
}

/// F40: terms of the destination's context — capitalized names (alone or in
/// groups of up to three), acronyms, code identifiers and file names, plus the
/// file and project of an editor's title (F28). Most frequent first, at most 20.
pub fn extract_terms(texts: &[String], title: &str, code: bool, names: &[String]) -> Vec<String> {
    let mut found: Vec<(String, usize)> = Vec::new();
    let mut add = |t: String| match found.iter_mut().find(|(f, _)| *f == t) {
        Some((_, n)) => *n += 1,
        None => found.push((t, 1)),
    };
    let title_parts: Vec<String> = if code {
        title
            .split(" - ")
            .take(2)
            .map(|s| s.trim().trim_start_matches('●').trim().to_string())
            .collect()
    } else {
        Vec::new()
    };
    for part in &title_parts {
        if !part.is_empty() && !part.contains("Visual Studio") {
            add(part.clone());
        }
    }
    for text in texts {
        let raw: Vec<&str> = text.split_whitespace().collect();
        let mut group: Vec<String> = Vec::new();
        let mut sentence_start = true;
        for (i, tok) in raw.iter().enumerate() {
            let word = tok.trim_matches(|c: char| ",;:!?()[]{}«»\"'“”".contains(c));
            let word = word
                .strip_suffix('.')
                .filter(|_| !is_identifier(tok.trim_end_matches([',', ';', ':'])))
                .unwrap_or(word);
            let clean = !word.contains('@') && !word.contains("://") && word.chars().count() >= 2;
            let next_capital = raw
                .get(i + 1)
                .and_then(|n| {
                    n.trim_start_matches(|c: char| !c.is_alphanumeric())
                        .chars()
                        .next()
                })
                .is_some_and(char::is_uppercase);
            let capital = clean && word.chars().next().is_some_and(char::is_uppercase);
            let acronym = capital
                && word.chars().count() <= 6
                && word.chars().all(|c| c.is_uppercase() || c.is_ascii_digit());
            // A capitalized word opening a sentence counts only when another
            // one follows (a full name, a list of names), never a lone button.
            let name_like = capital
                && !acronym
                && !STARTERS.contains(&word.to_lowercase().as_str())
                && (!sentence_start || next_capital || !group.is_empty());
            // Code tokens: `f(x).y` holds several identifiers.
            for piece in tok.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')) {
                let piece = piece.trim_matches('.');
                if piece.chars().count() >= 3 && is_identifier(piece) {
                    add(piece.to_string());
                }
            }
            if acronym && word.chars().count() >= 2 {
                add(word.to_string());
            }
            if name_like && !is_identifier(word) {
                group.push(word.to_string());
            }
            let breaks =
                !name_like || tok.ends_with([',', ';', ':', '.', '!', '?']) || group.len() == 3;
            if breaks && !group.is_empty() {
                add(std::mem::take(&mut group).join(" "));
            }
            sentence_start = tok.ends_with(['.', '!', '?', ':']);
        }
        if !group.is_empty() {
            add(group.join(" "));
        }
    }
    found.retain(|(t, _)| !names.iter().any(|n| n == t || n.split(' ').any(|p| p == t)));
    found.sort_by(|a, b| b.1.cmp(&a.1));
    found.into_iter().map(|(t, _)| t).take(20).collect()
}

/// What started a report: a dictation, a spoken instruction, Polish or a
/// saved transformation of the selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    #[default]
    Dictation,
    Instruction,
    Polish,
    Transform,
}

/// What the speech model was given for one transcription (spec §7).
#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct AsrReport {
    /// The loaded model, as `arch variant`.
    pub model: String,
    pub accepts_vocabulary: bool,
    pub accepts_prompt: bool,
    /// The "context for the speech model" setting.
    pub enabled: bool,
    /// Exactly what went in the vocabulary slot.
    pub vocabulary: Vec<String>,
    /// Exactly what went in the prompt slot (context or Whisper initial prompt).
    pub prompt: Option<String>,
    pub expressions: Vec<String>,
    pub app: Option<String>,
    pub before: Option<String>,
    /// `None`: the screen wasn't waited for; `Some(false)`: it came too late.
    pub screen_waited: Option<bool>,
    /// The model read its context back instead of speech; nothing was kept.
    pub echo_dropped: bool,
    /// The speech API failed and the local model transcribed: why.
    pub cloud_error: Option<String>,
}

/// F114: what one dictation or transformation read and sent. Memory only.
#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct ContextReport {
    pub kind: ReportKind,
    /// Unix seconds.
    pub at: i64,
    pub capture: Option<Capture>,
    pub asr: Option<AsrReport>,
    /// The exact user message sent to the LLM, if any.
    pub sent: Option<String>,
}

/// Reports kept, newest last.
const REPORTS_MAX: usize = 10;

static REPORTS: Lazy<Mutex<std::collections::VecDeque<ContextReport>>> =
    Lazy::new(Default::default);

pub fn set_report(mut report: ContextReport) {
    report.at = chrono::Utc::now().timestamp();
    let mut reports = REPORTS.lock().unwrap_or_else(|e| e.into_inner());
    reports.push_back(report);
    while reports.len() > REPORTS_MAX {
        reports.pop_front();
    }
}

/// Records the exact user message sent for the current dictation (F114).
// shortcut: goes to the newest report; a dictation whose rewrite ends after
// the next one started would land on that one. Pass an id if that happens.
pub fn record_sent(message: &str) {
    if let Some(r) = REPORTS.lock().unwrap_or_else(|e| e.into_inner()).back_mut() {
        r.sent = Some(message.to_string());
    }
}

/// The last dictations and transformations, newest first.
#[tauri::command]
#[specta::specta]
pub fn get_context_reports() -> Vec<ContextReport> {
    REPORTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .rev()
        .cloned()
        .collect()
}

/// Which parts of the context the LLM gets, for the rewrite or for Polish
/// and transformations (Settings → Context).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct LlmContextParts {
    /// The app, window, site and the text around the cursor or selection.
    pub screen: bool,
    pub learned_words: bool,
    pub style_rules: bool,
    pub past_dictations: bool,
}

impl Default for LlmContextParts {
    fn default() -> Self {
        Self {
            screen: true,
            learned_words: true,
            style_rules: true,
            past_dictations: true,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn change_rewrite_context_setting(app: tauri::AppHandle, value: LlmContextParts) {
    let mut settings = crate::settings::get_settings(&app);
    settings.rewrite_context = value;
    crate::settings::write_settings(&app, settings);
}

#[tauri::command]
#[specta::specta]
pub fn change_transform_context_setting(app: tauri::AppHandle, value: LlmContextParts) {
    let mut settings = crate::settings::get_settings(&app);
    settings.transform_context = value;
    crate::settings::write_settings(&app, settings);
}

/// Where the text goes, as History shows it: `process · host`.
pub fn app_label(capture: &Capture) -> String {
    [Some(capture.process.clone()), capture.host.clone()]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Keeps only what `parts` allows of `c`. The category, tone and the user's
/// own name and custom words always stay.
pub fn keep_parts(c: &mut LlmContext, parts: &LlmContextParts) {
    if !parts.screen {
        c.before.clear();
        c.after.clear();
        c.selection.clear();
        c.app.clear();
        c.window.clear();
        c.site = None;
        c.names.clear();
        c.terms.clear();
    }
    if !parts.past_dictations {
        c.past.clear();
    }
    if !parts.style_rules {
        c.rules.clear();
    }
}

/// The LLM context for `text` (a dictation or a selection): what the capture
/// may show, the user's name and words, learned spellings, past dictations
/// and style rules, as `parts` allows. `None` when context sending is off.
pub fn enriched_context(
    app: &tauri::AppHandle,
    settings: &crate::settings::AppSettings,
    capture: &Capture,
    text: &str,
    parts: &LlmContextParts,
) -> Option<LlmContext> {
    if !settings.context_send_enabled {
        return None;
    }
    let mut c = llm_context(capture)?;
    c.tone = settings.styles.tone(c.style);
    keep_parts(&mut c, parts);
    // F63: the user's own name.
    let me = settings.user_name.trim();
    if !me.is_empty() && !c.names.iter().any(|n| n == me) {
        c.names.push(me.to_string());
    }
    let learned = if parts.learned_words {
        crate::learning::terms_for(app, text)
    } else {
        Vec::new()
    };
    for word in settings.custom_words.iter().cloned().chain(learned) {
        if !c.terms.contains(&word) {
            c.terms.push(word);
        }
    }
    if settings.learning_enabled {
        let label = app_label(capture);
        if parts.past_dictations {
            use tauri::Manager;
            let now = chrono::Utc::now().timestamp();
            match app
                .state::<std::sync::Arc<crate::managers::history::HistoryManager>>()
                .past_dictations(now - 30 * 86_400)
            {
                Ok(past) => c.past = crate::recall::similar_dictations(&past, text, &label, now),
                Err(e) => log::warn!("Couldn't read past dictations: {e}"),
            }
        }
        if parts.style_rules {
            c.rules = crate::learning::active_rules(app, &label);
        }
    }
    Some(c)
}

/// Reads the destination now (the focused field at release). Bounded in time
/// (F34): an app that doesn't answer gives a capture without context.
/// `read_context = false` (F97) still checks for a password field.
#[cfg(windows)]
pub fn capture(read_context: bool, apps: &[String], domains: &[String]) -> Capture {
    windows::capture(read_context, apps, domains)
}

/// Last `n` characters of `s`.
pub fn tail_chars(s: &str, n: usize) -> String {
    let count = s.chars().count();
    s.chars().skip(count.saturating_sub(n)).collect()
}

/// F80: keeps the focused field to read it again during the observation.
#[cfg(windows)]
pub fn watch_focused() -> bool {
    windows::watch_focused()
}

/// The watched field's whole text, or `None` once it can't be read.
#[cfg(windows)]
pub fn read_watched() -> Option<String> {
    windows::read_watched()
}

#[cfg(not(windows))]
pub fn watch_focused() -> bool {
    false
}

#[cfg(not(windows))]
pub fn read_watched() -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn capture(_read_context: bool, _apps: &[String], _domains: &[String]) -> Capture {
    Capture::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_turned_off_are_left_out() {
        let mut c = LlmContext {
            content: Content::Email,
            style: Content::Email.style(),
            tone: Tone::Default,
            terminal: false,
            before: "avant".into(),
            after: "après".into(),
            selection: "sel".into(),
            names: vec!["Adryen".into()],
            terms: vec!["Quorvantis".into()],
            app: "OUTLOOK.EXE".into(),
            window: "devis".into(),
            site: Some("outlook.office.com".into()),
            past: vec!["hier".into()],
            rules: vec!["règle".into()],
        };
        let all = c.clone();
        keep_parts(&mut c, &LlmContextParts::default());
        assert_eq!(format!("{c:?}"), format!("{all:?}"));
        keep_parts(
            &mut c,
            &LlmContextParts {
                screen: false,
                learned_words: true,
                style_rules: false,
                past_dictations: false,
            },
        );
        assert!(c.before.is_empty() && c.after.is_empty() && c.selection.is_empty());
        assert!(c.app.is_empty() && c.window.is_empty() && c.site.is_none());
        assert!(c.names.is_empty() && c.terms.is_empty());
        assert!(c.past.is_empty() && c.rules.is_empty());
        assert_eq!(c.content, Content::Email);
    }

    #[test]
    fn the_last_ten_reports_are_kept_newest_first() {
        for i in 0..12 {
            set_report(ContextReport {
                sent: Some(format!("m{i}")),
                ..Default::default()
            });
        }
        // Other tests may record a send on the newest report meanwhile.
        let reports = get_context_reports();
        assert_eq!(reports.len(), REPORTS_MAX);
        assert_eq!(reports[1].sent.as_deref(), Some("m10"));
        assert_eq!(reports[9].sent.as_deref(), Some("m2"));
        assert!(reports[0].at > 0);
    }

    #[test]
    fn classifies_by_site_then_process() {
        assert_eq!(
            classify("chrome", Some("mail.google.com")),
            (Content::Email, false)
        );
        assert_eq!(
            classify("msedge", Some("chatgpt.com")),
            (Content::Ai, false)
        );
        assert_eq!(
            classify("chrome", Some("example.com")),
            (Content::Other, false)
        );
        assert_eq!(classify("chrome", None), (Content::Other, false));
        assert_eq!(classify("olk", None), (Content::Email, false));
        assert_eq!(classify("outlook", None), (Content::Email, false));
        assert_eq!(classify("slack", None), (Content::WorkChat, false));
        assert_eq!(classify("whatsapp", None), (Content::PersonalChat, false));
        assert_eq!(classify("code", None), (Content::Code, false));
        assert_eq!(classify("winword", None), (Content::Document, false));
        assert_eq!(classify("windowsterminal", None), (Content::Other, true));
        // Exact process names: `codex` is not VS Code.
        assert_eq!(classify("codex", None), (Content::Other, false));
        assert_eq!(Content::Email.style(), Style::Email);
        assert_eq!(Content::Ai.style(), Style::Other);
    }

    #[test]
    fn normalizes_hosts_and_rejects_non_addresses() {
        assert_eq!(
            normalize_host("https://mail.google.com/mail/u/0/#inbox").as_deref(),
            Some("mail.google.com")
        );
        assert_eq!(
            normalize_host("www.Example.COM.").as_deref(),
            Some("example.com")
        );
        assert_eq!(
            normalize_host("chatgpt.com/c/123").as_deref(),
            Some("chatgpt.com")
        );
        assert_eq!(
            normalize_host("http://user@host.example.org:8080/x").as_deref(),
            Some("host.example.org")
        );
        for bad in [
            "",
            "quelle heure est-il",
            "file:///C:/x.html",
            "chrome://settings",
            "c",
            "C:",
            "about:blank",
            "edge://newtab",
        ] {
            assert_eq!(normalize_host(bad), None, "{bad}");
        }
    }

    #[test]
    fn exclusions_match_exact_process_and_subdomains() {
        let apps = vec!["KeePassXC.exe".to_string(), "1password".to_string()];
        let domains = vec!["boursorama.com".to_string()];
        assert!(is_excluded("keepassxc", None, &apps, &domains));
        assert!(is_excluded("1password", None, &apps, &domains));
        assert!(!is_excluded("keepass", None, &apps, &domains));
        assert!(is_excluded(
            "chrome",
            Some("clients.boursorama.com"),
            &apps,
            &domains
        ));
        assert!(!is_excluded(
            "chrome",
            Some("notboursorama.com"),
            &apps,
            &domains
        ));
    }

    #[test]
    fn placeholder_is_empty() {
        assert!(is_placeholder(
            "Écrivez votre message…",
            "Écrivez votre message…",
            ""
        ));
        assert!(is_placeholder(
            "Reply to Claude...",
            "",
            "Reply to Claude..."
        ));
        assert!(!is_placeholder("Bonjour", "Message", ""));
        assert!(!is_placeholder("", "Message", ""));
    }

    #[test]
    fn excerpts_cut_on_words() {
        assert_eq!(tail("un deux trois", 9), "trois");
        assert_eq!(head("un deux trois", 9), "un deux");
        assert_eq!(tail("court", 300), "court");
        assert_eq!(tail("ééééé ààà", 4), "ààà");
    }

    #[test]
    fn redacts_emails_and_secrets() {
        let s = redact(&"Écris à adryen.martin@example.com, carte 4111 1111 1111 1111, IBAN FR76 3000 6000 0112 3456 7890 189, code 482913, clé sk_live_<KEY>, réunion à 15 h en 2026.".replace("<KEY>", "abcdefghijklmnop1234567890"));
        assert!(!s.contains('@'), "{s}");
        assert!(!s.contains("4111"), "{s}");
        assert!(!s.contains("FR76"), "{s}");
        assert!(!s.contains("482913"), "{s}");
        assert!(!s.contains("sk_live"), "{s}");
        assert!(s.contains("15 h en 2026"), "{s}");
        assert_eq!(
            redact("compute_invoice_total_for_customers"),
            "compute_invoice_total_for_customers"
        );
    }

    #[test]
    fn finds_names_next_to_emails() {
        let texts = vec![
            "Adryen Martin (adryen.martin@example.com)".to_string(),
            "Le lun. 3 nov. 2026 à 10:00, Ludivine Kerbrat <ludivine@example.com> a écrit :"
                .to_string(),
            "\"Kasimir Zbignowski\" <kasimir.z@example.com>".to_string(),
            "écrire à contact@example.com".to_string(),
            "Adryen Martin <adryen.martin@example.com>".to_string(),
            "À : quennehen.aurelie@example.com, jdupont42@example.com".to_string(),
        ];
        assert_eq!(
            names_near_emails(&texts),
            vec![
                "Adryen Martin",
                "Ludivine Kerbrat",
                "Kasimir Zbignowski",
                "Quennehen Aurelie"
            ]
        );
    }

    #[test]
    fn extracts_terms_from_context() {
        let t = |texts: &[&str], title: &str, code: bool| {
            extract_terms(
                &texts.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                title,
                code,
                &["Bérénice Kowalczyk".to_string()],
            )
        };
        // GE05
        let terms = t(&["Zorblax, Quintavex, Merindor"], "", false);
        for w in ["Zorblax", "Quintavex", "Merindor"] {
            assert!(terms.contains(&w.to_string()), "{w} in {terms:?}");
        }
        let terms = t(
            &[
                "À : Bérénice Kowalczyk <berenice.k@example.com> — Objet : Réunion Kubernetes. Bonjour, peux-tu relire le plan de migration PostgreSQL avant jeudi ? Merci, Ksenia Wróblewska.",
                "Cc : Kasimir Zbignowski, Aurélie Quennehen — équipe Vélizy.",
                "Répondre", "Transférer", "Envoyer",
            ],
            "",
            false,
        );
        for w in [
            "PostgreSQL",
            "Ksenia Wróblewska",
            "Kasimir Zbignowski",
            "Aurélie Quennehen",
            "Vélizy",
        ] {
            assert!(terms.contains(&w.to_string()), "{w} in {terms:?}");
        }
        for w in [
            "Bonjour",
            "Merci",
            "Bérénice Kowalczyk",
            "Bérénice",
            "Envoyer",
            "Répondre",
        ] {
            assert!(!terms.contains(&w.to_string()), "{w} not in {terms:?}");
        }
        // GE15 / F28: editor title and identifiers.
        let terms = t(
            &["def compute_invoice_total(items): return InvoiceAggregator(items).total"],
            "● billing_service.py - invoicing - Visual Studio Code",
            true,
        );
        for w in [
            "compute_invoice_total",
            "InvoiceAggregator",
            "billing_service.py",
            "invoicing",
        ] {
            assert!(terms.contains(&w.to_string()), "{w} in {terms:?}");
        }
        assert!(t(&["voir https://example.com/Doc et a@b.com"], "", false).is_empty());
    }

    #[test]
    fn sensitive_or_excluded_sends_nothing() {
        let field = Field {
            before: "Bonjour, écris à a@b.com".into(),
            after: String::new(),
            selection: String::new(),
            caret_known: true,
        };
        let mut c = Capture {
            content: Some(Content::Email),
            field: Some(field),
            names: vec!["Adryen Martin".into()],
            ..Default::default()
        };
        let ctx = llm_context(&c).expect("context");
        assert_eq!(ctx.before, "Bonjour, écris à [email]");
        assert_eq!(ctx.style, Style::Email);
        c.sensitive = true;
        assert!(llm_context(&c).is_none());
        c.sensitive = false;
        c.excluded = true;
        assert!(llm_context(&c).is_none());
        // Caret unknown: content is general context only, not sent as an excerpt.
        c.excluded = false;
        c.field.as_mut().unwrap().caret_known = false;
        assert_eq!(llm_context(&c).unwrap().before, "");
        // The app and window go along, the window title redacted like the field.
        c.process = "OUTLOOK.EXE".into();
        c.title = "Re: devis - jean@exemple.fr - Outlook".into();
        c.host = Some("outlook.office.com".into());
        let ctx = llm_context(&c).unwrap();
        assert_eq!(ctx.app, "OUTLOOK.EXE");
        assert!(!ctx.window.contains("jean@exemple.fr"), "{}", ctx.window);
        assert!(ctx.window.contains("devis"));
        assert_eq!(ctx.site.as_deref(), Some("outlook.office.com"));
    }
}
