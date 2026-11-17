//! Handy Adaptive learning (F80–F98, simple version): after a paste, watch
//! the inserted region for a while, compare what the user kept with what was
//! inserted, and remember corrected names and terms. They go back to the
//! rewrite as spellings to respect. No keystroke is ever recorded: only the
//! field is read, and only the learned terms are stored, encrypted (D11).

use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// Per-word alignment tag (PAR §8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Same,
    Case,
    Sub,
    Ins,
    Del,
}

/// Comparison key: letters and digits, lowercased.
pub(crate) fn key(w: &str) -> String {
    let k: String = w
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    if k.is_empty() {
        w.to_string()
    } else {
        k
    }
}

fn letters(w: &str) -> String {
    w.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Character edit distance divided by the longer length (0..=1).
pub fn distance(a: &str, b: &str) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=b.len() {
            let cur = row[j];
            row[j] = (prev + usize::from(a[i - 1] != b[j - 1]))
                .min(row[j] + 1)
                .min(row[j - 1] + 1);
            prev = cur;
        }
    }
    row[b.len()] as f64 / a.len().max(b.len()) as f64
}

/// Word-level alignment of `inserted` → `final`: Levenshtein where a
/// substitution costs the normalized character distance, so "Jon" aligns
/// with "John" instead of a deletion plus an insertion.
pub fn align(inserted: &[&str], fin: &[&str]) -> Vec<(Tag, Option<usize>, Option<usize>)> {
    let (n, m) = (inserted.len(), fin.len());
    let ki: Vec<String> = inserted.iter().map(|w| key(w)).collect();
    let kf: Vec<String> = fin.iter().map(|w| key(w)).collect();
    let sub = |i: usize, j: usize| {
        if ki[i] == kf[j] {
            0.0
        } else {
            distance(&ki[i], &kf[j])
        }
    };
    let mut d = vec![vec![0.0f64; m + 1]; n + 1];
    for i in 0..=n {
        d[i][0] = i as f64;
    }
    for j in 0..=m {
        d[0][j] = j as f64;
    }
    for i in 1..=n {
        for j in 1..=m {
            d[i][j] = (d[i - 1][j - 1] + sub(i - 1, j - 1))
                .min(d[i - 1][j] + 1.0)
                .min(d[i][j - 1] + 1.0);
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && (d[i][j] - (d[i - 1][j - 1] + sub(i - 1, j - 1))).abs() < 1e-9 {
            let tag = if inserted[i - 1] == fin[j - 1]
                || ki[i - 1] == kf[j - 1] && letters(inserted[i - 1]) == letters(fin[j - 1])
            {
                Tag::Same
            } else if ki[i - 1] == kf[j - 1] {
                Tag::Case
            } else {
                Tag::Sub
            };
            out.push((tag, Some(i - 1), Some(j - 1)));
            i -= 1;
            j -= 1;
        } else if i > 0 && (j == 0 || (d[i][j] - (d[i - 1][j] + 1.0)).abs() < 1e-9) {
            out.push((Tag::Del, Some(i - 1), None));
            i -= 1;
        } else {
            out.push((Tag::Ins, None, Some(j - 1)));
            j -= 1;
        }
    }
    out.reverse();
    out
}

/// A learned spelling: `term` is what the user wrote, `heard` every form that
/// was inserted for it ("Quen", "Quinn" → "Qwen"), most recent last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Learned {
    pub term: String,
    #[serde(deserialize_with = "one_or_many")]
    pub heard: Vec<String>,
    pub count: u32,
    /// Unix seconds.
    pub last_seen: i64,
    #[serde(default)]
    pub kind: WordKind,
    /// A pending word (seen once, needs a second sighting) is not used yet.
    #[serde(default = "yes")]
    pub active: bool,
}

fn yes() -> bool {
    true
}

/// A name or technical term, or a common word the recognizer mishears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum WordKind {
    #[default]
    Term,
    Misheard,
}

/// A phrasing preference drawn from the user's corrections by the judge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct StyleRule {
    pub rule: String,
    pub count: u32,
    pub active: bool,
    /// Unix seconds.
    pub last_seen: i64,
    /// Apps the rule holds in; empty = everywhere.
    #[serde(default)]
    pub apps: Vec<String>,
}

/// Everything learned, as stored (encrypted) in `learned.bin`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    pub words: Vec<Learned>,
    #[serde(default)]
    pub rules: Vec<StyleRule>,
}

pub const WORDS_MAX: usize = 2000;
pub const RULES_MAX: usize = 100;
const PENDING_DAYS: i64 = 30;

/// Reads the memory, including the older format: a bare array of words.
fn parse_memory(json: &[u8]) -> Memory {
    serde_json::from_slice::<Memory>(json)
        .or_else(|_| {
            serde_json::from_slice::<Vec<Learned>>(json).map(|words| Memory {
                words,
                rules: Vec::new(),
            })
        })
        .unwrap_or_default()
}

/// Drops words neither corrected nor dictated for 30 days, pending style rules
/// unseen as long, then the least used and oldest entries over the caps.
pub(crate) fn prune(m: &mut Memory, now: i64) {
    let expired = now - PENDING_DAYS * 86_400;
    // Grammar fixes learned before 0.2.6 as misheard words leave.
    for w in m.words.iter_mut().filter(|w| w.kind == WordKind::Misheard) {
        let term = w.term.clone();
        w.heard.retain(|h| !is_grammar_change(h, &term));
    }
    m.words
        .retain(|w| w.last_seen >= expired && !w.heard.is_empty());
    m.rules.retain(|r| r.active || r.last_seen >= expired);
    if m.words.len() > WORDS_MAX {
        m.words
            .sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
        m.words.truncate(WORDS_MAX);
    }
    if m.rules.len() > RULES_MAX {
        m.rules
            .sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
        m.rules.truncate(RULES_MAX);
    }
}

/// Forms heard kept per learned word.
const HEARD_MAX: usize = 10;

/// Memory saved before several heard forms held a single string.
fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Heard {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Heard::deserialize(d)? {
        Heard::One(s) => vec![s],
        Heard::Many(v) => v,
    })
}

/// Adds a heard form to a learned word; `true` if it's a new one.
fn add_heard(l: &mut Learned, heard: String) -> bool {
    if l.heard.iter().any(|h| h.eq_ignore_ascii_case(&heard)) {
        return false;
    }
    l.heard.push(heard);
    if l.heard.len() > HEARD_MAX {
        l.heard.remove(0);
    }
    true
}

/// Corrections worth learning (F85–F87): runs of 1 to 4 substituted words
/// between unchanged words, whose corrected form looks like a name or a term
/// (capital, digit or `_`), stays close to what was inserted (≤ 0.6) and is
/// not a truncation. Case-only, meaning changes, common-word fixes and
/// rewrites give nothing. At most 4 per dictation.
/// Also misheard common words (1 to 3 words, close spelling, no meaning word),
/// which stay pending until seen twice (see [`record_word`]).
/// Each rejected run is logged with a reason, never with its text.
pub fn candidates(inserted: &str, fin: &str) -> Vec<Candidate> {
    let reject = |reason: &str| log::debug!("Learning: rejected ({reason})");
    let a: Vec<&str> = inserted.split_whitespace().collect();
    let b: Vec<&str> = fin.split_whitespace().collect();
    let tags = align(&a, &b);
    let edited = tags
        .iter()
        .filter(|t| t.0 != Tag::Same && t.0 != Tag::Case)
        .count();
    // A rewrite, not a correction.
    if edited * 2 > a.len().max(b.len()) {
        reject("rewrite");
        return Vec::new();
    }
    let trim = |w: &str| {
        w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .to_string()
    };
    let mut out = Vec::new();
    let mut far = Vec::new();
    let mut k = 0;
    let changed = |t: &(Tag, Option<usize>, Option<usize>)| !matches!(t.0, Tag::Same | Tag::Case);
    // Punctuation after a word ends a term: "Quen, GEMA" → "Qwen, gemma" is two.
    let ends_item = |t: &(Tag, Option<usize>, Option<usize>)| {
        let w = t.2.map(|j| b[j]).or(t.1.map(|i| a[i])).unwrap_or("");
        w.ends_with([',', ';', ':', '.', '!', '?'])
    };
    while k < tags.len() {
        if !changed(&tags[k]) {
            k += 1;
            continue;
        }
        let start = k;
        while k < tags.len() && changed(&tags[k]) {
            k += 1;
            if ends_item(&tags[k - 1]) {
                break;
            }
        }
        let run = &tags[start..k];
        // A split or merge ("GPTOC" → "GPT OSS") still holds a substitution.
        if !run.iter().any(|t| t.0 == Tag::Sub) {
            reject("no_substitution");
            continue;
        }
        let bounded = |t: Option<&(Tag, Option<usize>, Option<usize>)>| {
            t.is_none_or(|t| matches!(t.0, Tag::Same | Tag::Case))
        };
        let after_item = start.checked_sub(1).is_some_and(|s| ends_item(&tags[s]));
        if run.len() > 4 {
            reject("too_long");
            continue;
        }
        if !(after_item || bounded(start.checked_sub(1).and_then(|s| tags.get(s))))
            || !(ends_item(&tags[k - 1]) || bounded(tags.get(k)))
        {
            reject("unbounded");
            continue;
        }
        let heard = run
            .iter()
            .filter_map(|t| t.1)
            .map(|i| trim(a[i]))
            .collect::<Vec<_>>()
            .join(" ");
        let term = run
            .iter()
            .filter_map(|t| t.2)
            .map(|j| trim(b[j]))
            .collect::<Vec<_>>()
            .join(" ");
        let (hk, tk) = (heard.to_lowercase(), term.to_lowercase());
        // A capital only marks a name when it doesn't just open a sentence.
        let opens = |first: Option<usize>, words: &[&str]| {
            first.is_none_or(|j| j == 0 || words[j - 1].ends_with(['.', '!', '?']))
        };
        let capital = |s: &str, opens: bool| {
            s.split(' ').enumerate().any(|(n, w)| {
                w.chars()
                    .enumerate()
                    .any(|(i, c)| c.is_uppercase() && (i > 0 || n > 0 || !opens))
            })
        };
        let term_opens = opens(run.iter().find_map(|t| t.2), &b);
        let heard_opens = opens(run.iter().find_map(|t| t.1), &a);
        // A name the ASR capitalized mid-sentence stays one when the user
        // writes it in lowercase ("Coen" → "qwen").
        let looks_like_term = capital(&term, term_opens)
            || capital(&heard, heard_opens)
            || term.chars().any(|c| c.is_ascii_digit() || c == '_');
        // Truncation, plural, conjugation: one form extends the other.
        let affix =
            hk.starts_with(&tk) || tk.starts_with(&hk) || hk.ends_with(&tk) || tk.ends_with(&hk);
        let d = distance(&hk, &tk);
        let kind = if term.is_empty() {
            Err("no_substitution")
        } else if affix {
            Err("affix")
        } else if looks_like_term {
            if d <= 0.6 {
                Ok(WordKind::Term)
            } else {
                Err("too_far")
            }
        } else if run.len() > 3 {
            Err("too_long")
        } else if has_meaning_word(&hk) || has_meaning_word(&tk) {
            Err("meaning_word")
        } else if is_grammar_change(&hk, &tk) {
            Err("grammar")
        } else if d <= MISHEARD_DISTANCE {
            Ok(WordKind::Misheard)
        } else {
            Err("too_far")
        };
        match kind {
            Ok(kind) => out.push(Candidate { term, heard, kind }),
            // Too far alone, but kept in case the same word is fixed twice here.
            Err("too_far") if d <= LOOSE_DISTANCE => far.push(Candidate {
                term,
                heard,
                kind: if looks_like_term {
                    WordKind::Term
                } else {
                    WordKind::Misheard
                },
            }),
            Err(reason) => reject(reason),
        }
    }
    // F-same-dictation: a word fixed twice in one dictation ("chef", "jeff" →
    // "jev") is learned even when one form was too far on its own.
    let same = |t: &str| {
        out.iter()
            .chain(&far)
            .filter(|c| c.term.to_lowercase() == t.to_lowercase())
            .count()
    };
    let repeated: Vec<Candidate> = far.iter().filter(|c| same(&c.term) >= 2).cloned().collect();
    for c in &far {
        if same(&c.term) < 2 {
            reject("too_far");
        }
    }
    out.extend(repeated);
    out.truncate(4);
    out
}

/// A correction the layer may learn (F85–F87).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub term: String,
    pub heard: String,
    pub kind: WordKind,
}

/// "gag" / "log" = 0.67: common words get more slack than names, and the
/// meaning-word list plus two sightings guard against meaning changes.
const MISHEARD_DISTANCE: f64 = 0.7;

/// "chef" / "jev" = 0.75: allowed only for a word fixed twice in one
/// dictation, or one the judge calls technical.
const LOOSE_DISTANCE: f64 = 0.8;

/// Words whose swap changes the meaning ("mardi" → "jeudi", "oui" → "non"):
/// never learned as misheard, however close they sound.
const MEANING_WORDS: &[&str] = &[
    // numbers
    "zéro",
    "zero",
    "un",
    "une",
    "deux",
    "trois",
    "quatre",
    "cinq",
    "six",
    "sept",
    "huit",
    "neuf",
    "dix",
    "onze",
    "douze",
    "treize",
    "quatorze",
    "quinze",
    "seize",
    "vingt",
    "trente",
    "cent",
    "mille",
    "one",
    "two",
    "three",
    "four",
    "five",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "twenty",
    "hundred",
    "thousand",
    // yes / no / negation
    "oui",
    "non",
    "si",
    "yes",
    "no",
    "ne",
    "pas",
    "jamais",
    "rien",
    "plus",
    "not",
    "never",
    // personal pronouns
    "je",
    "tu",
    "il",
    "elle",
    "on",
    "nous",
    "vous",
    "ils",
    "elles",
    "moi",
    "toi",
    "lui",
    "eux",
    "i",
    "you",
    "he",
    "she",
    "we",
    "they",
    "me",
    "him",
    "her",
    "us",
    "them",
    // English days and months (French ones come from adaptive)
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// Pairs a user fixes for grammar, not because the recognizer misheard.
const GRAMMAR_PAIRS: &[(&str, &str)] = &[
    ("their", "they're"),
    ("there", "their"),
    ("there", "they're"),
    ("then", "than"),
    ("your", "you're"),
    ("were", "where"),
    ("quand", "quant"),
    ("peut", "peux"),
    ("voie", "voix"),
    ("près", "prêt"),
    ("leur", "leurs"),
    ("quel", "quelle"),
];

/// French verb endings, longest first: "envoyer" and "envoyé" share "envoy".
const VERB_ENDINGS: &[&str] = &[
    "aient", "ées", "ais", "ait", "ant", "ée", "és", "er", "ez", "ai", "é",
];

/// A grammar fix, never a misheard word: a word under 4 letters (et/est,
/// ces/ses, le/la), the same verb with another ending, or a known pair.
fn is_grammar_change(heard: &str, term: &str) -> bool {
    let (h, t) = (heard.to_lowercase(), term.to_lowercase());
    let stem = |w: &str| {
        VERB_ENDINGS
            .iter()
            .find_map(|e| w.strip_suffix(e).filter(|s| !s.is_empty()))
            .unwrap_or(w)
            .to_string()
    };
    letters(&h).chars().count() < 4
        || letters(&t).chars().count() < 4
        || stem(&h) == stem(&t)
        || GRAMMAR_PAIRS
            .iter()
            .any(|&(a, b)| (h == a && t == b) || (h == b && t == a))
}

fn has_meaning_word(s: &str) -> bool {
    s.split_whitespace()
        .any(|w| MEANING_WORDS.contains(&w) || crate::adaptive::FR_COMMON_NAMES.contains(&w))
}

/// Whether `heard` → `term` can be a misheard common word (also checked on
/// what the LLM judge proposes).
pub fn is_misheard_pair(heard: &str, term: &str) -> bool {
    close_pair(heard, term, MISHEARD_DISTANCE)
}

fn close_pair(heard: &str, term: &str, max: f64) -> bool {
    let (h, t) = (heard.to_lowercase(), term.to_lowercase());
    !h.is_empty()
        && !t.is_empty()
        && h != t
        && !has_meaning_word(&h)
        && !has_meaning_word(&t)
        && !is_grammar_change(&h, &t)
        && distance(&h, &t) <= max
}

/// Whether `text` holds `word` as a whole word; both already lowercased.
pub(crate) fn contains_word(text: &str, word: &str) -> bool {
    !word.is_empty()
        && text.match_indices(word).any(|(i, _)| {
            let before = text[..i].chars().next_back();
            let after = text[i + word.len()..].chars().next();
            !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
        })
}

/// Restarts the 30 days of every active word the user said: its term or one
/// of its heard forms, as a whole word in `spoken` (the raw transcription).
/// Pending words still need a second correction. `true` when one was found.
pub(crate) fn mark_used(m: &mut Memory, spoken: &str, now: i64) -> bool {
    let text = unify(spoken).to_lowercase();
    let mut found = false;
    for w in m.words.iter_mut().filter(|w| w.active) {
        let said = std::iter::once(&w.term)
            .chain(&w.heard)
            .any(|f| contains_word(&text, &unify(f).to_lowercase()));
        if said {
            w.last_seen = now;
            found = true;
        }
    }
    found
}

/// After a dictation: keeps the learned words the user said for 30 more
/// days. `spoken` is the raw transcription, never the rewrite: a word the
/// rewrite writes on its own must not keep itself alive.
pub fn touch_used(app: &AppHandle, spoken: &str) {
    with_memory(app, |m| {
        if mark_used(m, spoken, chrono::Utc::now().timestamp()) {
            save(app, m);
        }
    });
}

fn same_term(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// What just became usable, as announced to the user and undone exactly by
/// the notification's Undo: a new word, a new heard form of a known word, or
/// a style rule.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LearnedNews {
    Word { term: String },
    Form { term: String, heard: String },
    Rule { rule: String },
}

impl LearnedNews {
    /// The text shown on the pill and in the notification.
    pub fn label(&self) -> &str {
        match self {
            Self::Word { term } | Self::Form { term, .. } => term,
            Self::Rule { rule } => rule,
        }
    }
}

/// Records a sighting; the news when the word becomes usable or a usable
/// word gains a form. A misheard word is pending until seen twice.
pub(crate) fn record_word(m: &mut Memory, c: &Candidate, now: i64) -> Option<LearnedNews> {
    match m.words.iter_mut().find(|l| same_term(&l.term, &c.term)) {
        Some(l) => {
            l.count += 1;
            l.last_seen = now;
            let new_form = add_heard(l, c.heard.clone());
            let activated = !l.active && l.count >= 2;
            l.active |= activated;
            if activated {
                Some(LearnedNews::Word {
                    term: l.term.clone(),
                })
            } else {
                (l.active && new_form).then(|| LearnedNews::Form {
                    term: l.term.clone(),
                    heard: c.heard.clone(),
                })
            }
        }
        None => {
            let active = c.kind == WordKind::Term;
            m.words.push(Learned {
                term: c.term.clone(),
                heard: vec![c.heard.clone()],
                count: 1,
                last_seen: now,
                kind: c.kind,
                active,
            });
            active.then(|| LearnedNews::Word {
                term: c.term.clone(),
            })
        }
    }
}

/// Undoes one heard form; a word left with none goes.
pub(crate) fn drop_form(m: &mut Memory, term: &str, heard: &str) {
    for w in m.words.iter_mut().filter(|w| same_term(&w.term, term)) {
        w.heard.retain(|h| !same_term(h, heard));
    }
    m.words.retain(|w| !w.heard.is_empty());
}

// ---------------------------------------------------------------- memory

static MEMORY: Mutex<Option<Memory>> = Mutex::new(None);

fn memory_path(app: &AppHandle) -> Option<PathBuf> {
    crate::portable::app_data_dir(app)
        .ok()
        .map(|d| d.join("learned.bin"))
}

#[cfg(windows)]
mod dpapi {
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };

    /// Encrypts or decrypts with the Windows account's key (D11).
    pub fn run(data: &[u8], protect: bool) -> Option<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB::default();
        // SAFETY: the input outlives the call; the output is copied then freed.
        unsafe {
            let ok = if protect {
                CryptProtectData(&input, None, None, None, None, 0, &mut out)
            } else {
                CryptUnprotectData(&input, None, None, None, None, 0, &mut out)
            };
            ok.ok()?;
            let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out.pbData as *mut _)));
            Some(bytes)
        }
    }
}

#[cfg(not(windows))]
mod dpapi {
    // shortcut: no at-rest encryption outside Windows (fork scope is Windows).
    pub fn run(data: &[u8], _protect: bool) -> Option<Vec<u8>> {
        Some(data.to_vec())
    }
}

fn with_memory<T>(app: &AppHandle, f: impl FnOnce(&mut Memory) -> T) -> T {
    let mut guard = MEMORY.lock().unwrap_or_else(|e| e.into_inner());
    let entries = guard.get_or_insert_with(|| {
        memory_path(app)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| dpapi::run(&b, false))
            .map(|b| parse_memory(&b))
            .unwrap_or_default()
    });
    // Expired words leave even when nothing is saved for a while.
    prune(entries, chrono::Utc::now().timestamp());
    f(entries)
}

fn save(app: &AppHandle, m: &mut Memory) {
    let (Some(path), Ok(json)) = (memory_path(app), {
        prune(m, chrono::Utc::now().timestamp());
        serde_json::to_vec(&*m)
    }) else {
        return;
    };
    let Some(bytes) = dpapi::run(&json, true) else {
        log::warn!("Couldn't encrypt the learned words; not saved");
        return;
    };
    // Written aside then renamed, so a crash never leaves half a file (NF-12).
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .is_err()
    {
        log::warn!("Couldn't save the learned words to {}", path.display());
    }
}

/// Learned spellings for the rewrite of `transcription`: those it sounds
/// like first, then the most used (≤ 40).
pub fn terms_for(app: &AppHandle, transcription: &str) -> Vec<String> {
    with_memory(app, |m| {
        crate::recall::relevant_terms(&m.words, transcription, 40)
    })
}

/// Learned terms alone (no heard forms), most used first: what the ASR gets.
pub fn plain_terms(app: &AppHandle) -> Vec<String> {
    with_memory(app, |m| {
        let mut words: Vec<&Learned> = m.words.iter().filter(|w| w.active).collect();
        words.sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
        words.into_iter().map(|w| w.term.clone()).collect()
    })
}

/// Every style rule, pending ones too, in stored order (the judge's `same_as`
/// numbers refer to this order).
pub fn all_rules(app: &AppHandle) -> Vec<String> {
    with_memory(app, |m| m.rules.iter().map(|r| r.rule.clone()).collect())
}

/// Applies the judge's verdicts with the local guards; returns what became
/// usable (terms and rules) to announce.
pub(crate) fn apply_verdicts(
    m: &mut Memory,
    verdicts: &[crate::judge::Verdict],
    local: &[Candidate],
    inserted: &str,
    kept: &str,
    now: i64,
) -> Vec<LearnedNews> {
    let (inserted, kept) = (unify(inserted).to_lowercase(), unify(kept).to_lowercase());
    use crate::judge::VerdictKind;
    let mut news = Vec::new();
    for v in verdicts {
        match v.kind {
            VerdictKind::Misheard => {
                let (Some(heard), Some(term)) = (v.heard.as_deref(), v.term.as_deref()) else {
                    continue;
                };
                let (heard, term) = (heard.trim(), term.trim());
                // Already counted by the local layer this dictation: the judge
                // only says whether it is technical.
                let counted = local.iter().any(|c| same_term(&c.term, term));
                let max = if v.technical {
                    LOOSE_DISTANCE
                } else {
                    MISHEARD_DISTANCE
                };
                if !counted && !close_pair(heard, term, max) {
                    log::debug!("Learning: judge item rejected (not a misheard pair)");
                    continue;
                }
                // The judge may invent a pair: both forms must be in the texts.
                if !counted
                    && !(contains_word(&inserted, &unify(heard).to_lowercase())
                        && contains_word(&kept, &unify(term).to_lowercase()))
                {
                    log::debug!("Learning: judge item rejected (not in the texts)");
                    continue;
                }
                let c = Candidate {
                    term: term.into(),
                    heard: heard.into(),
                    kind: WordKind::Misheard,
                };
                let mut new = if counted {
                    None
                } else {
                    record_word(m, &c, now)
                };
                // A technical or rare, hard-to-say word is usable at once.
                if v.technical {
                    if let Some(l) = m
                        .words
                        .iter_mut()
                        .find(|l| same_term(&l.term, term) && !l.active)
                    {
                        l.active = true;
                        new = Some(LearnedNews::Word {
                            term: l.term.clone(),
                        });
                    }
                }
                news.extend(new);
            }
            VerdictKind::StyleRule | VerdictKind::Filler => {
                let existing = v.same_as.and_then(|i| m.rules.get_mut(i));
                match existing {
                    Some(r) => {
                        r.count += 1;
                        r.last_seen = now;
                        // Two sightings, whatever the judge says about history.
                        if !r.active && r.count >= 2 {
                            r.active = true;
                            news.push(LearnedNews::Rule {
                                rule: r.rule.clone(),
                            });
                        }
                    }
                    None => {
                        let Some(rule) = v.rule.as_deref().map(str::trim).filter(|r| !r.is_empty())
                        else {
                            continue;
                        };
                        m.rules.push(StyleRule {
                            rule: rule.into(),
                            count: 1,
                            active: false,
                            last_seen: now,
                            apps: Vec::new(),
                        });
                    }
                }
            }
            VerdictKind::ContentChange | VerdictKind::Unclear => {}
        }
    }
    news
}

/// `same_as` from an index in `rules_sent` to the same rule's index now. A
/// verdict about a rule deleted during the call is dropped, so it can't come
/// back; an index the judge made up is a new rule.
fn remap_same_as(
    verdicts: &[crate::judge::Verdict],
    rules_sent: &[String],
    rules_now: &[StyleRule],
) -> Vec<crate::judge::Verdict> {
    verdicts
        .iter()
        .filter_map(|v| {
            let same_as = match v.same_as.and_then(|i| rules_sent.get(i)) {
                Some(t) => Some(rules_now.iter().position(|r| &r.rule == t)?),
                None => None,
            };
            Some(crate::judge::Verdict {
                same_as,
                ..v.clone()
            })
        })
        .collect()
}

/// Stores the judge's verdicts and announces what became usable.
/// `rules_sent` is the numbered list the judge saw: rules may have been
/// deleted or pruned during the call, so `same_as` is matched by text.
pub fn record_verdicts(
    app: &AppHandle,
    verdicts: &[crate::judge::Verdict],
    rules_sent: &[String],
    local: &[Candidate],
    inserted: &str,
    kept: &str,
) {
    if verdicts.is_empty() {
        return;
    }
    let now = chrono::Utc::now().timestamp();
    let news = with_memory(app, |m| {
        let verdicts = remap_same_as(verdicts, rules_sent, &m.rules);
        let news = apply_verdicts(m, &verdicts, local, inserted, kept, now);
        save(app, m);
        news
    });
    if !news.is_empty() {
        log::info!("Learning: the judge made {} item(s) usable", news.len());
        let _ = app.emit("learned-words", &news);
        announce(app, &news);
    }
}

/// Active style rules that hold in `app_name` (or everywhere).
pub fn active_rules(app: &AppHandle, app_name: &str) -> Vec<String> {
    with_memory(app, |m| {
        m.rules
            .iter()
            .filter(|r| r.active && (r.apps.is_empty() || r.apps.iter().any(|a| a == app_name)))
            .map(|r| r.rule.clone())
            .collect()
    })
}

/// Records the corrections found; returns them all (pending ones too).
fn learn(app: &AppHandle, inserted: &str, fin: &str) -> Vec<Candidate> {
    let found = candidates(inserted, fin);
    if found.is_empty() {
        return found;
    }
    let now = chrono::Utc::now().timestamp();
    // F95: new spellings are announced once, grouped, with an undo.
    let mut new_terms = Vec::new();
    with_memory(app, |m| {
        for c in &found {
            log::info!(
                "Learned a spelling ({} chars, {:?})",
                c.term.chars().count(),
                c.kind
            );
            new_terms.extend(record_word(m, c, now));
        }
        save(app, m);
    });
    if !new_terms.is_empty() {
        let _ = app.emit("learned-words", &new_terms);
        announce(app, &new_terms);
    }
    found
}

/// F95, as Wispr: the overlay pill says what was learned, over the window the
/// user dictates into, with the system's notification sound.
fn announce(app: &AppHandle, news: &[LearnedNews]) {
    use crate::notices::{Notice, NoticeCode, NoticeSource};
    let labels: Vec<&str> = news.iter().map(LearnedNews::label).collect();
    let (app, detail) = (app.clone(), labels.join(", "));
    std::thread::spawn(move || {
        // A dictation on the pill goes first: the news waits for it (≤ 20 s).
        for _ in 0..80 {
            if !crate::overlay::is_session_showing() {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        crate::overlay::show_overlay_notice(
            &app,
            &Notice::new(NoticeCode::WordsLearned, NoticeSource::Dictation).with_detail(detail),
        );
        if crate::settings::get_settings(&app).learned_sound_enabled {
            play_notification_sound();
        }
    });
}

#[cfg(windows)]
fn play_notification_sound() {
    use windows::core::w;
    use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};
    // SAFETY: a static alias string; the sound plays asynchronously.
    unsafe {
        let _ = PlaySoundW(
            w!("Notification.Default"),
            None,
            SND_ALIAS | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(not(windows))]
fn play_notification_sound() {}

pub fn count(app: &AppHandle) -> usize {
    with_memory(app, |m| m.words.len())
}

#[tauri::command]
#[specta::specta]
pub fn get_learned_words(app: AppHandle) -> Vec<Learned> {
    with_memory(&app, |m| m.words.clone())
}

#[tauri::command]
#[specta::specta]
pub fn delete_learned_word(app: AppHandle, term: String) {
    with_memory(&app, |m| {
        m.words.retain(|l| l.term != term);
        save(&app, m);
    });
}

#[tauri::command]
#[specta::specta]
pub fn change_asr_context_setting(
    app: AppHandle,
    value: crate::asr_context::AsrContextSettings,
) -> Result<(), String> {
    value.validate()?;
    let mut settings = crate::settings::get_settings(&app);
    settings.asr_context = value;
    crate::settings::write_settings(&app, settings);
    Ok(())
}

/// The phrases the user says often (30 days, ≥ 3 dictations), as the speech
/// model gets them.
#[tauri::command]
#[specta::specta]
pub fn get_frequent_expressions(app: AppHandle) -> Vec<String> {
    use tauri::Manager;
    let now = chrono::Utc::now().timestamp();
    app.state::<std::sync::Arc<crate::managers::history::HistoryManager>>()
        .past_dictations(now - 30 * 86_400)
        .map(|past| crate::recall::frequent_expressions(&past, now, 3, 20))
        .unwrap_or_default()
}

/// What the loaded ASR model accepts; `None` when no model is loaded.
#[tauri::command]
#[specta::specta]
pub fn get_asr_support(app: AppHandle) -> Option<crate::asr_context::AsrSupport> {
    use tauri::Manager;
    app.state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>()
        .asr_support()
}

#[tauri::command]
#[specta::specta]
pub fn get_style_rules(app: AppHandle) -> Vec<StyleRule> {
    with_memory(&app, |m| m.rules.clone())
}

#[tauri::command]
#[specta::specta]
pub fn delete_style_rule(app: AppHandle, rule: String) {
    with_memory(&app, |m| {
        m.rules.retain(|r| r.rule != rule);
        save(&app, m);
    });
}

#[tauri::command]
#[specta::specta]
pub fn clear_learned_words(app: AppHandle) {
    with_memory(&app, |m| {
        m.words.clear();
        save(&app, m);
    });
}

/// Clears the style rules only; the learned words stay.
#[tauri::command]
#[specta::specta]
pub fn clear_style_rules(app: AppHandle) {
    with_memory(&app, |m| {
        m.rules.clear();
        save(&app, m);
    });
}

/// Undoes one heard form of a learned word (the notification's Undo).
#[tauri::command]
#[specta::specta]
pub fn remove_heard_form(app: AppHandle, term: String, heard: String) {
    with_memory(&app, |m| {
        drop_form(m, &term, &heard);
        save(&app, m);
    });
}

/// Readable export to move to another machine (F113).
#[tauri::command]
#[specta::specta]
pub fn export_learned_words(app: AppHandle, path: String) -> Result<(), String> {
    // Words and style rules: everything learned.
    let json =
        with_memory(&app, |m| serde_json::to_string_pretty(&*m)).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ----------------------------------------------------------- observation

/// The current observation; a newer dictation ends the older one.
static OBSERVATION: AtomicU64 = AtomicU64::new(0);
const WINDOW: Duration = Duration::from_secs(60);

/// Ends the running observation now (a new dictation started): it learns
/// from its last reading before this dictation's rewrite (F98).
pub fn end_observation() {
    OBSERVATION.fetch_add(1, Ordering::SeqCst);
}

/// The dictated region: what lies between the whole text before it and the
/// whole text after it, as Wispr's anchors (E71, E72). `None` once the field
/// no longer starts and ends with them.
pub fn region<'a>(text: &'a str, before: &str, after: &str) -> Option<&'a str> {
    (text.len() >= before.len() + after.len() && text.starts_with(before) && text.ends_with(after))
        .then(|| &text[before.len()..text.len() - after.len()])
}

/// Spaces and apostrophes as fields give them back: the rewrite's French
/// narrow no-break space before "?" reads as a plain space in VS Code and
/// Notepad, so the pasted text was never found.
pub fn unify(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{a0}' | '\u{202f}' | '\u{2009}' | '\u{2007}' => ' ',
            '\u{2019}' => '\'',
            _ => c,
        })
        .collect()
}

/// Why `inserted` isn't in `text`, without either text: lengths, the longest
/// pasted prefix found, and the two characters that differ after it.
pub fn mismatch(text: &str, inserted: &str) -> String {
    let chars: Vec<char> = inserted.chars().collect();
    let found = |n: usize| text.contains(&chars[..n].iter().collect::<String>());
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if found(mid) {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let prefix: String = chars[..lo].iter().collect();
    let field_next = text
        .find(&prefix)
        .and_then(|i| text[i + prefix.len()..].chars().next());
    format!(
        "pasted text not found: field {} chars, pasted {} chars, same for {} chars, \
         then pasted {:?} vs field {:?}, case-insensitive match: {}",
        text.chars().count(),
        chars.len(),
        lo,
        chars.get(lo).map(|c| format!("U+{:04X}", *c as u32)),
        field_next.map(|c| format!("U+{:04X}", c as u32)),
        text.to_lowercase().contains(&inserted.to_lowercase())
    )
}

/// Wispr's anchors for the pasted text once it shows up in the field: the
/// whole text before and after it. With several copies, the one nearest the
/// caret read at release (in chars).
pub fn locate(text: &str, inserted: &str, caret: usize) -> Option<(String, String)> {
    if inserted.is_empty() {
        return None;
    }
    let i = text
        .match_indices(inserted)
        .map(|(i, _)| i)
        .min_by_key(|&i| text[..i].chars().count().abs_diff(caret))?;
    Some((
        text[..i].to_string(),
        text[i + inserted.len()..].to_string(),
    ))
}

/// F80–F83: watches the field the text was pasted into, then learns from the
/// last reading whose anchors still hold.
pub fn observe(
    app: &AppHandle,
    field: &crate::context::Field,
    inserted: &str,
    history_id: Option<i64>,
    app_name: String,
) {
    if !crate::settings::get_settings(app).learning_enabled {
        return;
    }
    if !field.caret_known {
        log::debug!("Learning: not observing, the field's caret position is unknown");
        return;
    }
    let caret = field.before.chars().count();
    let inserted = unify(inserted.trim());
    let id = OBSERVATION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("observe".into())
        .spawn(move || {
            if !crate::context::watch_focused() {
                log::debug!("Learning: not observing, the focused field can't be watched");
                return;
            }
            let started = Instant::now();
            let mut last: Option<String> = None;
            let mut anchors: Option<(String, String)> = None;
            // Lengths only in the log: the text itself stays private.
            let mut ended = "the 60 s window closed";
            // A newer dictation ended this one: learn from the last reading,
            // never read its field again.
            while started.elapsed() < WINDOW {
                if OBSERVATION.load(Ordering::SeqCst) != id {
                    ended = "a newer dictation started";
                    break;
                }
                match crate::context::read_watched().map(|t| unify(&t)) {
                    // An empty field was sent or cleared: keep the last reading.
                    Some(text) if !text.trim().is_empty() => {
                        if anchors.is_none() {
                            anchors = locate(&text, &inserted, caret);
                        }
                        match &anchors {
                            // The paste can take a moment to show up.
                            None if started.elapsed() < Duration::from_secs(3) => {}
                            None => {
                                ended = "the pasted text never showed up in the field";
                                log::debug!("Learning: {}", mismatch(&text, &inserted));
                                break;
                            }
                            Some((before, after)) => match region(&text, before, after) {
                                Some(r) => last = Some(r.trim().to_string()),
                                // Text typed outside the dictation (Wispr: anchor_mismatch).
                                None => {
                                    ended = "the text around the dictation changed";
                                    break;
                                }
                            },
                        }
                    }
                    Some(_) => {
                        ended = "the field was emptied (sent)";
                        break;
                    }
                    None => {
                        ended = "the field could no longer be read";
                        break;
                    }
                }
                // Often enough to catch a fix made just before Enter sends it
                // (Wispr learns from the last reading before the send, E224).
                std::thread::sleep(Duration::from_millis(300));
            }
            let found = match &last {
                Some(fin) if fin != &inserted => learn(&app, &inserted, fin),
                _ => Vec::new(),
            };
            log::debug!(
                "Learning: observation ended ({ended}); edited: {}, learned: {}",
                last.as_ref().is_some_and(|fin| fin != &inserted),
                found.len()
            );
            // The judge reads the correction in the background (style rules,
            // misheard words the local layer couldn't tell).
            let settings = crate::settings::get_settings(&app);
            if let Some(fin) = last.as_ref().filter(|fin| *fin != &inserted) {
                if settings.learning_enabled
                    && settings.learning_judge_enabled
                    && settings.context_send_enabled
                {
                    tauri::async_runtime::spawn(crate::judge::run(
                        app.clone(),
                        inserted.clone(),
                        fin.clone(),
                        app_name.clone(),
                        found.clone(),
                    ));
                }
            }
            // What the user kept feeds the next dictations' context.
            if let (Some(id), Some(fin)) = (history_id, &last) {
                use tauri::Manager;
                let hm = app.state::<std::sync::Arc<crate::managers::history::HistoryManager>>();
                if let Err(e) = hm.set_kept_text(id, fin) {
                    log::warn!("Learning: couldn't record the kept text: {e}");
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Texts holding every form the verdict tests use.
    const T_IN: &str = "conflig jeffu chefo mardi";
    const T_KEPT: &str = "config jevu jeudi";

    #[test]
    fn tracks_the_dictation_with_wispr_anchors() {
        // Notepad: a space joined after the pasted text used to cut the region
        // at the first space of the field.
        let pasted = "Concernant le benchmark, est-il meilleur entre Gemma et Quen ?";
        let field = format!("{pasted} ");
        let (b, a) = locate(&field, pasted, 0).unwrap();
        assert_eq!(region(&field, &b, &a), Some(pasted));
        let fixed = "Concernant le benchmark, est-il meilleur entre Gemma et Qwen ? ";
        let r = region(fixed, &b, &a).unwrap();
        assert_eq!(
            candidates(pasted, r.trim()),
            vec![("Qwen".into(), "Quen".into())]
        );
        // Text typed outside the dictation ends the tracking.
        assert_eq!(region("Bonjour, autre chose", "Bonjour, ", " Merci"), None);
        // A placeholder read at release doesn't matter: the field is searched.
        let (b, a) = locate("Salut Quinn.", "Salut Quinn.", 22).unwrap();
        assert_eq!((b.as_str(), a.as_str()), ("", ""));
        // Two copies: the one nearest the caret read at release.
        let (b, _) = locate("ok. ok. ok.", "ok.", 4).unwrap();
        assert_eq!(b, "ok. ");
    }

    #[test]
    fn finds_the_pasted_text_despite_a_narrow_no_break_space() {
        let pasted = unify("est-il meilleur entre Gemma et Quen\u{202f}?");
        let field = unify("Concernant le benchmark, est-il meilleur entre Gemma et Quen ?");
        assert!(locate(&field, &pasted, 0).is_some());
        assert_eq!(unify("l\u{2019}apprentissage"), "l'apprentissage");
    }

    #[test]
    fn one_learned_word_keeps_every_form_heard() {
        // Memory saved with a single heard form still loads.
        let mut l: Learned =
            serde_json::from_str(r#"{"term":"Qwen","heard":"Quen","count":1,"last_seen":0}"#)
                .unwrap();
        assert_eq!(l.heard, vec!["Quen"]);
        assert!(add_heard(&mut l, "Quinn".into()));
        assert!(!add_heard(&mut l, "quen".into()));
        assert_eq!(l.heard, vec!["Quen", "Quinn"]);
        for i in 0..20 {
            add_heard(&mut l, format!("forme{i}"));
        }
        assert_eq!(l.heard.len(), HEARD_MAX);
    }

    #[test]
    fn explains_a_mismatch_without_the_text() {
        let m = mismatch("Bonjour Adrien, ça va", "Bonjour Adryen, ça va");
        assert!(m.contains("same for 11 chars"), "{m}");
        assert!(m.contains("U+0079") && m.contains("U+0069"), "{m}");
        assert!(!m.contains("Bonjour"));
    }

    #[test]
    fn learns_the_users_vs_code_corrections() {
        assert_eq!(
            candidates(
                "Tu regarderas, j'ai écrit Gma, GPT, OSS, etc.",
                "Tu regarderas, j'ai écrit Gemma, GPT, OSS, etc."
            ),
            vec![("Gemma".into(), "Gma".into())]
        );
        // A lowercase correction of a name the ASR capitalized.
        assert_eq!(
            candidates(
                "Alors au final, Coen est meilleur que GPT OSS ou pas ?",
                "Alors au final, qwen est meilleur que GPT OSS ou pas ?"
            ),
            vec![("qwen".into(), "Coen".into())]
        );
        // One heard word split in two.
        assert_eq!(
            candidates(
                "Est-ce que GPTOC c'est mieux ou c'est GEMA le meilleur ?",
                "Est-ce que GPT OSS c'est mieux ou c'est gemma le meilleur ?"
            ),
            vec![
                ("GPT OSS".into(), "GPTOC".into()),
                ("gemma".into(), "GEMA".into())
            ]
        );
        // Two neighbouring fixes in a list stay two terms.
        assert_eq!(
            candidates(
                "Donc, au niveau du benchmark, entre Quen, GEMA et GPT OSS, c'est quoi le meilleur ?",
                "donc, au niveau du benchmark, entre Qwen, gemma et GPT OSS, c'est quoi le meilleur ?"
            ),
            vec![
                ("Qwen".into(), "Quen".into()),
                ("gemma".into(), "GEMA".into())
            ]
        );
        // Still nothing for a lowercase fix of a common word, or a sentence start.
        assert!(candidates("Je mange une pome ce soir.", "Je mange une pomme ce soir.").is_empty());
        assert!(candidates("Rapport prêt pour demain.", "Rappel prêt pour demain.").is_empty());
    }

    #[test]
    fn learns_names_and_terms_a01_to_a06() {
        assert_eq!(
            candidates(
                "Nous installons Corvanti ce soir.",
                "Nous installons Quorvantis ce soir."
            ),
            vec![("Quorvantis".into(), "Corvanti".into())]
        );
        assert_eq!(
            candidates(
                "Je travaille avec Kazimierz Bignowski sur le projet.",
                "Je travaille avec Kasimir Zbignowski sur le projet."
            ),
            vec![("Kasimir Zbignowski".into(), "Kazimierz Bignowski".into())]
        );
        assert_eq!(
            candidates(
                "Le dossier est suivi par Aurélie Kenen.",
                "Le dossier est suivi par Aurélie Quennehen."
            ),
            vec![("Quennehen".into(), "Kenen".into())]
        );
        assert!(candidates("Je pars à Paris demain.", "Je pars à PARIS demain.").is_empty()); // A04
        assert!(candidates(
            "Nous devons absolument terminer le rapport avant la réunion de jeudi prochain avec Paul.",
            "Merci de m'envoyer les chiffres dès que possible, on en reparle ensemble."
        )
        .is_empty()); // A05
        assert!(candidates("Rien ne change ici.", "Rien ne change ici.").is_empty());
        // A06
    }

    #[test]
    fn rejects_meaning_grammar_and_truncation() {
        assert!(
            candidates("La réunion est mardi matin.", "La réunion est jeudi matin.").is_empty()
        ); // L04
        assert!(candidates("Le projet avance bien.", "Le projets avance bien.").is_empty()); // L05
        assert!(candidates(
            "Quelle heure est-il maintenant ?",
            "Kelmorine heure est-il maintenant ?"
        )
        .is_empty()); // L07
        assert!(candidates("Salut Maximilien, ça va ?", "Salut Max, ça va ?").is_empty()); // S42
        assert!(candidates("Projet avance bien.", "Projets avance bien.").is_empty());
        assert!(candidates("Merci Jean. Projet validé.", "Merci Jean. Projets validé.").is_empty());
        assert_eq!(
            candidates(
                "on migre la base postgéré demain",
                "on migre la base PostgreSQL demain"
            ),
            vec![("PostgreSQL".into(), "postgéré".into())]
        ); // L03
    }

    /// The name/term layer as the tests above knew it: misheard common words
    /// (pending until seen twice) are covered by their own tests below.
    fn candidates(inserted: &str, fin: &str) -> Vec<(String, String)> {
        super::candidates(inserted, fin)
            .into_iter()
            .filter(|c| c.kind == WordKind::Term)
            .map(|c| (c.term, c.heard))
            .collect()
    }

    #[test]
    fn grammar_changes_are_never_misheard() {
        for (a, b) in [
            ("Il et parti hier soir.", "Il est parti hier soir."),
            ("Il a pris ces affaires.", "Il a pris ses affaires."),
            ("Je vais le voir demain.", "Je vais la voir demain."),
            ("On se voit ce soir.", "On se voit se soir."),
            ("Je l'ai envoyer hier.", "Je l'ai envoyé hier."),
            ("I think their right.", "I think they're right."),
            ("It was better then that.", "It was better than that."),
        ] {
            assert!(super::candidates(a, b).is_empty(), "{a} -> {b}");
        }
        // A misheard common word of 4+ letters still counts.
        assert_eq!(
            super::candidates("ouvre le fichier conflig", "ouvre le fichier config").len(),
            1
        );
        // Accepted trade-off: words under 4 letters are no longer learned.
        assert!(
            super::candidates("regarde le gag du serveur", "regarde le log du serveur").is_empty()
        );
    }

    #[test]
    fn purge_drops_grammar_pairs_already_learned() {
        let now = 1_000_000;
        let word = |term: &str, heard: &str, kind| Learned {
            term: term.into(),
            heard: vec![heard.into()],
            count: 2,
            last_seen: now,
            kind,
            active: true,
        };
        let mut m = Memory::default();
        m.words.push(word("est", "et", WordKind::Misheard));
        m.words.push(word("Qwen", "Quen", WordKind::Term));
        prune(&mut m, now);
        let terms: Vec<&str> = m.words.iter().map(|w| w.term.as_str()).collect();
        assert_eq!(terms, ["Qwen"]);
    }

    #[test]
    fn saying_a_heard_form_keeps_the_word() {
        let mut m = Memory::default();
        m.words.push(Learned {
            term: "Qwen".into(),
            heard: vec!["Quen".into()],
            count: 1,
            last_seen: 0,
            kind: WordKind::Term,
            active: true,
        });
        assert!(mark_used(&mut m, "on compare quen et gemma", 500));
        assert_eq!(m.words[0].last_seen, 500);
        assert!(!mark_used(&mut m, "on compare queneau", 900)); // not a whole word
        assert_eq!(m.words[0].last_seen, 500);
    }

    #[test]
    fn undo_a_new_form_keeps_the_word() {
        let c = |heard: &str| Candidate {
            term: "Qwen".into(),
            heard: heard.into(),
            kind: WordKind::Term,
        };
        let mut m = Memory::default();
        let word = LearnedNews::Word {
            term: "Qwen".into(),
        };
        assert_eq!(record_word(&mut m, &c("Quen"), 0), Some(word));
        let form = LearnedNews::Form {
            term: "Qwen".into(),
            heard: "Quinn".into(),
        };
        assert_eq!(record_word(&mut m, &c("Quinn"), 1), Some(form));
        drop_form(&mut m, "Qwen", "Quinn");
        assert_eq!(m.words[0].heard, vec!["Quen"]);
        drop_form(&mut m, "qwen", "quen");
        assert!(m.words.is_empty());
    }

    #[test]
    fn judge_pairs_must_be_in_the_texts() {
        use crate::judge::{Verdict, VerdictKind};
        let v = Verdict {
            kind: VerdictKind::Misheard,
            heard: Some("dashbord".into()),
            term: Some("dashboard".into()),
            rule: None,
            same_as: None,
            consistent_with_history: false,
            technical: true,
        };
        let kept = "le dashboard est prêt";
        let mut m = Memory::default();
        let news = apply_verdicts(&mut m, &[v.clone()], &[], "le tableau est prêt", kept, 0);
        assert!(news.is_empty() && m.words.is_empty());
        let news = apply_verdicts(&mut m, &[v], &[], "le dashbord est prêt", kept, 0);
        let word = LearnedNews::Word {
            term: "dashboard".into(),
        };
        assert_eq!(news, vec![word]);
    }

    #[test]
    fn a_style_rule_needs_two_sightings() {
        use crate::judge::{Verdict, VerdictKind};
        let rule = "Toujours vouvoyer.";
        let v = |same_as| Verdict {
            kind: VerdictKind::StyleRule,
            heard: None,
            term: None,
            rule: Some(rule.into()),
            same_as,
            consistent_with_history: true,
            technical: false,
        };
        let mut m = Memory::default();
        assert!(apply_verdicts(&mut m, &[v(None)], &[], "a", "b", 0).is_empty());
        assert!(!m.rules[0].active);
        let news = apply_verdicts(&mut m, &[v(Some(0))], &[], "a", "b", 1);
        assert!(m.rules[0].active);
        assert_eq!(news, vec![LearnedNews::Rule { rule: rule.into() }]);
    }

    #[test]
    fn misheard_common_word_is_learned_after_two_corrections() {
        let c = super::candidates(
            "regarde le conflig du serveur",
            "regarde le config du serveur",
        );
        assert_eq!(
            c,
            vec![Candidate {
                term: "config".into(),
                heard: "conflig".into(),
                kind: WordKind::Misheard
            }]
        );
        let mut m = Memory::default();
        assert!(record_word(&mut m, &c[0], 0).is_none());
        assert!(!m.words[0].active);
        assert!(record_word(&mut m, &c[0], 1).is_some());
        assert!(m.words[0].active);
    }

    #[test]
    fn meaning_changes_are_never_learned() {
        assert!(super::candidates("la réunion est mardi", "la réunion est jeudi").is_empty());
        assert!(super::candidates("je dis oui merci", "je dis non merci").is_empty());
        assert!(super::candidates("on part à deux", "on part à trois").is_empty());
        assert!(!is_misheard_pair("mardi", "jeudi"));
        assert!(is_misheard_pair("conflig", "config"));
    }

    #[test]
    fn same_as_follows_the_rule_text_not_its_old_position() {
        use crate::judge::{Verdict, VerdictKind};
        let rule = |r: &str| StyleRule {
            rule: r.into(),
            count: 1,
            active: false,
            last_seen: 0,
            apps: Vec::new(),
        };
        let v = |same_as, text: Option<&str>| Verdict {
            kind: VerdictKind::StyleRule,
            heard: None,
            term: None,
            rule: text.map(Into::into),
            same_as,
            consistent_with_history: true,
            technical: false,
        };
        let sent = vec!["A".to_string(), "B".to_string()];
        // "A" was deleted during the call: the judge's 1 ("B") is now 0, its 0
        // is dropped, and an index it made up stays a new rule.
        let mut m = Memory {
            rules: vec![rule("B")],
            ..Default::default()
        };
        let verdicts = [
            v(Some(1), None),
            v(Some(0), Some("A")),
            v(Some(9), Some("C")),
        ];
        let out = remap_same_as(&verdicts, &sent, &m.rules);
        let got: Vec<_> = out.iter().map(|v| v.same_as).collect();
        assert_eq!(got, vec![Some(0), None]);
        apply_verdicts(&mut m, &out, &[], T_IN, T_KEPT, 1);
        let rules: Vec<_> = m.rules.iter().map(|r| r.rule.as_str()).collect();
        assert_eq!(rules, vec!["B", "C"], "the deleted rule never comes back");
        assert_eq!(m.rules[0].count, 2);
    }

    #[test]
    fn verdicts_respect_the_guards() {
        use crate::judge::{Verdict, VerdictKind};
        let v = |kind, heard: Option<&str>, term: Option<&str>, rule: Option<&str>, same_as, ok| {
            Verdict {
                kind,
                heard: heard.map(Into::into),
                term: term.map(Into::into),
                rule: rule.map(Into::into),
                same_as,
                consistent_with_history: ok,
                technical: false,
            }
        };
        let mut m = Memory::default();
        // A meaning change learns nothing, even proposed as misheard.
        apply_verdicts(
            &mut m,
            &[
                v(
                    VerdictKind::Misheard,
                    Some("mardi"),
                    Some("jeudi"),
                    None,
                    None,
                    true,
                ),
                v(VerdictKind::ContentChange, None, None, None, None, true),
            ],
            &[],
            T_IN,
            T_KEPT,
            0,
        );
        assert!(m.words.is_empty() && m.rules.is_empty());
        // A misheard word from the judge is pending until seen twice too.
        apply_verdicts(
            &mut m,
            &[v(
                VerdictKind::Misheard,
                Some("conflig"),
                Some("config"),
                None,
                None,
                true,
            )],
            &[],
            T_IN,
            T_KEPT,
            0,
        );
        assert!(!m.words[0].active);
        // A lone rule not backed by history waits for a second sighting.
        let rule = "Préférer « de manière claire et concise ».";
        apply_verdicts(
            &mut m,
            &[v(
                VerdictKind::StyleRule,
                None,
                None,
                Some(rule),
                None,
                false,
            )],
            &[],
            T_IN,
            T_KEPT,
            0,
        );
        assert!(!m.rules[0].active);
        let on = apply_verdicts(
            &mut m,
            &[v(
                VerdictKind::StyleRule,
                None,
                None,
                Some("…"),
                Some(0),
                false,
            )],
            &[],
            T_IN,
            T_KEPT,
            1,
        );
        assert!(m.rules[0].active && m.rules[0].count == 2);
        assert_eq!(on, vec![LearnedNews::Rule { rule: rule.into() }]);
        // Even "consistent with history", a new rule waits for a second sighting.
        apply_verdicts(
            &mut m,
            &[v(
                VerdictKind::Filler,
                None,
                None,
                Some("Supprimer « donc voilà »."),
                None,
                true,
            )],
            &[],
            T_IN,
            T_KEPT,
            2,
        );
        assert!(!m.rules[1].active);
        // A `same_as` out of range is a new rule, not a crash.
        apply_verdicts(
            &mut m,
            &[v(
                VerdictKind::StyleRule,
                None,
                None,
                Some("Autre."),
                Some(99),
                false,
            )],
            &[],
            T_IN,
            T_KEPT,
            3,
        );
        assert_eq!(m.rules.len(), 3);
    }

    fn word(
        term: &str,
        heard: &str,
        count: u32,
        last_seen: i64,
        kind: WordKind,
        active: bool,
    ) -> Learned {
        Learned {
            term: term.into(),
            heard: vec![heard.into()],
            count,
            last_seen,
            kind,
            active,
        }
    }

    #[test]
    fn old_memory_array_still_loads() {
        let m = parse_memory(br#"[{"term":"Claude","heard":["Cloud"],"count":2,"last_seen":0}]"#);
        assert_eq!(m.words.len(), 1);
        assert_eq!(m.words[0].kind, WordKind::Term);
        assert!(m.words[0].active);
        assert!(m.rules.is_empty());
        // The new format round-trips.
        let json = serde_json::to_vec(&m).unwrap();
        assert_eq!(parse_memory(&json).words, m.words);
    }

    #[test]
    fn a_word_fixed_twice_in_one_dictation_is_learned() {
        // "chefo" → "jevu" alone is too far (0.8); with "jeffu" → "jevu" it counts.
        let c = super::candidates(
            "le chefo est prêt et le jeffu aussi",
            "le jevu est prêt et le jevu aussi",
        );
        assert_eq!(c.len(), 2, "{c:?}");
        let mut m = Memory::default();
        for x in &c {
            record_word(&mut m, x, 0);
        }
        assert_eq!(m.words.len(), 1);
        assert!(m.words[0].active);
        assert_eq!(m.words[0].heard, vec!["jeffu", "chefo"]);
        // Alone, "chefo" → "jevu" still learns nothing.
        assert!(super::candidates("le chefo est prêt", "le jevu est prêt").is_empty());
        // Case doesn't split a word in two.
        record_word(
            &mut m,
            &Candidate {
                term: "Jevu".into(),
                heard: "jefu".into(),
                kind: WordKind::Misheard,
            },
            1,
        );
        assert_eq!(m.words.len(), 1);
    }

    #[test]
    fn a_technical_word_is_learned_at_once() {
        use crate::judge::{Verdict, VerdictKind};
        let v = |heard: &str, technical| Verdict {
            kind: VerdictKind::Misheard,
            heard: Some(heard.into()),
            term: Some("jevu".into()),
            rule: None,
            same_as: None,
            consistent_with_history: false,
            technical,
        };
        // Not technical: one correction stays pending.
        let mut m = Memory::default();
        apply_verdicts(&mut m, &[v("jeffu", false)], &[], T_IN, T_KEPT, 0);
        assert!(!m.words[0].active);
        // Technical: usable at once, even a farther form ("chefo", 0.8).
        let mut m = Memory::default();
        let news = apply_verdicts(&mut m, &[v("chefo", true)], &[], T_IN, T_KEPT, 0);
        assert!(m.words[0].active);
        assert_eq!(
            news,
            vec![LearnedNews::Word {
                term: "jevu".into()
            }]
        );
        // Found locally this dictation: not counted twice, only promoted.
        let local = super::candidates("le jeffu est prêt", "le jevu est prêt");
        let mut m = Memory::default();
        record_word(&mut m, &local[0], 0);
        apply_verdicts(&mut m, &[v("jeffu", false)], &local, T_IN, T_KEPT, 0);
        assert!(!m.words[0].active && m.words[0].count == 1);
        apply_verdicts(&mut m, &[v("jeffu", true)], &local, T_IN, T_KEPT, 0);
        assert!(m.words[0].active && m.words[0].count == 1);
        // Still no meaning change, technical or not.
        let mut m = Memory::default();
        let mut mj = v("mardi", true);
        mj.term = Some("jeudi".into());
        apply_verdicts(&mut m, &[mj], &[], T_IN, T_KEPT, 0);
        assert!(m.words.is_empty());
    }

    #[test]
    fn prune_expires_pending_and_caps() {
        let day = 86_400;
        let mut m = Memory::default();
        m.words
            .push(word("config", "conflig", 1, 0, WordKind::Misheard, false));
        m.words
            .push(word("Qwen", "Quen", 1, 0, WordKind::Term, true));
        m.words.push(word(
            "Kubernetes",
            "cubernetis",
            1,
            2 * day,
            WordKind::Term,
            true,
        ));
        prune(&mut m, 31 * day);
        // Active or pending, a word unused for 30 days goes.
        assert_eq!(
            m.words.iter().map(|w| w.term.as_str()).collect::<Vec<_>>(),
            vec!["Kubernetes"]
        );
        m.words = (0..WORDS_MAX + 5)
            .map(|i| word(&format!("w{i}"), "", 1, i as i64, WordKind::Term, true))
            .collect();
        prune(&mut m, 0);
        assert_eq!(m.words.len(), WORDS_MAX);
        // The least used and oldest goes first.
        assert!(!m.words.iter().any(|w| w.term == "w0"));
    }

    #[test]
    fn dictating_a_learned_word_restarts_its_30_days() {
        let mut m = Memory::default();
        m.words
            .push(word("Qwen", "Quen", 1, 0, WordKind::Term, true));
        m.words
            .push(word("Node.js", "node jay s", 1, 0, WordKind::Term, true));
        m.words
            .push(word("config", "conflig", 1, 0, WordKind::Misheard, false));
        m.words
            .push(word("art", "hart", 1, 0, WordKind::Term, true));

        assert!(mark_used(
            &mut m,
            "J'utilise qwen et Node.js, le config dit partir.",
            9
        ));
        let seen = |t: &str| m.words.iter().find(|w| w.term == t).unwrap().last_seen;
        assert_eq!(seen("Qwen"), 9);
        assert_eq!(seen("Node.js"), 9);
        // Pending: dictating it proves nothing. Inside "partir": not the word.
        assert_eq!(seen("config"), 0);
        assert_eq!(seen("art"), 0);
        assert!(!mark_used(&mut m, "rien ici", 10));
    }
}
