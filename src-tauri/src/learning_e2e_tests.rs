//! End-to-end golden cases of the learning loop (wispr_insights/fixtures/golden):
//! correction → candidate → memory → the next dictation's rewrite context, ASR
//! vocabulary and fallback replacement. Real functions chained, no model call.

use crate::adaptive::context_block;
use crate::asr_context::{build, AsrCaps, AsrContextSettings, AsrSources};
use crate::context::{extract_terms, Content, LlmContext, Style, Tone};
use crate::learning::{
    candidates, mark_used, prune, record_decision, replace_heard, Memory, WordKind,
};
use crate::recall::relevant_terms;
use crate::validator::decide;

const NOW: i64 = 100 * 86_400;

fn caps_vocab() -> AsrCaps {
    AsrCaps {
        vocabulary: true,
        ..Default::default()
    }
}

fn base_ctx() -> LlmContext {
    LlmContext {
        content: Content::Other,
        style: Style::Other,
        tone: Tone::Default,
        terminal: false,
        before: String::new(),
        after: String::new(),
        selection: String::new(),
        names: Vec::new(),
        terms: Vec::new(),
        app: String::new(),
        window: String::new(),
        site: None,
        past: Vec::new(),
        rules: Vec::new(),
    }
}

/// Active terms, as `learning::plain_terms` gives the ASR.
fn plain_terms_of(m: &Memory) -> Vec<String> {
    m.words
        .iter()
        .filter(|w| w.active)
        .map(|w| w.term.clone())
        .collect()
}

/// One dictation corrected by the user, decided with the validator's reply
/// (`None`: no validator, the local fallback decides).
fn correct_with(m: &mut Memory, pasted: &str, kept: &str, reply: Option<&str>) {
    for (c, d) in decide(candidates(pasted, kept), reply) {
        record_decision(m, c, &d, NOW);
    }
}

/// One dictation corrected by the user, without a validator.
fn correct(m: &mut Memory, pasted: &str, kept: &str) {
    correct_with(m, pasted, kept, None);
}

/// The term learned from one corrected sentence, if any.
fn learned_with(pasted: &str, kept: &str, reply: Option<&str>) -> Option<String> {
    let mut m = Memory::default();
    correct_with(&mut m, pasted, kept, reply);
    m.words.iter().find(|w| w.active).map(|w| w.term.clone())
}

fn learned(pasted: &str, kept: &str) -> Option<String> {
    learned_with(pasted, kept, None)
}

#[test]
fn e13_a_corrected_word_is_reused_everywhere_in_the_next_dictation() {
    let mut m = Memory::default();
    correct(
        &mut m,
        "Nous installons Corvanti ce soir.",
        "Nous installons Quorvantis ce soir.",
    );
    // The next dictation, heard wrong again by the ASR:
    let terms = relevant_terms(&m.words, "nous installons corvanti demain", NOW, 80);
    assert_eq!(terms, vec![r#"Quorvantis (heard as "Corvanti")"#]);
    let ctx = LlmContext {
        terms: terms.clone(),
        ..base_ctx()
    };
    assert!(context_block(&ctx).contains("terms: Quorvantis (heard as"));
    let src = AsrSources {
        learned: plain_terms_of(&m),
        ..Default::default()
    };
    assert!(build(caps_vocab(), &src, &AsrContextSettings::default())
        .vocabulary
        .contains(&"Quorvantis".to_string()));
    // Seen once, and "Corvanti" could be a word: never replaced blindly.
    assert_eq!(
        replace_heard("Nous installons Corvanti demain.", &m.words),
        "Nous installons Corvanti demain."
    );
}

#[test]
fn e13_bis_a_word_confirmed_twice_is_replaced_without_rewrite() {
    let mut m = Memory::default();
    correct(
        &mut m,
        "Ouvre RandyFlow maintenant.",
        "Ouvre Handy Flow maintenant.",
    );
    assert_eq!(
        replace_heard("Ouvre RandyFlow.", &m.words),
        "Ouvre RandyFlow.",
        "one sighting is not enough"
    );
    correct(
        &mut m,
        "Lance RandyFlow ce soir.",
        "Lance Handy Flow ce soir.",
    );
    assert_eq!(
        replace_heard("Ouvre RandyFlow.", &m.words),
        "Ouvre Handy Flow."
    );
}

#[test]
fn e13_ter_an_unrelated_dictation_still_gets_the_spelling_without_a_mapping() {
    let mut m = Memory::default();
    correct(
        &mut m,
        "Demande à Cloud de relire.",
        "Demande à Claude de relire.",
    );
    // E226: "cloud" in an infra sentence is not forced; the bare term only.
    let terms = relevant_terms(&m.words, "on héberge ça ailleurs", NOW, 80);
    assert_eq!(terms, vec!["Claude"]);
}

#[test]
fn golden_learning_filter_l01_to_l08() {
    let some = |t: &str| Some(t.to_string());
    assert_eq!(
        learned("Merci Jordan, à demain.", "Merci Jordane, à demain."),
        some("Jordane")
    ); // L01
    assert_eq!(
        learned(
            "Nous utilisons Plurixan demain.",
            "Nous utilisons Plorixane demain."
        ),
        some("Plorixane")
    ); // L02
    assert_eq!(
        learned(
            "On migre la base Postgré demain.",
            "On migre la base PostgreSQL demain."
        ),
        some("PostgreSQL")
    ); // L03
    assert_eq!(
        learned("On se voit mardi soir.", "On se voit jeudi soir."),
        None
    ); // L04
    assert_eq!(
        learned("Le projet avance bien.", "Le projets avance bien."),
        None
    ); // L05
    assert_eq!(
        learned("Testons quelle solution.", "Testons Kelmorine solution."),
        None
    ); // L07 (Handy choice: a common word is no source form)
    assert_eq!(
        learned(
            "Le module ombrevecs est prêt.",
            "Le module Ombrevex est prêt."
        ),
        some("Ombrevex")
    ); // L08
}

#[test]
fn golden_learning_filter_l01_to_l08_with_the_validator() {
    let accept = |heard: &str, term: &str, kind: &str| {
        format!(r#"{{"accepted":[{{"heard":"{heard}","term":"{term}","kind":"{kind}"}}]}}"#)
    };
    let none = r#"{"accepted":[]}"#;
    let some = |t: &str| Some(t.to_string());
    let cases: [(&str, &str, String, Option<String>); 7] = [
        (
            "Merci Jordan, à demain.",
            "Merci Jordane, à demain.",
            accept("Jordan", "Jordane", "name"),
            some("Jordane"),
        ), // L01
        (
            "Nous utilisons Plurixan demain.",
            "Nous utilisons Plorixane demain.",
            accept("Plurixan", "Plorixane", "name"),
            some("Plorixane"),
        ), // L02
        (
            "On migre la base Postgré demain.",
            "On migre la base PostgreSQL demain.",
            accept("Postgré", "PostgreSQL", "technical"),
            some("PostgreSQL"),
        ), // L03
        (
            "On se voit mardi soir.",
            "On se voit jeudi soir.",
            none.into(),
            None,
        ), // L04, guarded before any call
        (
            "Le projet avance bien.",
            "Le projets avance bien.",
            none.into(),
            None,
        ), // L05
        (
            "Testons quelle solution.",
            "Testons Kelmorine solution.",
            none.into(),
            None,
        ), // L07, refused
        (
            "Le module ombrevecs est prêt.",
            "Le module Ombrevex est prêt.",
            accept("ombrevecs", "Ombrevex", "technical"),
            some("Ombrevex"),
        ), // L08
    ];
    for (pasted, kept, reply, expected) in cases {
        assert_eq!(learned_with(pasted, kept, Some(&reply)), expected, "{kept}");
    }
    // L04/L05 never reach the validator.
    assert!(candidates("On se voit mardi soir.", "On se voit jeudi soir.").is_empty());
    assert!(candidates("Le projet avance bien.", "Le projets avance bien.").is_empty());
}

#[test]
fn golden_l06_a_correction_chain_keeps_both_entries() {
    // E223: Wispr keeps Zorblax←Orblax and Zorblux←Zorblax side by side.
    let mut m = Memory::default();
    correct(
        &mut m,
        "On lance Orblax demain.",
        "On lance Zorblax demain.",
    );
    correct(
        &mut m,
        "On lance Zorblax demain.",
        "On lance Zorblux demain.",
    );
    let terms: Vec<&str> = m.words.iter().map(|w| w.term.as_str()).collect();
    assert_eq!(terms, vec!["Zorblax", "Zorblux"]);
}

#[test]
fn e02_e05_e15_screen_names_reach_both_asr_and_rewrite() {
    let terms = extract_terms(
        &[
            "Cc : Kasimir Zbignowski".into(),
            "On déploie Zorblax, Quintavex, Merindor.".into(),
        ],
        "",
        false,
        &[],
    );
    assert!(
        terms.contains(&"Kasimir Zbignowski".to_string()),
        "{terms:?}"
    );
    assert!(terms.contains(&"Zorblax".to_string()), "{terms:?}");
    let src = AsrSources {
        screen_terms: terms.clone(),
        ..Default::default()
    };
    let v = build(caps_vocab(), &src, &AsrContextSettings::default()).vocabulary;
    assert!(v.contains(&"Kasimir Zbignowski".to_string()), "{v:?}");
    // Rewrite side: the same names in the <context> block.
    let ctx = LlmContext {
        terms: terms.clone(),
        ..base_ctx()
    };
    assert!(context_block(&ctx).contains("Kasimir Zbignowski"));
    // E15: code identifiers from a .py file.
    let code = extract_terms(
        &["def compute_invoice_total(x):".into()],
        "billing_service.py - proj - Visual Studio Code",
        true,
        &[],
    );
    assert!(
        code.contains(&"compute_invoice_total".to_string()),
        "{code:?}"
    );
}

#[test]
fn a_full_rewrite_and_a_case_change_teach_nothing() {
    let mut m = Memory::default();
    correct(&mut m, "Je pars à Paris demain.", "Je pars à PARIS demain."); // A04
    correct(
        &mut m,
        "Nous allons organiser une réunion la semaine prochaine pour parler du budget.",
        "Finalement on annule tout et on reprend le dossier en janvier avec une autre equipe.",
    ); // A05
    assert!(m.words.is_empty(), "{:?}", m.words);
}

#[test]
fn the_learned_word_survives_a_year_and_its_uses_are_counted() {
    let mut m = Memory::default();
    correct(&mut m, "Merci Jeff, à demain.", "Merci Jev, à demain.");
    assert_eq!(m.words[0].kind, WordKind::Term);
    mark_used(&mut m, "merci jeff", "Merci Jev.", NOW);
    prune(&mut m, NOW + 365 * 86_400);
    assert_eq!(m.words.len(), 1);
    assert_eq!(m.words[0].used, 1);
}
