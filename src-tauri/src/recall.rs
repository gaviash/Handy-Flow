//! Handy Adaptive: what to recall for a dictation — the learned words that
//! sound like it and the past dictations that look like it. Pure functions:
//! the rewrite gets a few relevant lines, not the whole memory.

use crate::learning::{distance, key, Learned, WordKind};
use crate::managers::history::PastDictation;

const DAY: i64 = 86_400;
const PAST_DAYS: i64 = 30;
const PAST_MAX: usize = 5;
const PAST_SIMILAR: usize = 3;
const PAST_CHARS: usize = 300;
/// A heard form this close to a dictated word (or two joined words) is a hit.
const HEARD_DISTANCE: f64 = 0.34;

/// Short and frequent words say nothing about the topic (fr/en).
const STOPWORDS: &[&str] = &[
    "les", "des", "une", "est", "que", "qui", "pour", "dans", "sur", "avec", "pas", "par", "mais",
    "ces", "son", "ses", "nous", "vous", "elle", "ils", "cette", "sont", "the", "and", "for",
    "you", "that", "this", "with", "are", "not", "but", "have", "was",
];

/// Learned words a dictated word sounds like, for the rewrite: a name as
/// `term (heard as "a")`, a common word as `term (maybe heard as "a")`.
/// Pending words never.
pub fn relevant_terms(words: &[Learned], transcription: &str, max: usize) -> Vec<String> {
    let said: Vec<String> = transcription.split_whitespace().map(key).collect();
    // "cloud code" also matches a heard "Cloudcode".
    let pairs = said.windows(2).map(|w| format!("{}{}", w[0], w[1]));
    let said: Vec<String> = said.iter().cloned().chain(pairs).collect();
    let heard_now = |l: &Learned| {
        l.heard.iter().any(|h| {
            let h = key(h);
            said.iter().any(|s| distance(&h, s) <= HEARD_DISTANCE)
        })
    };
    // Only words heard in this dictation: an unrelated learned word in the
    // prompt only invites a wrong substitution.
    let mut heard: Vec<&Learned> = words.iter().filter(|l| l.active && heard_now(l)).collect();
    heard.sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
    heard
        .into_iter()
        .take(max)
        .map(|l| {
            let forms: Vec<String> = l.heard.iter().map(|h| format!("\"{h}\"")).collect();
            // A name is forced; a common word only when nothing else fits.
            let label = match l.kind {
                WordKind::Term => "heard as",
                WordKind::Misheard => "maybe heard as",
            };
            format!("{} ({label} {})", l.term, forms.join(", "))
        })
        .collect()
}

fn topic_words(text: &str) -> std::collections::HashSet<String> {
    text.split_whitespace()
        .map(key)
        .filter(|w| w.chars().count() > 2 && !STOPWORDS.contains(&w.as_str()))
        .collect()
}

/// Past dictations for the rewrite (≤ 5, < 30 days): the 3 sharing the most
/// words with this one, then the 2 latest in the same app. Each ≤ 300 chars.
pub fn similar_dictations(
    past: &[PastDictation],
    transcription: &str,
    app: &str,
    now: i64,
) -> Vec<String> {
    let recent: Vec<&PastDictation> = past
        .iter()
        .filter(|p| p.timestamp >= now - PAST_DAYS * DAY && !p.text.trim().is_empty())
        .collect();
    let words = topic_words(transcription);
    let mut scored: Vec<(f64, &PastDictation)> = recent
        .iter()
        .map(|p| {
            let other = topic_words(&p.text);
            let union = words.union(&other).count().max(1);
            (words.intersection(&other).count() as f64 / union as f64, *p)
        })
        .filter(|(s, _)| *s > 0.0)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.timestamp.cmp(&a.1.timestamp)));
    let mut chosen: Vec<&PastDictation> = scored
        .into_iter()
        .take(PAST_SIMILAR)
        .map(|(_, p)| p)
        .collect();
    let mut same_app: Vec<&PastDictation> = recent
        .into_iter()
        .filter(|p| !app.is_empty() && p.app.as_deref() == Some(app))
        .collect();
    same_app.sort_by_key(|p| std::cmp::Reverse(p.timestamp));
    for p in same_app {
        if chosen.len() >= PAST_MAX {
            break;
        }
        if !chosen.iter().any(|c| std::ptr::eq(*c, p)) {
            chosen.push(p);
        }
    }
    chosen
        .into_iter()
        .map(|p| cut(&p.text, PAST_CHARS))
        .collect()
}

/// Function words: a phrase made only of them is no expression (fr/en).
const FUNCTION_WORDS: &[&str] = &[
    "le", "la", "les", "l", "de", "du", "des", "d", "un", "une", "et", "à", "a", "au", "aux", "en",
    "y", "il", "elle", "on", "je", "j", "tu", "nous", "vous", "ils", "que", "qu", "qui", "ce", "c",
    "est", "pas", "ne", "n", "se", "s", "sa", "son", "ça", "the", "an", "of", "to", "and", "is",
    "it", "in", "i",
];

/// Phrases of 2 to 4 words the user said in at least `min_count` dictations
/// of the last 30 days, longest kept when a shorter one only repeats it.
pub fn frequent_expressions(
    past: &[PastDictation],
    now: i64,
    min_count: usize,
    max: usize,
) -> Vec<String> {
    use std::collections::{HashMap, HashSet};
    let mut counts: HashMap<String, usize> = HashMap::new();
    for p in past.iter().filter(|p| p.timestamp >= now - PAST_DAYS * DAY) {
        let words: Vec<String> = p
            .text
            .split(|c: char| c.is_whitespace() || c == '\'' || c == '’' || c == '-')
            .map(key)
            .filter(|w| !w.is_empty())
            .collect();
        // Once per dictation: "il y a il y a" is one sighting.
        let mut seen = HashSet::new();
        for n in 2..=4 {
            for w in words.windows(n) {
                if w.iter().all(|x| FUNCTION_WORDS.contains(&x.as_str())) {
                    continue;
                }
                let phrase = w.join(" ");
                if seen.insert(phrase.clone()) {
                    *counts.entry(phrase).or_default() += 1;
                }
            }
        }
    }
    let mut found: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(_, c)| *c >= min_count)
        .collect();
    // Longest first, so a shorter phrase it contains is dropped below.
    found.sort_by(|a, b| {
        b.0.split(' ')
            .count()
            .cmp(&a.0.split(' ').count())
            .then(b.1.cmp(&a.1))
            .then(a.0.cmp(&b.0))
    });
    let mut kept: Vec<(String, usize)> = Vec::new();
    for (phrase, count) in found {
        let inside = kept
            .iter()
            .any(|(k, c)| *c >= count && format!(" {k} ").contains(&format!(" {phrase} ")));
        if !inside {
            kept.push((phrase, count));
        }
    }
    kept.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    kept.into_iter().take(max).map(|(p, _)| p).collect()
}

/// At most `max` chars, cut at a word, "…" when cut.
fn cut(text: &str, max: usize) -> String {
    let text = text.trim();
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max).collect();
    let head = head.rsplit_once(' ').map_or(head.as_str(), |(h, _)| h);
    format!("{}…", head.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_term_heard_in_the_dictation_comes_first() {
        let w = |t: &str, h: &str, c| Learned {
            term: t.into(),
            heard: vec![h.into()],
            count: c,
            last_seen: 0,
            kind: WordKind::Term,
            active: true,
            used: 0,
        };
        let words = vec![w("Postgres", "PostGer", 9), w("Claude", "Cloud", 1)];
        let out = relevant_terms(&words, "ouvre cloud code", 1);
        assert_eq!(out, vec![r#"Claude (heard as "Cloud")"#]);
        // Without a match, nothing is sent: no "most used" filler.
        assert!(relevant_terms(&words, "bonjour", 5).is_empty());
    }

    #[test]
    fn misheard_words_are_hints_terms_are_forced() {
        let w = |term: &str, heard: &str, kind| Learned {
            term: term.into(),
            heard: vec![heard.into()],
            count: 3,
            last_seen: 0,
            kind,
            active: true,
            used: 0,
        };
        let words = [
            w("Qwen", "Quen", WordKind::Term),
            w("config", "conflig", WordKind::Misheard),
            w("Kubernetes", "cube", WordKind::Term),
        ];
        let got = relevant_terms(&words, "on teste quen sur le conflig", 40);
        assert_eq!(
            got,
            [
                r#"Qwen (heard as "Quen")"#,
                r#"config (maybe heard as "conflig")"#
            ]
        );
        assert!(relevant_terms(&words, "bonjour à tous", 40).is_empty());
    }

    #[test]
    fn similar_dictations_pick_by_words_then_same_app() {
        let d = |t: &str, a: &str, ts| PastDictation {
            text: t.into(),
            app: Some(a.into()),
            timestamp: ts,
        };
        let now = 40 * 86_400;
        let past = vec![
            d("le log du serveur est vide", "Slack", now - 10),
            d("acheter du pain", "Code", now - 5),
            d("très vieux log serveur", "Slack", 0), // older than 30 days
        ];
        let out = similar_dictations(&past, "regarde le log du serveur", "Code", now);
        assert_eq!(out, vec!["le log du serveur est vide", "acheter du pain"]);
        assert!(similar_dictations(&[], "x", "Code", now).is_empty());
    }

    #[test]
    fn frequent_expressions_are_repeated_phrases_of_the_month() {
        let d = |t: &str, ts| PastDictation {
            text: t.into(),
            app: None,
            timestamp: ts,
        };
        let now = 40 * 86_400;
        let past = vec![
            d("Il faut le dire de manière claire, merci.", now - 1),
            d("Explique-le de manière claire et simple.", now - 2),
            d("Bref, de manière claire pour tout le monde.", now - 3),
            d("de manière claire de manière claire", 0), // older than 30 days
            d("il y a il y a il y a", now - 4),
        ];
        let out = frequent_expressions(&past, now, 3, 20);
        assert_eq!(out, vec!["de manière claire"]);
    }

    #[test]
    fn long_past_dictations_are_cut_at_a_word() {
        let long = format!("{} fin", "mot ".repeat(100));
        let past = vec![PastDictation {
            text: long,
            app: Some("Code".into()),
            timestamp: 10,
        }];
        let out = similar_dictations(&past, "mot", "Code", 20);
        assert!(out[0].chars().count() <= 301);
        assert!(out[0].ends_with('…'));
    }
}
