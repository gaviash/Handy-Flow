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
        .or_else(|_| serde_json::from_slice::<Vec<Learned>>(json).map(|words| Memory { words, rules: Vec::new() }))
        .unwrap_or_default()
}

/// Drops pending entries unseen for 30 days, then the least used and oldest
/// entries over the caps.
pub(crate) fn prune(m: &mut Memory, now: i64) {
    let expired = now - PENDING_DAYS * 86_400;
    m.words.retain(|w| w.active || w.last_seen >= expired);
    m.rules.retain(|r| r.active || r.last_seen >= expired);
    if m.words.len() > WORDS_MAX {
        m.words.sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
        m.words.truncate(WORDS_MAX);
    }
    if m.rules.len() > RULES_MAX {
        m.rules.sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
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
        let kind = if term.is_empty() {
            Err("no_substitution")
        } else if affix {
            Err("affix")
        } else if looks_like_term {
            if distance(&hk, &tk) <= 0.6 {
                Ok(WordKind::Term)
            } else {
                Err("too_far")
            }
        } else if run.len() > 3 {
            Err("too_long")
        } else if has_meaning_word(&hk) || has_meaning_word(&tk) {
            Err("meaning_word")
        } else if distance(&hk, &tk) <= MISHEARD_DISTANCE {
            Ok(WordKind::Misheard)
        } else {
            Err("too_far")
        };
        match kind {
            Ok(kind) => out.push(Candidate { term, heard, kind }),
            Err(reason) => reject(reason),
        }
    }
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

/// Words whose swap changes the meaning ("mardi" → "jeudi", "oui" → "non"):
/// never learned as misheard, however close they sound.
const MEANING_WORDS: &[&str] = &[
    // numbers
    "zéro", "zero", "un", "une", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf",
    "dix", "onze", "douze", "treize", "quatorze", "quinze", "seize", "vingt", "trente", "cent",
    "mille", "one", "two", "three", "four", "five", "seven", "eight", "nine", "ten", "eleven",
    "twelve", "twenty", "hundred", "thousand",
    // yes / no / negation
    "oui", "non", "si", "yes", "no", "ne", "pas", "jamais", "rien", "plus", "not", "never",
    // personal pronouns
    "je", "tu", "il", "elle", "on", "nous", "vous", "ils", "elles", "moi", "toi", "lui", "eux",
    "i", "you", "he", "she", "we", "they", "me", "him", "her", "us", "them",
    // English days and months (French ones come from adaptive)
    "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday", "january",
    "february", "march", "april", "may", "june", "july", "august", "september", "october",
    "november", "december",
];

fn has_meaning_word(s: &str) -> bool {
    s.split_whitespace().any(|w| {
        MEANING_WORDS.contains(&w) || crate::adaptive::FR_COMMON_NAMES.contains(&w)
    })
}

/// Whether `heard` → `term` can be a misheard common word (also checked on
/// what the LLM judge proposes).
pub fn is_misheard_pair(heard: &str, term: &str) -> bool {
    let (h, t) = (heard.to_lowercase(), term.to_lowercase());
    !h.is_empty()
        && !t.is_empty()
        && h != t
        && !has_meaning_word(&h)
        && !has_meaning_word(&t)
        && distance(&h, &t) <= MISHEARD_DISTANCE
}

/// Records a sighting; `true` when the word becomes usable or gains a form
/// (news to announce). A misheard word is pending until seen twice.
pub(crate) fn record_word(m: &mut Memory, c: &Candidate, now: i64) -> bool {
    match m.words.iter_mut().find(|l| l.term == c.term) {
        Some(l) => {
            l.count += 1;
            l.last_seen = now;
            let new_form = add_heard(l, c.heard.clone());
            let activated = !l.active && l.count >= 2;
            l.active |= activated;
            activated || (l.active && new_form)
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
            active
        }
    }
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
            .and_then(|b| Some(parse_memory(&b)))
            .unwrap_or_default()
    });
    f(entries)
}

fn save(app: &AppHandle, m: &mut Memory) {
    let (Some(path), Ok(json)) = (memory_path(app), { prune(m, chrono::Utc::now().timestamp()); serde_json::to_vec(&*m) }) else {
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
    with_memory(app, |m| crate::recall::relevant_terms(&m.words, transcription, 40))
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
    _app_name: &str,
    now: i64,
) -> Vec<String> {
    use crate::judge::VerdictKind;
    let mut news = Vec::new();
    for v in verdicts {
        match v.kind {
            VerdictKind::Misheard => {
                let (Some(heard), Some(term)) = (v.heard.as_deref(), v.term.as_deref()) else {
                    continue;
                };
                let (heard, term) = (heard.trim(), term.trim());
                if !is_misheard_pair(heard, term) {
                    log::debug!("Learning: judge item rejected (not a misheard pair)");
                    continue;
                }
                let c = Candidate {
                    term: term.into(),
                    heard: heard.into(),
                    kind: WordKind::Misheard,
                };
                if record_word(m, &c, now) {
                    news.push(c.term);
                }
            }
            VerdictKind::StyleRule | VerdictKind::Filler => {
                let existing = v.same_as.and_then(|i| m.rules.get_mut(i));
                match existing {
                    Some(r) => {
                        r.count += 1;
                        r.last_seen = now;
                        if !r.active && (r.count >= 2 || v.consistent_with_history) {
                            r.active = true;
                            news.push(r.rule.clone());
                        }
                    }
                    None => {
                        let Some(rule) = v.rule.as_deref().map(str::trim).filter(|r| !r.is_empty())
                        else {
                            continue;
                        };
                        let active = v.consistent_with_history;
                        m.rules.push(StyleRule {
                            rule: rule.into(),
                            count: 1,
                            active,
                            last_seen: now,
                            apps: Vec::new(),
                        });
                        if active {
                            news.push(rule.into());
                        }
                    }
                }
            }
            VerdictKind::ContentChange | VerdictKind::Unclear => {}
        }
    }
    news
}

/// Stores the judge's verdicts and announces what became usable.
pub fn record_verdicts(app: &AppHandle, verdicts: &[crate::judge::Verdict], app_name: &str) {
    if verdicts.is_empty() {
        return;
    }
    let now = chrono::Utc::now().timestamp();
    let news = with_memory(app, |m| {
        let news = apply_verdicts(m, verdicts, app_name, now);
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
            if record_word(m, c, now) {
                new_terms.push(c.term.clone());
            }
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
fn announce(app: &AppHandle, words: &[String]) {
    use crate::notices::{Notice, NoticeCode, NoticeSource};
    let (app, detail) = (app.clone(), words.join(", "));
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
        m.rules.clear();
        save(&app, m);
    });
}

/// Readable export to move to another machine (F113).
#[tauri::command]
#[specta::specta]
pub fn export_learned_words(app: AppHandle, path: String) -> Result<(), String> {
    let json = with_memory(&app, |m| serde_json::to_string_pretty(&m.words)).map_err(|e| e.to_string())?;
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
                if settings.learning_enabled && settings.context_send_enabled {
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
    fn misheard_common_word_is_learned_after_two_corrections() {
        let c = super::candidates("regarde le gag du serveur", "regarde le log du serveur");
        assert_eq!(
            c,
            vec![Candidate {
                term: "log".into(),
                heard: "gag".into(),
                kind: WordKind::Misheard
            }]
        );
        let mut m = Memory::default();
        assert!(!record_word(&mut m, &c[0], 0));
        assert!(!m.words[0].active);
        assert!(record_word(&mut m, &c[0], 1));
        assert!(m.words[0].active);
    }

    #[test]
    fn meaning_changes_are_never_learned() {
        assert!(super::candidates("la réunion est mardi", "la réunion est jeudi").is_empty());
        assert!(super::candidates("je dis oui merci", "je dis non merci").is_empty());
        assert!(super::candidates("on part à deux", "on part à trois").is_empty());
        assert!(!is_misheard_pair("mardi", "jeudi"));
        assert!(is_misheard_pair("gag", "log"));
    }

    #[test]
    fn verdicts_respect_the_guards() {
        use crate::judge::{Verdict, VerdictKind};
        let v = |kind, heard: Option<&str>, term: Option<&str>, rule: Option<&str>, same_as, ok| Verdict {
            kind,
            heard: heard.map(Into::into),
            term: term.map(Into::into),
            rule: rule.map(Into::into),
            same_as,
            consistent_with_history: ok,
        };
        let mut m = Memory::default();
        // A meaning change learns nothing, even proposed as misheard.
        apply_verdicts(
            &mut m,
            &[
                v(VerdictKind::Misheard, Some("mardi"), Some("jeudi"), None, None, true),
                v(VerdictKind::ContentChange, None, None, None, None, true),
            ],
            "Code",
            0,
        );
        assert!(m.words.is_empty() && m.rules.is_empty());
        // A misheard word from the judge is pending until seen twice too.
        apply_verdicts(&mut m, &[v(VerdictKind::Misheard, Some("gag"), Some("log"), None, None, true)], "Code", 0);
        assert!(!m.words[0].active);
        // A lone rule not backed by history waits for a second sighting.
        let rule = "Préférer « de manière claire et concise ».";
        apply_verdicts(&mut m, &[v(VerdictKind::StyleRule, None, None, Some(rule), None, false)], "Code", 0);
        assert!(!m.rules[0].active);
        let on = apply_verdicts(&mut m, &[v(VerdictKind::StyleRule, None, None, Some("…"), Some(0), false)], "Code", 1);
        assert!(m.rules[0].active && m.rules[0].count == 2);
        assert_eq!(on, vec![rule.to_string()]);
        // A rule consistent with history is active at once.
        apply_verdicts(&mut m, &[v(VerdictKind::Filler, None, None, Some("Supprimer « donc voilà »."), None, true)], "Code", 2);
        assert!(m.rules[1].active);
        // A `same_as` out of range is a new rule, not a crash.
        apply_verdicts(&mut m, &[v(VerdictKind::StyleRule, None, None, Some("Autre."), Some(99), false)], "Code", 3);
        assert_eq!(m.rules.len(), 3);
    }

    fn word(term: &str, heard: &str, count: u32, last_seen: i64, kind: WordKind, active: bool) -> Learned {
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
    fn prune_expires_pending_and_caps() {
        let day = 86_400;
        let mut m = Memory::default();
        m.words.push(word("log", "gag", 1, 0, WordKind::Misheard, false));
        m.words.push(word("Qwen", "Quen", 1, 0, WordKind::Term, true));
        prune(&mut m, 31 * day);
        assert_eq!(
            m.words.iter().map(|w| w.term.as_str()).collect::<Vec<_>>(),
            vec!["Qwen"]
        );
        m.words = (0..WORDS_MAX + 5)
            .map(|i| word(&format!("w{i}"), "", 1, i as i64, WordKind::Term, true))
            .collect();
        prune(&mut m, 0);
        assert_eq!(m.words.len(), WORDS_MAX);
        // The least used and oldest goes first.
        assert!(!m.words.iter().any(|w| w.term == "w0"));
    }
}
