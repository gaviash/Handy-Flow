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

/// The learned dictionary for the rewrite (Wispr `dictionary_context`, E50):
/// first the words a dictated word sounds like, a name as
/// `term (heard as "a")`, a common word as `term (maybe heard as "a")`; then
/// every other active name as its bare spelling, newest first by Wispr's
/// buckets (≤ 24 h, ≤ 14 days, older, E79) then most used. A bare term has
/// no mapping, so a heard form is never forced outside its context (E226).
/// Pending words never; at most `max`.
pub fn relevant_terms(words: &[Learned], transcription: &str, now: i64, max: usize) -> Vec<String> {
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
    let (mut heard, rest): (Vec<&Learned>, Vec<&Learned>) = words
        .iter()
        .filter(|l| l.active)
        .partition(|l| heard_now(l));
    heard.sort_by(|a, b| b.count.cmp(&a.count).then(b.last_seen.cmp(&a.last_seen)));
    let bucket = |l: &Learned| match now - l.last_seen {
        age if age <= DAY => 0,
        age if age <= 14 * DAY => 1,
        _ => 2,
    };
    // A common word without its heard form says nothing to the rewrite.
    let mut rest: Vec<&Learned> = rest
        .into_iter()
        .filter(|l| l.kind == WordKind::Term)
        .collect();
    rest.sort_by(|a, b| {
        bucket(a)
            .cmp(&bucket(b))
            .then(b.used.cmp(&a.used))
            .then(b.count.cmp(&a.count))
    });
    let mapped = heard.into_iter().map(|l| {
        let forms: Vec<String> = l.heard.iter().map(|h| format!("\"{h}\"")).collect();
        let label = match l.kind {
            WordKind::Term => "heard as",
            WordKind::Misheard => "maybe heard as",
        };
        format!("{} ({label} {})", l.term, forms.join(", "))
    });
    mapped
        .chain(rest.into_iter().map(|l| l.term.clone()))
        .take(max)
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
        .map(|p| cut(&crate::context::redact(&p.text), PAST_CHARS))
        .collect()
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

    const NOW: i64 = 100 * 86_400;

    fn w(term: &str, heard: &str, kind: WordKind, count: u32, used: u32, age_days: i64) -> Learned {
        Learned {
            term: term.into(),
            heard: vec![heard.into()],
            count,
            last_seen: NOW - age_days * 86_400,
            kind,
            active: true,
            used,
            source: Default::default(),
        }
    }

    #[test]
    fn every_active_term_is_sent_heard_ones_first() {
        let mut pending = w("specs", "spikes", WordKind::Misheard, 1, 0, 0);
        pending.active = false;
        let words = vec![
            w("Postgres", "PostGer", WordKind::Term, 9, 0, 0),
            w("Claude", "Cloud", WordKind::Term, 1, 0, 0),
            pending,
        ];
        let out = relevant_terms(&words, "ouvre cloud code", NOW, 80);
        assert_eq!(out, vec![r#"Claude (heard as "Cloud")"#, "Postgres"]);
        assert!(!out.iter().any(|t| t.starts_with("specs")));
    }

    #[test]
    fn unheard_terms_have_no_mapping() {
        let words = vec![w("Claude", "Cloud", WordKind::Term, 3, 0, 0)];
        let out = relevant_terms(&words, "bonjour", NOW, 80);
        assert_eq!(out, vec!["Claude"]);
    }

    #[test]
    fn unheard_terms_follow_wispr_recency_buckets() {
        let words = vec![
            w("Old", "old", WordKind::Term, 9, 50, 20),
            w("Week", "week", WordKind::Term, 1, 9, 3),
            w("Recent", "recent", WordKind::Term, 1, 1, 0),
            w("Recent2", "recentb", WordKind::Term, 1, 4, 0),
        ];
        assert_eq!(
            relevant_terms(&words, "", NOW, 80),
            vec!["Recent2", "Recent", "Week", "Old"]
        );
        assert_eq!(
            relevant_terms(&words, "", NOW, 2),
            vec!["Recent2", "Recent"]
        );
    }

    #[test]
    fn an_unheard_misheard_word_is_not_sent() {
        let words = [
            w("Qwen", "Quen", WordKind::Term, 3, 0, 0),
            w("config", "conflig", WordKind::Misheard, 3, 0, 0),
        ];
        assert_eq!(
            relevant_terms(&words, "on teste quen sur le conflig", NOW, 40),
            [
                r#"Qwen (heard as "Quen")"#,
                r#"config (maybe heard as "conflig")"#
            ]
        );
        assert_eq!(relevant_terms(&words, "bonjour à tous", NOW, 40), ["Qwen"]);
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
    fn past_dictations_hide_personal_details() {
        let past = vec![PastDictation {
            text: "Contact alice@example.com".into(),
            app: Some("Mail".into()),
            timestamp: 10,
        }];
        let result = similar_dictations(&past, "contact", "Mail", 20);
        assert_eq!(result.len(), 1);
        assert!(!result[0].contains("alice@example.com"));
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
