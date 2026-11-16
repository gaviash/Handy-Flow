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
fn key(w: &str) -> String {
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
pub fn candidates(inserted: &str, fin: &str) -> Vec<(String, String)> {
    let a: Vec<&str> = inserted.split_whitespace().collect();
    let b: Vec<&str> = fin.split_whitespace().collect();
    let tags = align(&a, &b);
    let edited = tags
        .iter()
        .filter(|t| t.0 != Tag::Same && t.0 != Tag::Case)
        .count();
    // A rewrite, not a correction.
    if edited * 2 > a.len().max(b.len()) {
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
            continue;
        }
        let bounded = |t: Option<&(Tag, Option<usize>, Option<usize>)>| {
            t.is_none_or(|t| matches!(t.0, Tag::Same | Tag::Case))
        };
        let after_item = start.checked_sub(1).is_some_and(|s| ends_item(&tags[s]));
        if run.len() > 4
            || !(after_item || bounded(start.checked_sub(1).and_then(|s| tags.get(s))))
            || !(ends_item(&tags[k - 1]) || bounded(tags.get(k)))
        {
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
        if !term.is_empty() && looks_like_term && !affix && distance(&hk, &tk) <= 0.6 {
            out.push((term, heard));
        }
    }
    out.truncate(4);
    out
}

// ---------------------------------------------------------------- memory

static MEMORY: Mutex<Option<Vec<Learned>>> = Mutex::new(None);

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

fn with_memory<T>(app: &AppHandle, f: impl FnOnce(&mut Vec<Learned>) -> T) -> T {
    let mut guard = MEMORY.lock().unwrap_or_else(|e| e.into_inner());
    let entries = guard.get_or_insert_with(|| {
        memory_path(app)
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| dpapi::run(&b, false))
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    });
    f(entries)
}

fn save(app: &AppHandle, entries: &[Learned]) {
    let (Some(path), Ok(json)) = (memory_path(app), serde_json::to_vec(entries)) else {
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

/// Learned spellings as terms for the rewrite, most used first.
pub fn terms(app: &AppHandle) -> Vec<String> {
    with_memory(app, |m| {
        let mut m = m.clone();
        m.sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
        m.iter()
            .take(30)
            .map(|l| {
                let heard = l
                    .heard
                    .iter()
                    .map(|h| format!("\"{h}\""))
                    .collect::<Vec<_>>();
                format!("{} (heard as {})", l.term, heard.join(", "))
            })
            .collect()
    })
}

fn learn(app: &AppHandle, inserted: &str, fin: &str) -> usize {
    let found = candidates(inserted, fin);
    let n = found.len();
    if found.is_empty() {
        return 0;
    }
    let now = chrono::Utc::now().timestamp();
    // F95: new spellings are announced once, grouped, with an undo.
    let mut new_terms = Vec::new();
    with_memory(app, |m| {
        for (term, heard) in found {
            log::info!("Learned a spelling ({} chars)", term.chars().count());
            match m.iter_mut().find(|l| l.term == term) {
                Some(l) => {
                    l.count += 1;
                    l.last_seen = now;
                    // A new way of hearing the same word is news too.
                    if add_heard(l, heard) {
                        new_terms.push(term.clone());
                    }
                }
                None => {
                    new_terms.push(term.clone());
                    m.push(Learned {
                        term,
                        heard: vec![heard],
                        count: 1,
                        last_seen: now,
                    })
                }
            }
        }
        save(app, m);
    });
    if !new_terms.is_empty() {
        let _ = app.emit("learned-words", &new_terms);
        announce(app, &new_terms);
    }
    n
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
    with_memory(app, |m| m.len())
}

#[tauri::command]
#[specta::specta]
pub fn get_learned_words(app: AppHandle) -> Vec<Learned> {
    with_memory(&app, |m| m.clone())
}

#[tauri::command]
#[specta::specta]
pub fn delete_learned_word(app: AppHandle, term: String) {
    with_memory(&app, |m| {
        m.retain(|l| l.term != term);
        save(&app, m);
    });
}

#[tauri::command]
#[specta::specta]
pub fn clear_learned_words(app: AppHandle) {
    with_memory(&app, |m| {
        m.clear();
        save(&app, m);
    });
}

/// Readable export to move to another machine (F113).
#[tauri::command]
#[specta::specta]
pub fn export_learned_words(app: AppHandle, path: String) -> Result<(), String> {
    let json = with_memory(&app, |m| serde_json::to_string_pretty(m)).map_err(|e| e.to_string())?;
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
pub fn observe(app: &AppHandle, field: &crate::context::Field, inserted: &str) {
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
                _ => 0,
            };
            log::debug!(
                "Learning: observation ended ({ended}); edited: {}, learned: {found}",
                last.as_ref().is_some_and(|fin| fin != &inserted)
            );
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
}
