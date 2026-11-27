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
    /// Pasted texts that held the term (Wispr `frequencyUsed`, E78).
    #[serde(default)]
    pub used: u32,
    #[serde(default)]
    pub source: Source,
}

/// Where a learned entry comes from (EX-6.1). The user's own words are
/// `settings.custom_words`, not entries here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    #[default]
    UserEdits,
    /// Reserved: words read on screen (Wispr `app_context`, off there too).
    AppContext,
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
    /// Unix seconds of the one re-validation of pending words (EX-5); 0 = not yet.
    #[serde(default)]
    pub revalidated_at: i64,
    /// Pairs the user undid, `(heard, term)` lowercased: never learned again (EX-6.2).
    #[serde(default)]
    pub refused: Vec<(String, String)>,
    /// What learning decided over the last 30 days (EX-7.2).
    #[serde(default)]
    pub events: Vec<Event>,
}

/// One learning decision, kept 30 days for the counters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub at: i64,
    /// "observed", "learned", "pending" or "rejected:<reason>".
    pub what: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Observed,
    Learned,
    Pending,
    Rejected(&'static str),
}

/// The counters shown in Settings → Learning.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Type)]
pub struct LearningStats {
    pub observed: u32,
    pub learned: u32,
    pub pending: u32,
    pub rejected: u32,
    /// Rejections per reason, most frequent first.
    pub rejected_by: Vec<(String, u32)>,
}

const STATS_DAYS: i64 = 30;
const EVENTS_MAX: usize = 5000;

/// Counts one decision and forgets those older than 30 days.
pub(crate) fn note(m: &mut Memory, o: Outcome, now: i64) {
    let what = match o {
        Outcome::Observed => "observed".to_string(),
        Outcome::Learned => "learned".to_string(),
        Outcome::Pending => "pending".to_string(),
        Outcome::Rejected(r) => format!("rejected:{r}"),
    };
    m.events.retain(|e| e.at > now - STATS_DAYS * 86_400);
    m.events.push(Event { at: now, what });
    if m.events.len() > EVENTS_MAX {
        m.events.remove(0);
    }
}

pub(crate) fn stats(m: &Memory, now: i64) -> LearningStats {
    let mut s = LearningStats::default();
    for e in m.events.iter().filter(|e| e.at > now - STATS_DAYS * 86_400) {
        match e.what.as_str() {
            "observed" => s.observed += 1,
            "learned" => s.learned += 1,
            "pending" => s.pending += 1,
            other => {
                s.rejected += 1;
                let reason = other.strip_prefix("rejected:").unwrap_or(other);
                match s.rejected_by.iter_mut().find(|(r, _)| r == reason) {
                    Some((_, n)) => *n += 1,
                    None => s.rejected_by.push((reason.to_string(), 1)),
                }
            }
        }
    }
    s.rejected_by
        .sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    s
}

pub const WORDS_MAX: usize = 2000;
pub const RULES_MAX: usize = 100;
const PENDING_DAYS: i64 = 30;

/// Reads the memory, including the older format: a bare array of words.
fn parse_memory(json: &[u8]) -> Result<Memory, serde_json::Error> {
    serde_json::from_slice::<Memory>(json).or_else(|_| {
        serde_json::from_slice::<Vec<Learned>>(json).map(|words| Memory {
            words,
            rules: Vec::new(),
            revalidated_at: 0,
            refused: Vec::new(),
            events: Vec::new(),
        })
    })
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
    // Wispr never expires its dictionary: only pending words leave.
    m.words
        .retain(|w| (w.active || w.last_seen >= expired) && !w.heard.is_empty());
    m.rules.retain(|r| r.active || r.last_seen >= expired);
    if m.words.len() > WORDS_MAX {
        m.words.sort_by(|a, b| {
            b.used
                .cmp(&a.used)
                .then(b.count.cmp(&a.count))
                .then(b.last_seen.cmp(&a.last_seen))
        });
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

/// Corrections worth learning (F85–F87, EX-2.1): runs of 1 to 4 substituted
/// words between unchanged words. Each run is either a [`Candidate`] for the
/// validator, however far it sounds (`.env` ← « point oeuvre »), or
/// [`Local::Guarded`] by a rule that decides alone: rewrite, case only,
/// truncation or plural, possessive or contraction, meaning word, grammar.
/// Candidates that look like names come first, at most 4 per dictation.
pub fn local_candidates(inserted: &str, fin: &str) -> Vec<Local> {
    let a: Vec<&str> = inserted.split_whitespace().collect();
    let b: Vec<&str> = fin.split_whitespace().collect();
    let tags = align(&a, &b);
    // Dictated words that changed; text typed after the dictation (Ins) is
    // no rewrite of it.
    let edited = tags
        .iter()
        .filter(|t| matches!(t.0, Tag::Sub | Tag::Del))
        .count();
    // A rewrite, not a correction (spec §8.7: > 30 words; or most of a
    // dictation long enough to tell).
    if edited > 30 || (a.len() >= 4 && edited * 2 > a.len()) {
        return vec![Local::Guarded("rewrite")];
    }
    // A leading dot stays: `.env` is a file name.
    let trim = |w: &str| {
        let w = w
            .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
            .trim_start_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '.');
        if w.chars().any(char::is_alphanumeric) {
            w.to_string()
        } else {
            String::new()
        }
    };
    let mut out = Vec::new();
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
            out.push(Local::Guarded("no_substitution"));
            continue;
        }
        let bounded = |t: Option<&(Tag, Option<usize>, Option<usize>)>| {
            t.is_none_or(|t| matches!(t.0, Tag::Same | Tag::Case))
        };
        let after_item = start.checked_sub(1).is_some_and(|s| ends_item(&tags[s]));
        if run.len() > 4 {
            out.push(Local::Guarded("too_long"));
            continue;
        }
        if !(after_item || bounded(start.checked_sub(1).and_then(|s| tags.get(s))))
            || !(ends_item(&tags[k - 1]) || bounded(tags.get(k)))
        {
            out.push(Local::Guarded("unbounded"));
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
            || term.chars().any(|c| c.is_ascii_digit() || c == '_')
            || term.starts_with('.');
        // A name may lengthen what was heard (Jordan → Jordane, E218), never
        // shorten it (E227). A common word's plural is grammar; another affix
        // (slope → slop, prêt → prête) is the validator's call.
        let affix = if looks_like_term {
            hk.starts_with(&tk) || hk.ends_with(&tk)
        } else {
            is_plural(&hk, &tk) || is_plural(&tk, &hk)
        };
        // EX-4 (L07): a name far from a single common word replaces that
        // word, it is not what the recognizer heard ("quelle" → "Kelmorine").
        let common_source = looks_like_term
            && !hk.contains(' ')
            && crate::lexicon::is_common(&hk)
            && distance(&hk, &tk) > 0.6;
        let guard = if term.is_empty() || heard.is_empty() {
            Some("no_substitution")
        } else if common_source {
            Some("common_source")
        } else if is_possessive_or_contraction(&hk, &tk) {
            Some("possessive")
        } else if affix {
            Some("affix")
        } else if looks_like_term {
            None
        } else if run.len() > 3 {
            Some("too_long")
        } else if has_meaning_word(&hk) || has_meaning_word(&tk) {
            Some("meaning_word")
        } else if is_grammar_change(&hk, &tk) {
            Some("grammar")
        } else {
            None
        };
        out.push(match guard {
            Some(reason) => Local::Guarded(reason),
            None => Local::Candidate(Candidate {
                term,
                heard,
                kind: if looks_like_term {
                    WordKind::Term
                } else {
                    WordKind::Misheard
                },
            }),
        });
    }
    // Names first (Wispr E76), 4 candidates at most.
    let (mut names, rest): (Vec<Local>, Vec<Local>) = out
        .into_iter()
        .partition(|l| matches!(l, Local::Candidate(c) if c.kind == WordKind::Term));
    names.extend(rest);
    let mut sent = 0;
    names.retain(|l| match l {
        Local::Candidate(_) => {
            sent += 1;
            sent <= 4
        }
        Local::Guarded(_) => true,
    });
    names
}

/// The candidates alone, guards logged by reason (never with their text).
#[cfg(test)]
pub fn candidates(inserted: &str, fin: &str) -> Vec<Candidate> {
    local_candidates(inserted, fin)
        .into_iter()
        .filter_map(|l| match l {
            Local::Candidate(c) => Some(c),
            Local::Guarded(reason) => {
                log::debug!("Learning: guarded ({reason})");
                None
            }
        })
        .collect()
}

/// A local verdict on one changed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Local {
    Candidate(Candidate),
    Guarded(&'static str),
}

/// `long` is `short` with a plural ending.
fn is_plural(short: &str, long: &str) -> bool {
    long.strip_prefix(short)
        .is_some_and(|d| matches!(d, "s" | "x" | "es"))
}

/// "John" → "John's", "do not" → "don't": not a word to learn (Wispr §9.2).
fn is_possessive_or_contraction(heard: &str, term: &str) -> bool {
    let (h, t) = (heard.replace('’', "'"), term.replace('’', "'"));
    let possessive = |a: &str, b: &str| b.strip_suffix("'s").or(b.strip_suffix('\'')) == Some(a);
    let words = |s: &str| s.split(' ').count();
    possessive(&h, &t)
        || possessive(&t, &h)
        || (h.contains('\'') != t.contains('\'') && words(&h) != words(&t))
}

/// A correction the layer may learn (F85–F87).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub term: String,
    pub heard: String,
    pub kind: WordKind,
}

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

/// A grammar fix, never a misheard word: two common words, one under 4 letters
/// (et/est, ces/ses, le/la; not guite/git or npn/npm), the same verb with
/// another ending, or a known pair.
fn is_grammar_change(heard: &str, term: &str) -> bool {
    let (h, t) = (heard.to_lowercase(), term.to_lowercase());
    let stem = |w: &str| {
        VERB_ENDINGS
            .iter()
            .find_map(|e| w.strip_suffix(e).filter(|s| !s.is_empty()))
            .unwrap_or(w)
            .to_string()
    };
    let short = |w: &str| letters(w).chars().count() < 4;
    ((short(&h) || short(&t)) && crate::lexicon::is_common(&h) && crate::lexicon::is_common(&t))
        || stem(&h) == stem(&t)
        || GRAMMAR_PAIRS
            .iter()
            .any(|&(a, b)| (h == a && t == b) || (h == b && t == a))
}

fn has_meaning_word(s: &str) -> bool {
    s.split_whitespace()
        .any(|w| MEANING_WORDS.contains(&w) || crate::adaptive::FR_COMMON_NAMES.contains(&w))
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
/// Counts one use of each term written in `kept`, the pasted text (Wispr
/// `frequencyUsed`, E78). Pending words still need a second correction.
/// `true` when a word changed.
pub(crate) fn mark_used(m: &mut Memory, spoken: &str, kept: &str, now: i64) -> bool {
    let text = unify(spoken).to_lowercase();
    let kept = unify(kept).to_lowercase();
    let mut changed = false;
    for w in m.words.iter_mut().filter(|w| w.active) {
        let said = std::iter::once(&w.term)
            .chain(&w.heard)
            .any(|f| contains_word(&text, &unify(f).to_lowercase()));
        if said {
            w.last_seen = now;
            changed = true;
        }
        if contains_word(&kept, &unify(&w.term).to_lowercase()) {
            w.used += 1;
            changed = true;
        }
    }
    changed
}

/// After a dictation: keeps the learned words the user said for 30 more
/// days and counts the ones the pasted text holds. `spoken` is the raw
/// transcription: a word the rewrite writes on its own must not keep itself
/// alive.
pub fn touch_used(app: &AppHandle, spoken: &str, kept: &str) {
    with_memory(app, |m| {
        if mark_used(m, spoken, kept, chrono::Utc::now().timestamp()) {
            let _ = save(app, m);
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
    Word { term: String, heard: String },
    Form { term: String, heard: String },
    Rule { rule: String },
}

impl LearnedNews {
    /// The text shown on the pill and in the notification.
    pub fn label(&self) -> &str {
        match self {
            Self::Word { term, .. } | Self::Form { term, .. } => term,
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
            if c.kind == WordKind::Term {
                l.kind = WordKind::Term;
            }
            let new_form = add_heard(l, c.heard.clone());
            let activated = !l.active && l.count >= 2;
            l.active |= activated;
            if activated {
                Some(LearnedNews::Word {
                    term: l.term.clone(),
                    heard: c.heard.clone(),
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
                used: 0,
                source: Source::UserEdits,
            });
            active.then(|| LearnedNews::Word {
                term: c.term.clone(),
                heard: c.heard.clone(),
            })
        }
    }
}

/// Undoes one heard form; a word left with none goes. The pair is refused
/// from now on.
pub(crate) fn drop_form(m: &mut Memory, term: &str, heard: &str) {
    refuse(m, heard, term);
    for w in m.words.iter_mut().filter(|w| same_term(&w.term, term)) {
        w.heard.retain(|h| !same_term(h, heard));
        // Its confirmation goes too: a word this one activated is pending again.
        w.count = w.count.saturating_sub(1);
        w.active = w.kind == WordKind::Term || w.count >= 2;
    }
    m.words.retain(|w| !w.heard.is_empty());
}

/// Deletes a word (the Undo of a new word, or the list's delete); its pairs are
/// refused so the same correction doesn't learn it back.
pub(crate) fn forget_word(m: &mut Memory, term: &str) {
    let heard: Vec<String> = m
        .words
        .iter()
        .filter(|w| w.term == term)
        .flat_map(|w| w.heard.clone())
        .collect();
    for h in heard {
        refuse(m, &h, term);
    }
    m.words.retain(|w| w.term != term);
}

fn refuse(m: &mut Memory, heard: &str, term: &str) {
    let pair = (heard.to_lowercase(), term.to_lowercase());
    if !m.refused.contains(&pair) {
        m.refused.push(pair);
    }
}

// ---------------------------------------------------------------- memory

static MEMORY: Mutex<Option<Memory>> = Mutex::new(None);
static MEMORY_UNSAFE_TO_SAVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn load_memory(path: &std::path::Path) -> Result<(Memory, Option<PathBuf>), String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((Memory::default(), None)),
        Err(e) => return Err(format!("Cannot read {}: {e}", path.display())),
    };
    if let Some(memory) = dpapi::run(&bytes, false).and_then(|b| parse_memory(&b).ok()) {
        return Ok((memory, None));
    }
    let backup = path.with_extension(format!(
        "bad-{}",
        chrono::Utc::now().format("%Y%m%d-%H%M%S%.f")
    ));
    std::fs::rename(path, &backup)
        .map_err(|e| format!("Cannot preserve {}: {e}", path.display()))?;
    Ok((Memory::default(), Some(backup)))
}

fn memory_path(app: &AppHandle) -> Option<PathBuf> {
    crate::portable::app_data_dir(app)
        .ok()
        .map(|d| d.join("learned.bin"))
}

#[cfg(windows)]
use crate::dpapi;

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
        let result = memory_path(app)
            .ok_or_else(|| "Cannot locate learned memory".to_string())
            .and_then(|p| load_memory(&p));
        match result {
            Ok((memory, backup)) => {
                if let Some(backup) = backup {
                    log::warn!(
                        "Unreadable learned memory preserved at {}",
                        backup.display()
                    );
                    crate::notices::raise(
                        app,
                        crate::notices::Notice::new(
                            crate::notices::NoticeCode::LearningMemoryFailed,
                            crate::notices::NoticeSource::Learning,
                        )
                        .with_detail(backup.display().to_string()),
                    );
                }
                memory
            }
            Err(detail) => {
                MEMORY_UNSAFE_TO_SAVE.store(true, Ordering::SeqCst);
                log::warn!("{detail}");
                crate::notices::raise(
                    app,
                    crate::notices::Notice::new(
                        crate::notices::NoticeCode::LearningMemoryFailed,
                        crate::notices::NoticeSource::Learning,
                    )
                    .with_detail(detail),
                );
                Memory::default()
            }
        }
    });
    // Expired words leave even when nothing is saved for a while.
    prune(entries, chrono::Utc::now().timestamp());
    f(entries)
}

fn save(app: &AppHandle, m: &mut Memory) -> Result<(), String> {
    let result = if MEMORY_UNSAFE_TO_SAVE.load(Ordering::SeqCst) {
        Err("Learned memory could not be safely loaded".to_string())
    } else {
        memory_path(app)
            .ok_or_else(|| "Cannot locate learned memory".to_string())
            .and_then(|path| save_memory(&path, m))
    };
    if let Err(detail) = &result {
        log::warn!("Learning: {detail}");
        crate::notices::raise(
            app,
            crate::notices::Notice::new(
                crate::notices::NoticeCode::LearningSaveFailed,
                crate::notices::NoticeSource::Learning,
            )
            .with_detail(detail.clone()),
        );
    } else {
        crate::notices::clear_matching(
            app,
            crate::notices::NoticeSource::Learning,
            Some(crate::notices::NoticeCode::LearningSaveFailed),
        );
    }
    result
}

fn save_memory(path: &std::path::Path, m: &mut Memory) -> Result<(), String> {
    prune(m, chrono::Utc::now().timestamp());
    let json = serde_json::to_vec(&*m).map_err(|e| e.to_string())?;
    let bytes =
        dpapi::run(&json, true).ok_or_else(|| "Couldn't encrypt learned memory".to_string())?;
    // Written aside then renamed, so a crash never leaves half a file (NF-12).
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The learned dictionary for the rewrite of `transcription` (≤ 80): the
/// words it sounds like with their heard forms, then every other active name.
pub fn terms_for(app: &AppHandle, transcription: &str) -> Vec<String> {
    let now = chrono::Utc::now().timestamp();
    with_memory(app, |m| {
        crate::recall::relevant_terms(&m.words, transcription, now, 80)
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

/// Applies the judge's style verdicts; returns the rules that became usable.
pub(crate) fn apply_verdicts(
    m: &mut Memory,
    verdicts: &[crate::judge::Verdict],
    now: i64,
) -> Vec<LearnedNews> {
    // A rule repeated within one reply is still one sighting.
    let mut counted_rules: Vec<usize> = Vec::new();
    use crate::judge::VerdictKind;
    let mut news = Vec::new();
    for v in verdicts {
        match v.kind {
            VerdictKind::StyleRule | VerdictKind::Filler => {
                // The judge's number, else the same text: a rule repeated
                // without `same_as` still counts as a second sighting.
                let same_text = v.rule.as_deref().map(str::trim).and_then(|t| {
                    m.rules
                        .iter()
                        .position(|r| r.rule.trim().eq_ignore_ascii_case(t))
                });
                let index = v.same_as.or(same_text);
                if index.is_some_and(|i| counted_rules.contains(&i)) {
                    continue;
                }
                match index.and_then(|i| m.rules.get_mut(i)) {
                    Some(r) => {
                        counted_rules.extend(index);
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
                        counted_rules.push(m.rules.len() - 1);
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
pub fn record_verdicts(app: &AppHandle, verdicts: &[crate::judge::Verdict], rules_sent: &[String]) {
    if verdicts.is_empty() {
        return;
    }
    let now = chrono::Utc::now().timestamp();
    let news = with_memory(app, |m| {
        let verdicts = remap_same_as(verdicts, rules_sent, &m.rules);
        let news = apply_verdicts(m, &verdicts, now);
        if save(app, m).is_ok() {
            news
        } else {
            Vec::new()
        }
    });
    if !news.is_empty() {
        log::info!("Learning: the judge made {} rule(s) usable", news.len());
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

/// Stores one validated candidate: active at once, pending a second
/// sighting, or nothing.
pub(crate) fn record_decision(
    m: &mut Memory,
    mut c: Candidate,
    d: &crate::validator::Decision,
    now: i64,
) -> Option<LearnedNews> {
    use crate::validator::Decision;
    // Undone while the validator was answering: stays undone.
    if memory_guard(m, &c) == Some("undone") {
        return None;
    }
    c.kind = match d {
        Decision::Active => WordKind::Term,
        Decision::Pending => WordKind::Misheard,
        Decision::Rejected(_) => return None,
    };
    log::info!(
        "Learned a spelling ({} chars, {:?})",
        c.term.chars().count(),
        d
    );
    record_word(m, &c, now)
}

/// EX-5: pending words sent once to the validator, one candidate per word
/// (its latest heard form). Empty once done.
pub(crate) fn pending_candidates(m: &Memory) -> Vec<Candidate> {
    if m.revalidated_at != 0 {
        return Vec::new();
    }
    m.words
        .iter()
        .filter(|w| !w.active)
        .filter_map(|w| {
            Some(Candidate {
                term: w.term.clone(),
                heard: w.heard.last()?.clone(),
                kind: w.kind,
            })
        })
        .collect()
}

/// The validator's reply on pending words: accepted → active, refused →
/// removed; a void or missing reply changes nothing (retried at next start).
pub(crate) fn apply_revalidation(
    m: &mut Memory,
    sent: &[Candidate],
    reply: Option<&str>,
    now: i64,
) -> Vec<LearnedNews> {
    let Some(accepted) = reply.and_then(|r| crate::validator::parse_reply(r, sent)) else {
        return Vec::new();
    };
    let mut news = Vec::new();
    let mut note_later = Vec::new();
    for c in sent {
        let ok = accepted.iter().any(|(a, _)| a == c);
        if let Some(w) = m
            .words
            .iter_mut()
            .find(|w| !w.active && same_term(&w.term, &c.term))
        {
            if ok {
                w.active = true;
                w.kind = WordKind::Term;
                note_later.push(Outcome::Learned);
                news.push(LearnedNews::Word {
                    term: w.term.clone(),
                    heard: c.heard.clone(),
                });
            } else if w.heard.last() == Some(&c.heard) {
                w.heard.clear();
                note_later.push(Outcome::Rejected("refused"));
            }
        }
    }
    for o in note_later {
        note(m, o, now);
    }
    m.words.retain(|w| !w.heard.is_empty());
    m.revalidated_at = now;
    news
}

/// EX-5: once after startup, the words still pending go to the validator.
pub async fn revalidate_pending(app: AppHandle) {
    let now = chrono::Utc::now().timestamp();
    let sent = with_memory(&app, |m| {
        let sent = pending_candidates(m);
        if sent.is_empty() && m.revalidated_at == 0 {
            m.revalidated_at = now;
            let _ = save(&app, m);
        }
        sent
    });
    if sent.is_empty() {
        return;
    }
    let reply = crate::validator::ask(&app, crate::validator::message("", "", &sent)).await;
    let news = with_memory(&app, |m| {
        let news = apply_revalidation(m, &sent, reply.as_deref(), now);
        if reply.is_some() {
            if save(&app, m).is_err() {
                return Vec::new();
            }
        }
        news
    });
    log::info!(
        "Learning: {} pending word(s) re-validated, {} made usable",
        sent.len(),
        news.len()
    );
    if !news.is_empty() {
        let _ = app.emit("learned-words", &news);
        announce(&app, &news);
    }
}

/// Stores the decided candidates and announces what became usable (F95:
/// once, grouped, with an undo).
fn record_decisions(app: &AppHandle, decided: Vec<(Candidate, crate::validator::Decision)>) {
    let now = chrono::Utc::now().timestamp();
    let news: Vec<LearnedNews> = with_memory(app, |m| {
        let news = decided
            .into_iter()
            .filter_map(|(c, d)| {
                use crate::validator::Decision;
                let o = match &d {
                    Decision::Active => Outcome::Learned,
                    Decision::Pending => Outcome::Pending,
                    Decision::Rejected(r) => Outcome::Rejected(r),
                };
                note(m, o, now);
                record_decision(m, c, &d, now)
            })
            .collect();
        if save(app, m).is_ok() {
            news
        } else {
            Vec::new()
        }
    });
    if !news.is_empty() {
        let _ = app.emit("learned-words", &news);
        announce(app, &news);
    }
}

/// What memory alone decides about a candidate: a pair the user undid, or a
/// A candidate memory doesn't need to ask about, or why not. A known word
/// corrected again is one more confirmation: without it, `replace_heard`
/// never applies a word the rewrite didn't fix (count + used stays at 1).
pub(crate) fn screen_candidate(
    m: &mut Memory,
    c: Candidate,
    now: i64,
) -> Result<Candidate, &'static str> {
    match memory_guard(m, &c) {
        Some("known") => {
            record_word(m, &c, now);
            Err("known")
        }
        Some(reason) => Err(reason),
        None => Ok(c),
    }
}

/// word already active with this heard form (nothing to ask).
pub(crate) fn memory_guard(m: &Memory, c: &Candidate) -> Option<&'static str> {
    let pair = (c.heard.to_lowercase(), c.term.to_lowercase());
    if m.refused.contains(&pair) {
        return Some("undone");
    }
    m.words
        .iter()
        .any(|w| {
            w.active
                && same_term(&w.term, &c.term)
                && w.heard.iter().any(|h| same_term(h, &c.heard))
        })
        .then_some("known")
}

/// F95, as Wispr: the overlay pill says what was learned, over the window the
/// user dictates into, with the system's notification sound.
/// Waits for no dictation to hold the pill; false if one still does.
fn pill_free(is_showing: impl Fn() -> bool, tries: u32, wait: Duration) -> bool {
    for _ in 0..tries {
        if !is_showing() {
            return true;
        }
        std::thread::sleep(wait);
    }
    !is_showing()
}

fn announce(app: &AppHandle, news: &[LearnedNews]) {
    use crate::notices::{Notice, NoticeCode, NoticeSource};
    let labels: Vec<&str> = news.iter().map(LearnedNews::label).collect();
    let (app, detail) = (app.clone(), labels.join(", "));
    std::thread::spawn(move || {
        // A dictation on the pill goes first, however long. Still there after
        // 5 min: the pill is left alone (the main window still shows the news).
        let showing = crate::overlay::is_session_showing;
        if !pill_free(showing, 1200, Duration::from_millis(250)) {
            return;
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

/// EX-7.2: what learning decided over the last 30 days.
#[tauri::command]
#[specta::specta]
pub fn get_learning_stats(app: AppHandle) -> LearningStats {
    with_memory(&app, |m| stats(m, chrono::Utc::now().timestamp()))
}

#[tauri::command]
#[specta::specta]
pub fn delete_learned_word(app: AppHandle, term: String) -> Result<(), String> {
    with_memory(&app, |m| {
        forget_word(m, &term);
        save(&app, m)
    })
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
pub fn delete_style_rule(app: AppHandle, rule: String) -> Result<(), String> {
    with_memory(&app, |m| {
        m.rules.retain(|r| r.rule != rule);
        save(&app, m)
    })
}

#[tauri::command]
#[specta::specta]
pub fn clear_learned_words(app: AppHandle) -> Result<(), String> {
    with_memory(&app, |m| {
        m.words.clear();
        save(&app, m)
    })
}

/// Clears the style rules only; the learned words stay.
#[tauri::command]
#[specta::specta]
pub fn clear_style_rules(app: AppHandle) -> Result<(), String> {
    with_memory(&app, |m| {
        m.rules.clear();
        save(&app, m)
    })
}

/// Undoes one heard form of a learned word (the notification's Undo).
#[tauri::command]
#[specta::specta]
pub fn remove_heard_form(app: AppHandle, term: String, heard: String) -> Result<(), String> {
    with_memory(&app, |m| {
        drop_form(m, &term, &heard);
        save(&app, m)
    })
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

/// A heard form that can't be an ordinary word: a capital after its first
/// char, a digit or `_` ("RandyFlow", "jf2"), or a word missing from the
/// lexicon ("corvanti"; never "cloud", EX-4).
fn no_real_word(form: &str) -> bool {
    form.chars().skip(1).any(char::is_uppercase)
        || form.chars().any(|c| c.is_ascii_digit() || c == '_')
        || form.split(' ').any(|w| !crate::lexicon::is_common(w))
}

/// Spec §6 rule 5, §9.4: when no rewrite ran, a confirmed name replaces its
/// heard forms, as whole words, case-insensitively.
/// Only forms that can't be a real word are replaced ("cloud" is left to the
/// rewrite's context check).
pub(crate) fn replace_heard(text: &str, words: &[Learned]) -> String {
    let mut out = text.to_string();
    for w in words
        .iter()
        .filter(|w| w.active && w.kind == WordKind::Term && w.count + w.used >= 2)
    {
        for h in w.heard.iter().filter(|h| no_real_word(h)) {
            // `\b` does not consume the separator: "RandyFlow RandyFlow" both match.
            let re = regex::Regex::new(&format!(r"(?i)\b{}\b", regex::escape(h)));
            if let Ok(re) = re {
                out = re.replace_all(&out, regex::NoExpand(&w.term)).into_owned();
            }
        }
    }
    out
}

/// [`replace_heard`] with the user's learned words.
pub fn apply_learned(app: &AppHandle, text: &str) -> String {
    with_memory(app, |m| replace_heard(text, &m.words))
}

/// Chars kept on each side of a terminal dictation as its anchors.
const TERMINAL_ANCHOR: usize = 200;

/// A terminal's caret offset doesn't map to the tail it is read from, so a
/// terminal is watched by its text alone.
fn can_observe(caret_known: bool, terminal: bool) -> bool {
    caret_known || terminal
}

/// The anchors of a terminal dictation in the tail of its buffer: up to 200
/// chars each side of its only occurrence. Twice or more (scrollback, output
/// printed since): `None` (spec R6, anchor_ambiguous).
pub fn locate_tail(text: &str, inserted: &str) -> Option<(String, String)> {
    let mut found = text.match_indices(inserted).map(|(i, _)| i);
    let i = found.next().filter(|_| !inserted.is_empty())?;
    if found.next().is_some() {
        return None;
    }
    let before: Vec<char> = text[..i].chars().collect();
    let before: String = before[before.len().saturating_sub(TERMINAL_ANCHOR)..]
        .iter()
        .collect();
    let after: String = text[i + inserted.len()..]
        .chars()
        .take(TERMINAL_ANCHOR)
        .collect();
    Some((before, after))
}

/// A terminal dictation's region in a tail read: after the only occurrence of
/// `before`, up to `after`, or `lines` lines when nothing followed it, so
/// output printed below never joins the region.
pub fn region_near<'a>(text: &'a str, before: &str, after: &str, lines: usize) -> Option<&'a str> {
    // Without a before-anchor, output scrolled into place could pass for the
    // dictation (spec R6): the region can't be trusted.
    if before.is_empty() {
        return None;
    }
    let start = {
        let mut at = text.match_indices(before).map(|(i, _)| i);
        let i = at.next()?;
        if at.next().is_some() {
            return None;
        }
        i + before.len()
    };
    let rest = &text[start..];
    let end = if after.is_empty() {
        rest.match_indices('\n')
            .nth(lines.max(1) - 1)
            .map_or(rest.len(), |(i, _)| i)
    } else {
        rest.find(after)?
    };
    Some(&rest[..end]).filter(|r| !r.trim().is_empty())
}

/// Why an observation ended (Wispr's end reasons, EX-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    Window,
    NextDictation,
    Emptied,
    AroundChanged,
    NotFound,
    Unreadable,
}

/// Whether an ended observation may teach anything: never when the text was
/// lost or unreadable, nor when the final reading failed (R7, uncertain);
/// after a change around it, only when the dictation is still intact.
pub fn learnable(end: End, final_read_ok: bool, intact: bool) -> bool {
    match end {
        End::NotFound | End::Unreadable => false,
        End::AroundChanged => final_read_ok && intact,
        _ => final_read_ok,
    }
}

/// The region to learn from once an observation ends. `final_read` is the
/// field read once more at the end event (`None`: that read failed);
/// `bounded`: the dictation had text on both sides; `region` cuts it out.
pub fn finish(
    last: Option<String>,
    final_read: Option<String>,
    bounded: bool,
    end: End,
    region: &dyn Fn(&str) -> Option<String>,
) -> Option<String> {
    // The final reading failed: uncertain, nothing learned (R7).
    let text = final_read?;
    // Sent or cleared: the last reading is what the user kept.
    if text.trim().is_empty() {
        return last.filter(|_| learnable(end, true, true));
    }
    match region(&text) {
        Some(r) => learnable(end, true, true).then_some(r),
        // Text changed around it (Wispr anchor_mismatch_transcript_intact).
        None => {
            let intact = bounded || last.as_ref().is_some_and(|l| text.contains(l.as_str()));
            let end = if end == End::NotFound || end == End::Unreadable {
                end
            } else {
                End::AroundChanged
            };
            last.filter(|_| learnable(end, true, intact))
        }
    }
}

/// F80–F83: watches the field the text was pasted into, then learns from the
/// last reading whose anchors still hold.
pub fn observe(
    app: &AppHandle,
    field: &crate::context::Field,
    inserted: &str,
    history_id: Option<i64>,
    app_name: String,
    terminal: bool,
) {
    if !crate::settings::get_settings(app).learning_enabled {
        return;
    }
    if !can_observe(field.caret_known, terminal) {
        log::debug!("Learning: not observing, the field's caret position is unknown");
        return;
    }
    let caret = field.before.chars().count();
    let inserted = unify(inserted.trim());
    let lines = inserted.lines().count().max(1);
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
            let cut = |text: &str, anchors: &Option<(String, String)>| {
                let (before, after) = anchors.as_ref()?;
                if terminal {
                    region_near(text, before, after, lines)
                } else {
                    region(text, before, after)
                }
                .map(|r| r.trim().to_string())
            };
            // A newer dictation ended this one: one immediate reading, then
            // never its field again.
            let mut end = End::Window;
            while started.elapsed() < WINDOW {
                if OBSERVATION.load(Ordering::SeqCst) != id {
                    end = End::NextDictation;
                    break;
                }
                match crate::context::read_watched(terminal).map(|t| unify(&t)) {
                    Some(text) if !text.trim().is_empty() => {
                        if anchors.is_none() {
                            anchors = if terminal {
                                locate_tail(&text, &inserted)
                            } else {
                                locate(&text, &inserted, caret)
                            };
                        }
                        if anchors.is_none() {
                            // The paste can take a moment to show up.
                            if started.elapsed() >= Duration::from_secs(3) {
                                end = End::NotFound;
                                log::debug!("Learning: {}", mismatch(&text, &inserted));
                                break;
                            }
                        } else {
                            match cut(&text, &anchors) {
                                Some(r) => last = Some(r),
                                // Text typed outside the dictation (Wispr: anchor_mismatch).
                                None => {
                                    end = End::AroundChanged;
                                    break;
                                }
                            }
                        }
                    }
                    Some(_) => {
                        end = End::Emptied;
                        break;
                    }
                    None => {
                        end = End::Unreadable;
                        break;
                    }
                }
                // Often enough to catch a fix made just before Enter sends it
                // (Wispr learns from the last reading before the send, E224).
                std::thread::sleep(Duration::from_millis(300));
            }
            // EX-3: one more reading at the end event, no 300 ms wait.
            let final_read = match end {
                End::NotFound | End::Unreadable => None,
                _ => crate::context::read_watched(terminal).map(|t| unify(&t)),
            };
            let bounded = anchors
                .as_ref()
                .is_some_and(|(b, a)| !b.trim().is_empty() && !a.trim().is_empty());
            let last = finish(last, final_read, bounded, end, &|t| cut(t, &anchors));
            let ended = format!("{end:?}");
            let edited = last.as_ref().filter(|fin| *fin != &inserted);
            let locals = edited
                .map(|fin| local_candidates(&inserted, fin))
                .unwrap_or_default();
            let now = chrono::Utc::now().timestamp();
            // Guards decide first: local rules, then what memory already says.
            let found: Vec<Candidate> = with_memory(&app, |m| {
                if edited.is_some() {
                    note(m, Outcome::Observed, now);
                }
                let mut found = Vec::new();
                for l in locals {
                    let reason = match l {
                        Local::Guarded(reason) => Err(reason),
                        Local::Candidate(c) => screen_candidate(m, c, now),
                    };
                    match reason {
                        Ok(c) => found.push(c),
                        Err(reason) => {
                            log::debug!("Learning: candidate → guarded ({reason})");
                            note(m, Outcome::Rejected(reason), now);
                        }
                    }
                }
                if edited.is_some() {
                    let _ = save(&app, m);
                }
                found
            });
            log::debug!(
                "Learning: observation ended ({ended}); edited: {}, candidates: {}",
                edited.is_some(),
                found.len()
            );
            if let Some(fin) = edited {
                // EX-2: the validator decides each candidate in the background.
                if !found.is_empty() {
                    let (app, inserted, fin) = (app.clone(), inserted.clone(), fin.clone());
                    tauri::async_runtime::spawn(async move {
                        let decided =
                            crate::validator::validate(&app, &inserted, &fin, found).await;
                        // Switched off while the validator was retried.
                        if crate::settings::get_settings(&app).learning_enabled {
                            record_decisions(&app, decided);
                        }
                    });
                }
                // The judge reads the correction for style rules and fillers.
                if crate::judge::judge_allowed(&crate::settings::get_settings(&app)) {
                    tauri::async_runtime::spawn(crate::judge::run(
                        app.clone(),
                        inserted.clone(),
                        fin.clone(),
                        app_name.clone(),
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

    #[test]
    fn save_memory_reports_write_failure() {
        let path =
            std::env::temp_dir().join(format!("handy-missing-{}/learned.bin", std::process::id()));
        let error = save_memory(&path, &mut Memory::default()).unwrap_err();
        assert!(error.contains(&path.to_string_lossy().to_string()));
    }

    #[test]
    fn unreadable_memory_is_preserved_before_saving() {
        let dir = std::env::temp_dir().join(format!("handy-memory-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("learned.bin");
        std::fs::write(&path, b"invalid memory").unwrap();
        let (memory, backup) = load_memory(&path).unwrap();
        assert!(backup.is_some());
        std::fs::write(&path, serde_json::to_vec(&memory).unwrap()).unwrap();
        assert!(std::fs::read_dir(&dir).unwrap().any(|entry| {
            let entry = entry.unwrap();
            entry.path() != path && std::fs::read(entry.path()).unwrap() == b"invalid memory"
        }));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The region between two anchors, as `observe` reads it.
    fn between<'a>(b: &'a str, a: &'a str) -> impl Fn(&str) -> Option<String> + 'a {
        move |t: &str| region(t, b, a).map(|r| r.trim().to_string())
    }

    #[test]
    fn learning_stats_count_the_last_30_days() {
        let day = 86_400;
        let mut m = Memory::default();
        note(&mut m, Outcome::Observed, 0);
        note(&mut m, Outcome::Observed, 40 * day);
        note(&mut m, Outcome::Learned, 40 * day);
        note(&mut m, Outcome::Pending, 41 * day);
        note(&mut m, Outcome::Rejected("grammar"), 41 * day);
        note(&mut m, Outcome::Rejected("refused"), 42 * day);
        note(&mut m, Outcome::Rejected("grammar"), 42 * day);
        let s = stats(&m, 45 * day);
        assert_eq!((s.observed, s.learned, s.pending, s.rejected), (1, 1, 1, 3));
        assert_eq!(
            s.rejected_by,
            vec![("grammar".to_string(), 2), ("refused".to_string(), 1)]
        );
        // Older than 30 days: dropped from storage too.
        assert_eq!(m.events.len(), 6);
    }

    #[test]
    fn short_technical_terms_are_not_grammar() {
        assert!(!is_grammar_change("guite", "git"));
        assert!(!is_grammar_change("NPN", "npm"));
        // Both common short words: still grammar.
        assert!(is_grammar_change("et", "est"));
        assert!(is_grammar_change("ces", "ses"));
    }

    #[test]
    fn a_known_word_corrected_again_is_applied_without_the_rewrite() {
        let mut m = Memory::default();
        let c = super::candidates("Merci Bérénisse, à demain.", "Merci Bérénice, à demain.");
        let mut term = c[0].clone();
        term.kind = WordKind::Term;
        record_word(&mut m, &term, 0);
        assert_eq!(replace_heard("Bérénisse", &m.words), "Bérénisse");
        // Corrected again: memory knows it, but it is one more confirmation.
        assert!(screen_candidate(&mut m, term, 1).is_err());
        assert_eq!(replace_heard("Bérénisse", &m.words), "Bérénice");
    }

    #[test]
    fn learned_news_never_takes_the_pill_from_a_dictation() {
        let none = Duration::ZERO;
        assert!(pill_free(|| false, 3, none));
        // A dictation still on the pill after the wait: the pill is left alone.
        assert!(!pill_free(|| true, 3, none));
    }

    #[test]
    fn undoing_an_activation_keeps_the_forms_learned_before() {
        let c = |heard: &str| Candidate {
            term: "Bérénice".into(),
            heard: heard.into(),
            kind: WordKind::Misheard,
        };
        let mut m = Memory::default();
        assert_eq!(record_word(&mut m, &c("Bérénisse"), 0), None);
        // The second correction activates the word: its news names that form.
        let news = record_word(&mut m, &c("Bérénise"), 1);
        let Some(LearnedNews::Word { term, heard }) = news else {
            panic!("{news:?}");
        };
        // The Undo drops this pair only, and the word is pending again.
        drop_form(&mut m, &term, &heard);
        assert_eq!(m.words[0].heard, vec!["Bérénisse"]);
        assert!(!m.words[0].active);
    }

    #[test]
    fn undoing_a_new_word_refuses_its_pairs() {
        let mut m = Memory::default();
        let c = super::candidates("Merci Jeff, à demain.", "Merci Jev, à demain.");
        record_word(&mut m, &c[0], 0);
        forget_word(&mut m, "Jev");
        assert!(m.words.is_empty());
        assert_eq!(memory_guard(&m, &c[0]), Some("undone"));
    }

    #[test]
    fn an_undone_pair_is_never_learned_again() {
        let mut m = Memory::default();
        let c = super::candidates("Merci Jeff, à demain.", "Merci Jev, à demain.");
        assert_eq!(memory_guard(&m, &c[0]), None);
        record_word(&mut m, &c[0], 0);
        // Known and active with this form: nothing to ask.
        assert_eq!(memory_guard(&m, &c[0]), Some("known"));
        // The notification's Undo: gone, and refused from now on.
        drop_form(&mut m, "Jev", "Jeff");
        assert!(m.words.is_empty());
        assert_eq!(memory_guard(&m, &c[0]), Some("undone"));
        // Another form of the same word is still welcome.
        let other = Candidate {
            term: "Jev".into(),
            heard: "Chef".into(),
            kind: WordKind::Term,
        };
        assert_eq!(memory_guard(&m, &other), None);
        // Survives a save.
        let back = parse_memory(&serde_json::to_vec(&m).unwrap()).unwrap();
        assert_eq!(memory_guard(&back, &c[0]), Some("undone"));
        // An answer still in flight can't bring it back.
        let d = crate::validator::Decision::Active;
        assert!(record_decision(&mut m, c[0].clone(), &d, 2).is_none());
        assert!(m.words.is_empty());
    }

    #[test]
    fn entries_say_where_they_come_from() {
        let mut m = Memory::default();
        record_word(
            &mut m,
            &super::candidates("Merci Jeff.", "Merci Jev.")[0],
            0,
        );
        assert_eq!(m.words[0].source, Source::UserEdits);
        // Older memory has no source: the user's edits.
        let l: Learned =
            serde_json::from_str(r#"{"term":"Qwen","heard":"Quen","count":1,"last_seen":0}"#)
                .unwrap();
        assert_eq!(l.source, Source::UserEdits);
    }

    #[test]
    fn pending_words_are_revalidated_once() {
        let pending = |term: &str, heard: &str| Learned {
            term: term.into(),
            heard: vec![heard.into()],
            count: 1,
            last_seen: 0,
            kind: WordKind::Misheard,
            active: false,
            used: 0,
            source: Default::default(),
        };
        let mut m = Memory {
            words: vec![
                pending("specs", "spikes"),
                pending("slop", "slope"),
                pending("devra correspondre", "devrait correspondre"),
            ],
            ..Default::default()
        };
        let sent = pending_candidates(&m);
        assert_eq!(sent.len(), 3);
        // Void or unreachable: nothing changes, retried at the next start.
        assert!(apply_revalidation(&mut m, &sent, None, 5).is_empty());
        assert!(apply_revalidation(&mut m, &sent, Some("nope"), 5).is_empty());
        assert_eq!((m.words.len(), m.revalidated_at), (3, 0));
        let reply = r#"{"accepted":[{"heard":"spikes","term":"specs","kind":"technical"},
            {"heard":"slope","term":"slop","kind":"technical"}]}"#;
        let news = apply_revalidation(&mut m, &sent, Some(reply), 5);
        assert_eq!(news.len(), 2);
        let terms: Vec<(&str, bool, WordKind)> = m
            .words
            .iter()
            .map(|w| (w.term.as_str(), w.active, w.kind))
            .collect();
        assert_eq!(
            terms,
            vec![
                ("specs", true, WordKind::Term),
                ("slop", true, WordKind::Term)
            ]
        );
        assert_eq!(m.revalidated_at, 5);
        // Done once: nothing is sent again.
        assert!(pending_candidates(&m).is_empty());
    }

    #[test]
    fn only_a_certain_end_is_learnable() {
        assert!(learnable(End::Window, true, false));
        assert!(learnable(End::NextDictation, true, false));
        assert!(learnable(End::Emptied, true, false));
        assert!(!learnable(End::Window, false, true), "final reading failed");
        assert!(!learnable(End::AroundChanged, true, false));
        assert!(learnable(End::AroundChanged, true, true));
        assert!(!learnable(End::NotFound, true, true));
        assert!(!learnable(End::Unreadable, true, true));
    }

    #[test]
    fn an_observation_ends_with_a_final_reading() {
        let r = between("Salut ", " !");
        let last = || Some("Jef".to_string());
        // O01: the window closes; the final reading holds a later fix.
        assert_eq!(
            finish(last(), Some("Salut Jev !".into()), true, End::Window, &r),
            Some("Jev".into())
        );
        // O02 / L08: fixed then sent at once; a final reading that still holds
        // the fix wins over the last poll.
        assert_eq!(
            finish(last(), Some("Salut Jev !".into()), true, End::Emptied, &r),
            Some("Jev".into())
        );
        // Sent: the field is empty now, the last reading is what was sent.
        assert_eq!(
            finish(last(), Some(String::new()), true, End::Emptied, &r),
            last()
        );
        // O03: a newer dictation ends it; one immediate reading.
        assert_eq!(
            finish(
                last(),
                Some("Salut Jev !".into()),
                true,
                End::NextDictation,
                &r
            ),
            Some("Jev".into())
        );
        // The final reading failed: uncertain, nothing learned (R7).
        assert_eq!(finish(last(), None, true, End::Window, &r), None);
        // O06: text typed around it, anchors bounded on both sides: last reading.
        assert_eq!(
            finish(
                last(),
                Some("Bon. Salut Jef ! Ok".into()),
                true,
                End::AroundChanged,
                &r
            ),
            last()
        );
        // Unbounded region, but the pasted text is still there unchanged: last reading.
        assert_eq!(
            finish(
                last(),
                Some("Bon. Jef ! Ok".into()),
                false,
                End::AroundChanged,
                &r
            ),
            last()
        );
        // Unbounded and gone: nothing.
        assert_eq!(
            finish(
                last(),
                Some("Tout autre chose".into()),
                false,
                End::AroundChanged,
                &r
            ),
            None
        );
        // O08: never found, or no longer readable: nothing.
        assert_eq!(
            finish(last(), Some("Salut Jev !".into()), true, End::NotFound, &r),
            None
        );
        assert_eq!(
            finish(
                last(),
                Some("Salut Jev !".into()),
                true,
                End::Unreadable,
                &r
            ),
            None
        );
    }

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
    fn a_confirmed_term_replaces_a_heard_form_that_is_no_real_word() {
        let words = [word("Handy Flow", "RandyFlow", 2, 0, WordKind::Term, true)];
        assert_eq!(
            replace_heard("Ouvre RandyFlow.", &words),
            "Ouvre Handy Flow."
        );
        assert_eq!(replace_heard("ouvre randyflow", &words), "ouvre Handy Flow");
    }

    #[test]
    fn a_heard_form_that_is_a_real_word_is_never_replaced_blindly() {
        // Without the LLM there is no context check: "cloud", "Jeff" stay.
        let words = [
            word("Claude", "Cloud", 5, 0, WordKind::Term, true),
            word("Jev", "Jeff", 5, 0, WordKind::Term, true),
        ];
        assert_eq!(
            replace_heard("On héberge ça dans le cloud. Merci Jeff.", &words),
            "On héberge ça dans le cloud. Merci Jeff."
        );
    }

    #[test]
    fn pending_misheard_or_once_seen_words_never_replace() {
        let words = [
            word("Handy Flow", "RandyFlow", 1, 0, WordKind::Term, true),
            word("SuperApp", "SuprApp2", 3, 0, WordKind::Misheard, true),
            word("Broker Name", "BrokerName", 3, 0, WordKind::Term, false),
        ];
        let t = "RandyFlow SuprApp2 BrokerName";
        assert_eq!(replace_heard(t, &words), t);
    }

    #[test]
    fn only_whole_words_are_replaced() {
        let words = [word("Handy Flow", "RandyFlow", 2, 0, WordKind::Term, true)];
        assert_eq!(replace_heard("RandyFlows", &words), "RandyFlows");
    }

    #[test]
    fn a_terminal_region_is_found_near_its_anchors_while_output_scrolls() {
        let before = "PS C:\\> claude\n> ";
        let after = "\n  ? for shortcuts";
        let field = format!("old output\nmore output\n{before}merci Jev{after}");
        assert_eq!(region_near(&field, before, after, 1), Some("merci Jev"));
    }

    #[test]
    fn an_ambiguous_terminal_anchor_ends_the_observation() {
        assert_eq!(region_near("> a\n> merci Jev\n?", "> ", "\n?", 1), None);
    }

    #[test]
    fn an_empty_after_anchor_takes_as_many_lines_as_were_pasted() {
        assert_eq!(
            region_near("out\n> merci Jev", "out\n> ", "", 1),
            Some("merci Jev")
        );
        // Output printed after the dictation is never part of the region.
        assert_eq!(
            region_near("out\n> merci Jev\nCompiling handy", "out\n> ", "", 1),
            Some("merci Jev")
        );
    }

    #[test]
    fn an_empty_before_anchor_never_makes_a_terminal_region() {
        // Pasted at the very start of the read: scrolled output could take its
        // place under the same after-anchor, so the region can't be trusted.
        assert_eq!(
            region_near(
                "merci Jev
footer",
                "",
                "
footer",
                1
            ),
            None
        );
    }

    #[test]
    fn adjacent_heard_forms_are_all_replaced() {
        let words = [word("Handy Flow", "RandyFlow", 2, 0, WordKind::Term, true)];
        assert_eq!(
            replace_heard("RandyFlow RandyFlow", &words),
            "Handy Flow Handy Flow"
        );
    }

    #[test]
    fn two_empty_anchors_never_make_a_region() {
        assert_eq!(region_near("anything printed", "", "", 1), None);
    }

    #[test]
    fn the_terminal_dictation_must_appear_once_in_the_tail() {
        let (b, a) = locate_tail("out\n> merci Jeff", "merci Jeff").unwrap();
        assert_eq!((b.as_str(), a.as_str()), ("out\n> ", ""));
        // Twice (scrollback, or output printed after the paste): abandon, spec R6.
        assert_eq!(
            locate_tail("> merci Jeff\nok\n> merci Jeff", "merci Jeff"),
            None
        );
        // Anchors keep at most 200 chars on each side.
        let long = format!("{}> merci Jeff", "x".repeat(500));
        assert_eq!(
            locate_tail(&long, "merci Jeff").unwrap().0.chars().count(),
            200
        );
    }

    #[test]
    fn a_terminal_is_observed_without_a_known_caret() {
        assert!(can_observe(false, true));
        assert!(!can_observe(false, false));
        assert!(can_observe(true, false));
    }

    const A05_PASTED: &str =
        "Nous allons organiser une réunion la semaine prochaine pour parler du budget.";
    const A05_EDITED: &str =
        "Finalement on annule tout et on reprend le dossier en janvier avec une autre equipe.";

    #[test]
    fn golden_alignment_cases() {
        let pair = |t: &str, h: &str| vec![(t.to_string(), h.to_string())];
        assert_eq!(
            candidates(
                "Nous installons Corvanti ce soir.",
                "Nous installons Quorvantis ce soir."
            ),
            pair("Quorvantis", "Corvanti")
        ); // A01
        assert_eq!(
            candidates(
                "Le dossier est suivi par Aurélie Kenen.",
                "Le dossier est suivi par Aurélie Quennehen."
            ),
            pair("Quennehen", "Kenen")
        ); // A03
        assert!(super::candidates("Je pars à Paris demain.", "Je pars à PARIS demain.").is_empty()); // A04
        assert!(super::candidates(A05_PASTED, A05_EDITED).is_empty()); // A05
    }

    #[test]
    fn golden_a02_a_two_word_name_is_learned_as_one_phrase() {
        // Handy keeps phrase mode on (spec §9.2.0), unlike this Wispr account.
        assert_eq!(
            candidates(
                "Je travaille avec Kazimierz Bignowski sur le projet.",
                "Je travaille avec Kasimir Zbignowski sur le projet."
            ),
            vec![(
                "Kasimir Zbignowski".to_string(),
                "Kazimierz Bignowski".to_string()
            )]
        );
    }

    #[test]
    fn a_one_word_dictation_can_be_corrected() {
        // Internal capital: a term even at the start of the field.
        assert_eq!(
            candidates("Postgre", "PostgreSQL"),
            vec![("PostgreSQL".to_string(), "Postgre".to_string())]
        );
    }

    #[test]
    fn typing_after_the_dictation_is_not_a_rewrite() {
        assert_eq!(
            candidates(
                "Merci Jeff.",
                "Merci Jev. Et voici une longue suite tapée ensuite au clavier"
            ),
            vec![("Jev".to_string(), "Jeff".to_string())]
        );
    }

    #[test]
    fn a_name_may_extend_what_was_heard_but_not_truncate_it() {
        // L01 (E218): Wispr learns Jordane←Jordan. E227: a truncated name is refused.
        assert_eq!(
            candidates("Merci Jordan, à demain.", "Merci Jordane, à demain."),
            vec![("Jordane".to_string(), "Jordan".to_string())]
        );
        assert!(
            super::candidates("Merci Jordane, à demain.", "Merci Jordan, à demain.").is_empty()
        );
        // A common word keeps the plural/conjugation guard (L05).
        assert!(super::candidates("Le projet avance bien.", "Le projets avance bien.").is_empty());
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
            used: 0,
            source: Default::default(),
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
            used: 0,
            source: Default::default(),
        });
        assert!(mark_used(&mut m, "on compare quen et gemma", "", 500));
        assert_eq!(m.words[0].last_seen, 500);
        assert!(!mark_used(&mut m, "on compare queneau", "", 900)); // not a whole word
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
            heard: "Quen".into(),
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
    fn a_style_rule_needs_two_sightings() {
        use crate::judge::{Verdict, VerdictKind};
        let rule = "Toujours vouvoyer.";
        let v = |same_as| Verdict {
            kind: VerdictKind::StyleRule,
            rule: Some(rule.into()),
            same_as,
            consistent_with_history: true,
        };
        let mut m = Memory::default();
        assert!(apply_verdicts(&mut m, &[v(None)], 0).is_empty());
        assert!(!m.rules[0].active);
        let news = apply_verdicts(&mut m, &[v(Some(0))], 1);
        assert!(m.rules[0].active);
        assert_eq!(news, vec![LearnedNews::Rule { rule: rule.into() }]);
    }

    #[test]
    fn only_a_plural_or_a_shortened_name_is_guarded_as_an_affix() {
        // A plural stays local (L05); a name is never shortened (E227).
        assert!(super::candidates("Le projet avance bien.", "Le projets avance bien.").is_empty());
        assert!(
            super::candidates("Merci Jordane, à demain.", "Merci Jordan, à demain.").is_empty()
        );
        // Another shortened common word goes to the validator.
        let c = super::candidates("C'est du slope pur.", "C'est du slop pur.");
        assert_eq!(c.len(), 1);
        assert_eq!((c[0].heard.as_str(), c[0].term.as_str()), ("slope", "slop"));
    }

    #[test]
    fn a_short_technical_word_is_a_term_candidate() {
        // "vu" → "Vue": short, but a term for the validator, never a grammar fix.
        let c = super::candidates("on code en vu", "on code en Vue");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].kind, WordKind::Term);
    }

    #[test]
    fn a_repeated_rule_counts_even_without_same_as() {
        use crate::judge::{Verdict, VerdictKind};
        let v = Verdict {
            kind: VerdictKind::StyleRule,
            rule: Some("Toujours vouvoyer.".into()),
            same_as: None,
            consistent_with_history: false,
        };
        let mut m = Memory::default();
        apply_verdicts(&mut m, &[v.clone()], 0);
        apply_verdicts(&mut m, &[v], 1);
        assert_eq!(m.rules.len(), 1);
        assert!(m.rules[0].active);
    }

    #[test]
    fn one_reply_counts_a_rule_once() {
        use crate::judge::{Verdict, VerdictKind};
        let v = Verdict {
            kind: VerdictKind::StyleRule,
            rule: Some("Toujours vouvoyer.".into()),
            same_as: None,
            consistent_with_history: false,
        };
        let mut m = Memory::default();
        apply_verdicts(&mut m, &[v.clone(), v], 0);
        assert_eq!((m.rules.len(), m.rules[0].count), (1, 1));
        assert!(!m.rules[0].active);
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
            rule: text.map(Into::into),
            same_as,
            consistent_with_history: true,
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
        apply_verdicts(&mut m, &out, 1);
        let rules: Vec<_> = m.rules.iter().map(|r| r.rule.as_str()).collect();
        assert_eq!(rules, vec!["B", "C"], "the deleted rule never comes back");
        assert_eq!(m.rules[0].count, 2);
    }

    #[test]
    fn verdicts_respect_the_guards() {
        use crate::judge::{Verdict, VerdictKind};
        let v = |kind, rule: Option<&str>, same_as, ok| Verdict {
            kind,
            rule: rule.map(Into::into),
            same_as,
            consistent_with_history: ok,
        };
        let mut m = Memory::default();
        // A meaning change learns nothing.
        apply_verdicts(
            &mut m,
            &[v(VerdictKind::ContentChange, None, None, true)],
            0,
        );
        assert!(m.words.is_empty() && m.rules.is_empty());
        // A lone rule not backed by history waits for a second sighting.
        let rule = "Préférer « de manière claire et concise ».";
        apply_verdicts(
            &mut m,
            &[v(VerdictKind::StyleRule, Some(rule), None, false)],
            0,
        );
        assert!(!m.rules[0].active);
        let on = apply_verdicts(
            &mut m,
            &[v(VerdictKind::StyleRule, Some("…"), Some(0), false)],
            1,
        );
        assert!(m.rules[0].active && m.rules[0].count == 2);
        assert_eq!(on, vec![LearnedNews::Rule { rule: rule.into() }]);
        // Even "consistent with history", a new rule waits for a second sighting.
        let filler = v(
            VerdictKind::Filler,
            Some("Supprimer « donc voilà »."),
            None,
            true,
        );
        apply_verdicts(&mut m, &[filler], 2);
        assert!(!m.rules[1].active);
        // A `same_as` out of range is a new rule, not a crash.
        apply_verdicts(
            &mut m,
            &[v(VerdictKind::StyleRule, Some("Autre."), Some(99), false)],
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
            used: 0,
            source: Default::default(),
        }
    }

    #[test]
    fn old_memory_array_still_loads() {
        let m = parse_memory(br#"[{"term":"Claude","heard":["Cloud"],"count":2,"last_seen":0}]"#)
            .unwrap();
        assert_eq!(m.words.len(), 1);
        assert_eq!(m.words[0].kind, WordKind::Term);
        assert!(m.words[0].active);
        assert!(m.rules.is_empty());
        // The new format round-trips.
        let json = serde_json::to_vec(&m).unwrap();
        assert_eq!(parse_memory(&json).unwrap().words, m.words);
    }

    #[test]
    fn a_word_fixed_twice_in_one_dictation_is_learned() {
        // "chefo" → "jevu" is far (0.8); with "jeffu" → "jevu" it is a second sighting.
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
        assert_eq!(m.words[0].heard, vec!["chefo", "jeffu"]);
        // Alone, "chefo" → "jevu" (0.8) goes to the validator; without one it waits.
        let alone = super::candidates("le chefo est prêt", "le jevu est prêt");
        assert_eq!(
            crate::validator::fallback(&alone[0]),
            crate::validator::Decision::Pending
        );
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
        // A pending word unused for 30 days goes; an active one stays (Wispr).
        assert_eq!(
            m.words.iter().map(|w| w.term.as_str()).collect::<Vec<_>>(),
            vec!["Qwen", "Kubernetes"]
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
            "",
            9
        ));
        let seen = |t: &str| m.words.iter().find(|w| w.term == t).unwrap().last_seen;
        assert_eq!(seen("Qwen"), 9);
        assert_eq!(seen("Node.js"), 9);
        // Pending: dictating it proves nothing. Inside "partir": not the word.
        assert_eq!(seen("config"), 0);
        assert_eq!(seen("art"), 0);
        assert!(!mark_used(&mut m, "rien ici", "", 10));
    }

    #[test]
    fn an_active_word_survives_a_year_without_use() {
        let mut m = Memory::default();
        m.words
            .push(word("Jev", "Jeff", 5, 0, WordKind::Term, true));
        prune(&mut m, 365 * 86_400);
        assert!(m.words.iter().any(|w| w.term == "Jev"));
    }

    #[test]
    fn a_pending_word_still_expires_after_30_days() {
        let mut m = Memory::default();
        m.words
            .push(word("slop", "slope", 1, 0, WordKind::Misheard, false));
        prune(&mut m, 31 * 86_400);
        assert!(m.words.is_empty());
    }

    #[test]
    fn the_term_in_the_pasted_text_counts_one_use() {
        let mut m = Memory::default();
        m.words
            .push(word("Jev", "Jeff", 5, 0, WordKind::Term, true));
        assert!(mark_used(&mut m, "merci jeff", "Merci Jev.", 7));
        assert_eq!(m.words[0].used, 1);
    }

    #[test]
    fn a_heard_form_alone_refreshes_but_does_not_count() {
        let mut m = Memory::default();
        m.words
            .push(word("Jev", "Jeff", 5, 0, WordKind::Term, true));
        assert!(mark_used(&mut m, "merci jeff", "Merci Jeff.", 7));
        assert_eq!((m.words[0].last_seen, m.words[0].used), (7, 0));
    }
}
