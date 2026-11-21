//! EX-4: the 50 000 most frequent French and English words, to tell a rare
//! word (a name, a technical term) from a common one. Data CC BY-SA 4.0,
//! see THIRD_PARTY_NOTICES.md.

use once_cell::sync::Lazy;
use std::collections::HashSet;

static WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    include_str!("../resources/lexicon/fr.txt")
        .lines()
        .chain(include_str!("../resources/lexicon/en.txt").lines())
        .collect()
});

/// True when the word, after an elision (`l'`, `d'`), is in the French or the
/// English list, or when every part of a hyphenated word is.
pub fn is_common(word: &str) -> bool {
    let w = word.trim().to_lowercase().replace('’', "'");
    let w = w.rsplit('\'').next().unwrap_or("");
    !w.is_empty() && (WORDS.contains(w) || w.split('-').all(|p| !p.is_empty() && WORDS.contains(p)))
}

#[cfg(test)]
mod tests {
    use super::is_common;

    #[test]
    fn common_words_are_known_in_either_language() {
        for w in [
            "quelle",
            "cloud",
            "projet",
            "the",
            "Projet",
            "l'projet",
            "porte-monnaie",
        ] {
            assert!(is_common(w), "{w}");
        }
    }

    #[test]
    fn rare_words_and_names_are_not_common() {
        for w in [
            "kelmorine",
            "zorblax",
            "corvanti",
            ".env",
            "",
            "l'",
            "ultra-zorblax",
        ] {
            assert!(!is_common(w), "{w}");
        }
    }
}
