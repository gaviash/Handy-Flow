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
use tauri::AppHandle;

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

/// A learned spelling: `term` is what the user wrote, `heard` what was inserted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Learned {
    pub term: String,
    pub heard: String,
    pub count: u32,
    /// Unix seconds.
    pub last_seen: i64,
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
    while k < tags.len() {
        if tags[k].0 != Tag::Sub {
            k += 1;
            continue;
        }
        let start = k;
        while k < tags.len() && tags[k].0 == Tag::Sub {
            k += 1;
        }
        let run = &tags[start..k];
        let bounded = |t: Option<&(Tag, Option<usize>, Option<usize>)>| {
            t.is_none_or(|t| matches!(t.0, Tag::Same | Tag::Case))
        };
        if run.len() > 4
            || !bounded(start.checked_sub(1).and_then(|s| tags.get(s)))
            || !bounded(tags.get(k))
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
        let first = run[0].2;
        let opens = first.is_none_or(|j| j == 0 || b[j - 1].ends_with(['.', '!', '?']));
        let capital = term.split(' ').enumerate().any(|(n, w)| {
            w.chars()
                .enumerate()
                .any(|(i, c)| c.is_uppercase() && (i > 0 || n > 0 || !opens))
        });
        let looks_like_term = capital || term.chars().any(|c| c.is_ascii_digit() || c == '_');
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
            .map(|l| format!("{} (heard as \"{}\")", l.term, l.heard))
            .collect()
    })
}

fn learn(app: &AppHandle, inserted: &str, fin: &str) {
    let found = candidates(inserted, fin);
    if found.is_empty() {
        return;
    }
    let now = chrono::Utc::now().timestamp();
    with_memory(app, |m| {
        for (term, heard) in found {
            log::info!("Learned a spelling ({} chars)", term.chars().count());
            match m.iter_mut().find(|l| l.term == term) {
                Some(l) => {
                    l.count += 1;
                    l.last_seen = now;
                    l.heard = heard;
                }
                None => m.push(Learned {
                    term,
                    heard,
                    count: 1,
                    last_seen: now,
                }),
            }
        }
        save(app, m);
    });
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
const ANCHOR_CHARS: usize = 40;

/// Ends the running observation now (a new dictation started): it learns
/// from its last reading before this dictation's rewrite (F98).
pub fn end_observation() {
    OBSERVATION.fetch_add(1, Ordering::SeqCst);
}

/// The inserted region in the field's text, between the anchors read at
/// release. `None` if an anchor is missing or ambiguous (PAR §7.1, R6).
pub fn region<'a>(text: &'a str, before: &str, after: &str) -> Option<&'a str> {
    let start = if before.is_empty() {
        0
    } else {
        let mut found = text.match_indices(before);
        let (i, _) = found.next()?;
        if found.next().is_some() {
            return None;
        }
        i + before.len()
    };
    let rest = &text[start..];
    let end = if after.is_empty() {
        rest.len()
    } else {
        rest.find(after)?
    };
    Some(&rest[..end])
}

/// F80–F83: watches the field the text was pasted into, then learns from the
/// last reading whose anchors still hold.
pub fn observe(app: &AppHandle, field: &crate::context::Field, inserted: &str) {
    if !field.caret_known || !crate::settings::get_settings(app).learning_enabled {
        return;
    }
    let before: String = crate::context::tail_chars(&field.before, ANCHOR_CHARS);
    let after: String = field.after.chars().take(ANCHOR_CHARS).collect();
    let inserted = inserted.trim().to_string();
    let id = OBSERVATION.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    let _ = std::thread::Builder::new()
        .name("observe".into())
        .spawn(move || {
            if !crate::context::watch_focused() {
                return;
            }
            let started = Instant::now();
            let mut last: Option<String> = None;
            // A newer dictation ended this one: learn from the last reading,
            // never read its field again.
            while OBSERVATION.load(Ordering::SeqCst) == id && started.elapsed() < WINDOW {
                match crate::context::read_watched() {
                    // An empty field was sent or cleared: keep the last reading.
                    Some(text) if !text.trim().is_empty() => match region(&text, &before, &after) {
                        // Tracked only once the pasted text has shown up there.
                        Some(r) if last.is_some() || r.trim() == inserted => {
                            last = Some(r.trim().to_string())
                        }
                        _ if last.is_none() && started.elapsed() < Duration::from_secs(3) => {}
                        _ => break,
                    },
                    _ => break,
                }
                std::thread::sleep(Duration::from_secs(1));
            }
            if let Some(fin) = last {
                if fin != inserted {
                    learn(&app, &inserted, &fin);
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn finds_the_inserted_region() {
        assert_eq!(
            region("Bonjour Adryen, merci.", "Bonjour ", ", merci."),
            Some("Adryen")
        );
        assert_eq!(region("tout le texte", "", ""), Some("tout le texte"));
        assert_eq!(region("a X b a Y b", "a ", " b"), None); // ambiguous anchor
        assert_eq!(region("autre chose", "Bonjour ", ""), None);
    }
}
