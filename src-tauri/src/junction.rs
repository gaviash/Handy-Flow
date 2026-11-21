//! Wispr's junction rules (spec §5, EX-1): just before the paste, the text's
//! case, final punctuation and spaces are fitted to the text around the
//! cursor. Deterministic, nothing learned. In doubt the text is kept.

use crate::context::{Content, Field, Tone};

pub struct JoinContext<'a> {
    pub field: Option<&'a Field>,
    pub style: Tone,
    pub content: Option<Content>,
    pub known_names: &'a [String],
}

pub struct Joined {
    pub text: String,
    /// The spaces around the text were decided here: no extra trailing space.
    pub spacing_decided: bool,
}

const ENDS: [char; 3] = ['.', '!', '?'];
/// Skipped when looking for the word a sentence goes on from.
const CLOSING: &str = "\"'»”’)]";

pub fn join(text: &str, ctx: &JoinContext) -> Joined {
    let mut out = text.trim().to_string();
    // Rule 0: no field or empty text, unchanged. Unknown caret (R4, J17):
    // only the space a glued word needs.
    let field = match ctx.field {
        Some(f) if !out.is_empty() => f,
        _ => {
            return Joined {
                text: text.to_string(),
                spacing_decided: false,
            }
        }
    };
    let lb = field.before.rsplit('\n').next().unwrap_or("");
    let space_before = needs_space_before(lb, &out);
    if !field.caret_known {
        let text = if space_before { format!(" {out}") } else { out };
        return Joined {
            text,
            spacing_decided: false,
        };
    }
    let la = field.after.split('\n').next().unwrap_or("");
    let sel = field.selection.as_str();

    // Rule 2: a chat message's final period, by style.
    let surround = format!("{lb}{sel}{la}");
    if sel.is_empty() && !surround.contains(ENDS) && drops_period(&out, ctx) {
        out.pop();
    }
    // Rule 7: a placeholder-like field: case, punctuation and spaces stay.
    let placeholder = (lb.ends_with("...") || lb.ends_with('…')) && la.is_empty() && sel.is_empty();
    if placeholder {
        return Joined {
            text: out,
            spacing_decided: true,
        };
    }
    // Rule 4: a replaced selection gives its case, final mark and spaces.
    if !sel.is_empty() {
        if sel.trim_start().starts_with(char::is_lowercase) && !keeps_capital(&out, ctx.known_names)
        {
            out = lower_first(&out);
        }
        if !sel.trim_end().ends_with(ENDS) {
            strip_end(&mut out);
        }
        let lead = if sel.starts_with(char::is_whitespace) {
            " "
        } else {
            ""
        };
        let trail = if sel.ends_with(char::is_whitespace) {
            " "
        } else {
            ""
        };
        return Joined {
            text: format!("{lead}{out}{trail}"),
            spacing_decided: true,
        };
    }
    // Rule 5: a sentence going on, text following, spaces.
    let goes_on = lb
        .trim_end()
        .trim_end_matches(|c: char| CLOSING.contains(c) || c.is_whitespace())
        .ends_with(char::is_alphanumeric);
    if goes_on
        && !ctx
            .known_names
            .iter()
            .any(|n| first_word(&out) == n.as_str())
    {
        out = lower_first_word(&out);
    }
    let next = la.trim_start().chars().next();
    if next.is_some_and(|c| c.is_lowercase() || ",.:;!?%>".contains(c)) {
        strip_end(&mut out);
    }
    let space_after = match la.chars().next() {
        None => out.ends_with(['.', '!', '?', '…']),
        Some(c) => c.is_alphanumeric(),
    };
    Joined {
        text: format!(
            "{}{out}{}",
            if space_before { " " } else { "" },
            if space_after { " " } else { "" }
        ),
        spacing_decided: true,
    }
}

fn cjk(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{9fff}' | '\u{ac00}'..='\u{d7af}' | '\u{f900}'..='\u{faff}' | '\u{ff00}'..='\u{ffef}')
}

/// After a letter, a digit, a sentence mark or a closing quote; never after an
/// opening, a newline, a space or CJK, nor before a mark that sticks to it.
fn needs_space_before(lb: &str, out: &str) -> bool {
    lb.chars()
        .last()
        .is_some_and(|c| (c.is_alphanumeric() && !cjk(c)) || ".,;:!?»”’)]".contains(c))
        && !out.starts_with(|c: char| ",.;:!?)]}»".contains(c))
}

/// Rule 2 (spec §5): very casual always, casual up to 10 sentences, default
/// in a work or personal chat up to 2, formal never. Only a plain period.
fn drops_period(out: &str, ctx: &JoinContext) -> bool {
    if !out.ends_with('.') || out.ends_with("..") {
        return false;
    }
    let sentences = out
        .split_inclusive(ENDS)
        .filter(|s| !s.trim().is_empty())
        .count();
    match ctx.style {
        Tone::VeryCasual => true,
        Tone::Casual => sentences <= 10,
        Tone::Formal => false,
        Tone::Default | Tone::Excited => {
            matches!(ctx.content, Some(Content::WorkChat | Content::PersonalChat)) && sentences <= 2
        }
    }
}

/// Drops a final `.`, `!` or `?` run; an ellipsis stays.
fn strip_end(out: &mut String) {
    if !out.ends_with("..") {
        while out.ends_with(ENDS) {
            out.pop();
        }
    }
}

fn first_word(text: &str) -> &str {
    text.split(' ')
        .next()
        .unwrap_or("")
        .trim_end_matches(|c: char| !c.is_alphanumeric())
}

/// A known name, an acronym or a word with an inner capital keeps its case.
fn keeps_capital(text: &str, known: &[String]) -> bool {
    let w = first_word(text);
    known.iter().any(|n| n == w) || w == "I" || w.chars().skip(1).any(char::is_uppercase)
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map_or(String::new(), |c| c.to_lowercase().chain(chars).collect())
}

/// "Le compte rendu" → "le compte rendu". Kept: acronyms, mixed case, "I",
/// two capitalized words (a name), and a lone word that may be a name.
fn lower_first_word(text: &str) -> String {
    let mut words = text.split(' ');
    let first = words.next().unwrap_or("");
    let core: String = first.chars().filter(|c| c.is_alphabetic()).collect();
    let second_capital = words
        .next()
        .and_then(|w| w.chars().next())
        .is_some_and(char::is_uppercase);
    let lone = !text.contains(' ');
    let keep = core.chars().count() < 2 && core != "A" && core != "À"
        || core.chars().skip(1).any(char::is_uppercase)
        || second_capital
        || (lone && !crate::adaptive::FR_COMMON_NAMES.contains(&core.to_lowercase().as_str()));
    if keep || !core.starts_with(char::is_uppercase) {
        return text.to_string();
    }
    lower_first(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(before: &str, after: &str, selection: &str) -> Field {
        Field {
            before: before.into(),
            after: after.into(),
            selection: selection.into(),
            caret_known: true,
        }
    }

    fn join_in(
        f: &Field,
        text: &str,
        style: Tone,
        content: Option<Content>,
        known: &[String],
    ) -> Joined {
        join(
            text,
            &JoinContext {
                field: Some(f),
                style,
                content,
                known_names: known,
            },
        )
    }

    fn at(before: &str, after: &str, text: &str) -> String {
        join_in(&field(before, after, ""), text, Tone::Default, None, &[]).text
    }

    #[test]
    fn joins_at_the_cursor_j01_to_j17() {
        assert_eq!(
            at("Le rendez-vous est fixé à ", " demain matin.", "10 heures."),
            "10 heures"
        );
        assert_eq!(
            at(
                "Peux-tu résumer ",
                " en trois points courts ?",
                "Le compte rendu de la réunion budgétaire."
            ),
            "le compte rendu de la réunion budgétaire"
        );
        assert_eq!(
            at("On se voit mardi soir.\nDeu", "xième ligne", "10:00"),
            " 10:00 "
        );
        assert_eq!(at("On se voit ", " soir.", "Mercredi."), "mercredi"); // J04
        assert_eq!(
            at("Voici le compte rendu.", "", "Nouvelle phrase de test."),
            " Nouvelle phrase de test. "
        );
        assert_eq!(
            at("Il est arrivé au projet. ", "", "Nouvelle phrase de test."),
            "Nouvelle phrase de test. "
        );
        assert_eq!(
            at("test ", "", "Deuxième phrase de contrôle."),
            "deuxième phrase de contrôle. "
        );
        // J08: plus an end space, as after any finished sentence at the end of the field.
        assert_eq!(
            at(
                "Il faut absolument",
                "",
                "Vérifiez les chiffres avant lundi."
            ),
            " vérifiez les chiffres avant lundi. "
        );
        assert_eq!(
            at("", "", "Voici mon message de test."),
            "Voici mon message de test. "
        );
        assert_eq!(
            at(
                "",
                "Explique-moi la différence entre Kubernetes et Terraform.",
                "Donne aussi un exemple concret."
            ),
            "Donne aussi un exemple concret. "
        );
        assert_eq!(at("Réunion avec ", " demain", "Paris."), "Paris");
        assert_eq!(at("Le projet ", " avance", "NASA."), "NASA");
        assert_eq!(
            at("Voir ", " pour les détails.", "https://example.com/doc"),
            "https://example.com/doc"
        );
        assert_eq!(
            at("Appelle ", "()", "computeInvoiceTotal"),
            "computeInvoiceTotal"
        );
        assert_eq!(at("Hier ", " suis parti", "I think"), "I think");
        assert_eq!(at("Avec ", " demain", "Jean Dupont."), "Jean Dupont");
        assert_eq!(at("Lance ", "", "npm run build"), "npm run build"); // J14, terminal too
    }

    #[test]
    fn unknown_caret_changes_nothing() {
        let unknown = |before: &str| Field {
            before: before.into(),
            ..Default::default()
        };
        let none = |f: &Field| {
            join_in(
                f,
                "Bonjour.",
                Tone::VeryCasual,
                Some(Content::PersonalChat),
                &[],
            )
        };
        // J17: only the space a glued word needs; no case, punctuation or end space.
        let j = none(&unknown("il faut"));
        assert_eq!(j.text, " Bonjour.");
        assert!(!j.spacing_decided);
        assert_eq!(none(&unknown("il faut ")).text, "Bonjour.");
        let no_field = join(
            " Bonjour.",
            &JoinContext {
                field: None,
                style: Tone::Default,
                content: None,
                known_names: &[],
            },
        );
        assert_eq!(no_field.text, " Bonjour.");
        assert!(!no_field.spacing_decided);
    }

    #[test]
    fn known_caret_decides_spacing() {
        assert!(join_in(&field("a ", "", ""), "b", Tone::Default, None, &[]).spacing_decided);
    }

    #[test]
    fn terminal_prompt_symbol_keeps_the_capital() {
        // `>` is not a letter: not a sentence going on, and the prompt's space is kept.
        assert_eq!(
            at("PS C:\\> ", "", "Liste les fichiers."),
            "Liste les fichiers. "
        );
        assert_eq!(
            at("PS C:\\>", "", "Liste les fichiers."),
            "Liste les fichiers. "
        );
    }

    #[test]
    fn known_name_keeps_its_capital() {
        let known = ["Jev".to_string()];
        let j = join_in(
            &field("merci ", "", ""),
            "Jev a validé.",
            Tone::Default,
            None,
            &known,
        );
        assert_eq!(j.text, "Jev a validé. ");
        assert_eq!(at("merci ", "", "Jev a validé."), "jev a validé. ");
    }

    #[test]
    fn continuation_skips_closing_quotes_but_not_other_symbols() {
        assert_eq!(
            at("il a dit « oui » ", "", "Puis il est parti."),
            "puis il est parti. "
        );
        assert_eq!(at("Bonjour, ", "", "Merci pour tout."), "Merci pour tout. ");
        assert_eq!(at("Total : 42 ", "", "Euros au mois."), "euros au mois. ");
    }

    #[test]
    fn following_text_drops_the_final_mark() {
        assert_eq!(
            at("Il a fini ", "% du travail", "Presque 80."),
            "presque 80"
        );
        assert_eq!(at("Tu viens ", "! dit-il", "Demain soir?"), "demain soir");
        assert_eq!(at("Tu viens ", "> suite", "Demain soir?"), "demain soir");
        assert_eq!(at("Il est ", "Paul arrive", "Parti hier."), "parti hier. ");
    }

    #[test]
    fn leading_space_never_after_an_opening_a_newline_or_cjk() {
        assert_eq!(at("(", ")", "Voir plus."), "Voir plus.");
        assert_eq!(at("Ligne\n", "", "Suite."), "Suite. ");
        assert_eq!(at("日本語", "", "Test."), "Test. ");
        assert_eq!(at("fin.", "", "Suite."), " Suite. ");
    }

    #[test]
    fn trailing_space_only_before_a_letter_or_digit() {
        assert_eq!(at("Voir ", "«doc»", "La page"), "la page");
        assert_eq!(at("Voir ", "2 pages", "Les deux"), "les deux ");
    }

    #[test]
    fn selection_takes_the_case_and_end_of_what_it_replaces() {
        let sel = |selection: &str, text: &str, known: &[String]| {
            join_in(
                &field("On se voit ", " soir.", selection),
                text,
                Tone::Default,
                None,
                known,
            )
            .text
        };
        assert_eq!(sel("mardi", "Jeudi.", &[]), "jeudi");
        assert_eq!(sel("Mardi.", "Jeudi.", &[]), "Jeudi.");
        assert_eq!(sel(" mardi ", "Jeudi", &[]), " jeudi ");
        assert_eq!(sel("paul", "Jev.", &["Jev".to_string()]), "Jev");
        assert_eq!(sel("le truc", "NASA.", &[]), "NASA");
    }

    #[test]
    fn messaging_final_period_follows_the_style() {
        let chat = |text: &str, tone: Tone, content: Content| {
            join_in(&field("", "", ""), text, tone, Some(content), &[]).text
        };
        let eleven = "Un. ".repeat(10) + "Onze.";
        assert_eq!(
            chat("On se voit demain.", Tone::VeryCasual, Content::Document),
            "On se voit demain"
        );
        assert_eq!(
            chat(
                "Ok. On se voit demain.",
                Tone::Casual,
                Content::PersonalChat
            ),
            "Ok. On se voit demain"
        );
        assert_eq!(
            chat(&eleven, Tone::Casual, Content::PersonalChat),
            format!("{eleven} ")
        );
        assert_eq!(
            chat("Ok. Demain.", Tone::Default, Content::WorkChat),
            "Ok. Demain"
        );
        assert_eq!(
            chat("Ok. Oui. Demain.", Tone::Default, Content::WorkChat),
            "Ok. Oui. Demain. "
        );
        assert_eq!(
            chat("Ok. Demain.", Tone::Default, Content::Document),
            "Ok. Demain. "
        );
        assert_eq!(
            chat("On se voit demain.", Tone::Formal, Content::PersonalChat),
            "On se voit demain. "
        );
        assert_eq!(
            chat("Tu viens ?", Tone::VeryCasual, Content::PersonalChat),
            "Tu viens ? "
        );
        // The user already writes with periods there, or replaces a selection.
        let f = field("Salut. ", "", "");
        assert_eq!(
            join_in(
                &f,
                "On se voit demain.",
                Tone::Casual,
                Some(Content::PersonalChat),
                &[]
            )
            .text,
            "On se voit demain. "
        );
        let f = field("Re ", "", "ça");
        assert_eq!(
            join_in(
                &f,
                "Ok demain.",
                Tone::VeryCasual,
                Some(Content::PersonalChat),
                &[]
            )
            .text,
            "ok demain"
        );
    }

    #[test]
    fn placeholder_like_field_skips_case_and_punctuation() {
        assert_eq!(
            at("Écris quelque chose...", "", "Bonjour à tous."),
            "Bonjour à tous."
        );
        assert_eq!(at("Tapez…", "", "Bonjour à tous."), "Bonjour à tous.");
    }
}
