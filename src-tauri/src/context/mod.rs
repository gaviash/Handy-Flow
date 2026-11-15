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
    pub terminal: bool,
    pub before: String,
    pub after: String,
    pub selection: String,
    pub names: Vec<String>,
}

const BEFORE_CHARS: usize = 300;
const AFTER_CHARS: usize = 150;
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
        terminal: capture.terminal,
        before,
        after,
        selection,
        names: capture.names.clone(),
    })
}

/// F114: what the last dictation read, kept and sent. Memory only.
#[derive(Debug, Clone, Default, Serialize, Type)]
pub struct ContextReport {
    pub capture: Option<Capture>,
    /// The exact user message sent to the LLM, if any.
    pub sent: Option<String>,
}

static LAST_REPORT: Lazy<Mutex<ContextReport>> = Lazy::new(Default::default);

pub fn set_report(report: ContextReport) {
    *LAST_REPORT.lock().unwrap_or_else(|e| e.into_inner()) = report;
}

/// Records the exact user message sent for the current dictation (F114).
pub fn record_sent(message: &str) {
    LAST_REPORT.lock().unwrap_or_else(|e| e.into_inner()).sent = Some(message.to_string());
}

#[tauri::command]
#[specta::specta]
pub fn get_last_context_report() -> ContextReport {
    LAST_REPORT
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// Reads the destination now (the focused field at release). Bounded in time
/// (F34): an app that doesn't answer gives a capture without context.
/// `read_context = false` (F97) still checks for a password field.
#[cfg(windows)]
pub fn capture(read_context: bool, apps: &[String], domains: &[String]) -> Capture {
    windows::capture(read_context, apps, domains)
}

#[cfg(not(windows))]
pub fn capture(_read_context: bool, _apps: &[String], _domains: &[String]) -> Capture {
    Capture::default()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    }
}
