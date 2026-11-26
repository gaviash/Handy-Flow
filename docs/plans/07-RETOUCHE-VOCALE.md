# Plan 7 : retouche vocale (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : H1 ; V6 réduite (pas de champ entier). Dépend des plans 3 et 5 (bouton d'historique). Durée : 1 j. Rien n'est codé.
**Goal :** retoucher la dernière dictée à la voix sans la sélectionner. **Architecture :** la région du plan 3, sélectionnée par UI Automation, puis le chemin d'instruction actuel. **Tech Stack :** Rust, crate `windows`.

## A. But

Juste après une dictée, tu dis « rends ça formel » et le texte collé est remplacé, sans le sélectionner.

## B. Constats

- L'instruction vocale existe (Ctrl+Alt+Espace maintenu) mais exige une sélection ; sinon, notice.
- **Correction d'une affirmation :** j'avais écrit que tu avais tenté deux fois une retouche sans sélection dans Gmail. Le journal montre que les n° 312 et 314 étaient de vraies instructions, **avec** une sélection, et qu'un texte a été collé. Le seul indice d'un besoin est le n° 313 : une consigne (« change le prénom dedans… ») dite avec le raccourci de **dictée**, donc traitée comme du texte.
- Le plan 3 sait retrouver dans un champ la région de la dernière dictée. Il reste à la sélectionner : `IUIAutomationTextRange::Select` (à sonder dans Chrome, VS Code, Bloc-notes).

## C. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Cible, dans l'ordre : sélection ; sinon dernière dictée collée, si elle est encore dans le champ qui a le focus, retrouvée une seule fois et depuis moins de 5 min ; sinon notice, rien ne change |
| 2 | Remplacement : sélection de la région par UI Automation, relue pour vérifier qu'elle contient bien le texte attendu, puis collage par-dessus. Champ, sélection et génération relus **juste avant** le collage. Le moindre doute → rien n'est modifié, texte au presse-papiers et notice. Risque restant : un clic dans les quelques millisecondes entre ce contrôle et le collage |
| 3 | Ctrl+Z de l'application rend le texte d'avant |
| 4 | Terminal : pas de retouche (aucune sélection fiable), notice |
| 5 | Plusieurs retouches de suite : chacune part du dernier résultat |
| 6 | Le résultat est suivi par le plan 3, marqué « retouche » : il n'apprend pas de mots de l'ASR, seulement ce que tu corriges ensuite |
| 7 | Polish et transformations par touche : même cible quand rien n'est sélectionné |
| 8 | Avant/après consultable dans l'historique (bouton du plan 5), pas de fenêtre à l'écran |

## E. Tes réponses (25/11)

- **Q10 : Ctrl+Alt+Espace**, le raccourci d'instruction actuel. Ctrl+Espace reste de la dictée pure, sans devinette.
- **Q11 : une notice**, rien ne change.

## F. Acceptation

| Cas | Attendu |
|---|---|
| dictée, puis « rends ça formel » | texte remplacé, rien autour ne change |
| texte modifié à la main entre les deux | version modifiée retouchée, si plausible (plan 3) |
| même texte deux fois dans le champ | rien modifié, notice |
| champ changé, fenêtre changée | rien modifié, notice |
| mot de passe ou application exclue | refus |
| refus du modèle (« je ne peux pas ») | texte inchangé, notice |
| Ctrl+Z | texte d'avant |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/retouche-vocale`, créée depuis la branche du plan 6.

## H. Review Focus

1. Sélection d'un seul mot par mégarde : c'est elle qui est modifiée (comme aujourd'hui). → tâche 2.
2. Le champ change entre la parole et la réponse du modèle : rien n'est collé ailleurs. → tâche 3.
3. Deux dictées collées à la suite : la dernière seulement. → tâche 2.
4. Texte au-delà de 8 000 caractères : notice existante. → tâche 2.
5. Le champ n'accepte pas `Select` : texte au presse-papiers, notice. → tâche 3.

## I. Tâches

### Tâche 0 : sonde (avec ton accord, 15 min)
- [ ] Exception consentie à la règle « aucun test qui prend le focus » : cette sonde change la sélection du champ de test, rien d'autre. Test manuel `#[ignore]` qui sélectionne un texte donné dans le champ focalisé (`FindText` + `Select`) et relit la sélection. Cibles : Gmail, ChatGPT, VS Code (chat et éditeur), Bloc-notes, avec un texte fictif tapé par toi. Aucune touche envoyée.
- [ ] Décision écrite ici : applications prises en charge. Si `Select` échoue dans Gmail **et** VS Code, le plan s'arrête et je te propose autre chose.

### Tâche 1 : sélectionner une région
**Files :** Modify `context/windows.rs`, `context/mod.rs`.
**Produit :** `pub fn select_region(element: &[i32], before: &str, after: &str, expected: &str) -> Result<(), SelectError>` (`Gone`, `NotFound`, `Ambiguous`, `Refused`) ; relit la sélection et la compare (`unify`, espaces ramenés à un) ; échec → sélection d'avant remise.
- [ ] Tests sur un texte factice : `the_region_between_its_anchors_is_selected`, `a_moved_anchor_is_not_found`, `two_possible_regions_are_ambiguous`, `a_selection_that_reads_differently_is_refused`.
- [ ] Commit `feat(context): select a known text in its field`.

### Tâche 2 : la cible
**Files :** Create `src-tauri/src/retouch.rs` ; Modify `learning.rs` (dernière région plausible), `actions.rs`.
**Produit :** `struct LastPaste { session: u64, element: Vec<i32>, before: String, after: String, text: String, at: Instant, history_id: Option<i64> }` (bornes du suivi, plan 3) (en mémoire seulement, effacé à 5 min, à la mise en veille et pour un champ privé) ; `enum Target { Selection(String), Last(LastPaste), None }` ; `struct Now { element: Vec<i32>, terminal: bool, text: String, selection: Option<String>, at: Instant }` ; `fn target(last: Option<&LastPaste>, now: &Now) -> Target` (la région actuelle vient des bornes, et doit être plausible).
- [ ] Tests : `a_selection_wins` ; `the_last_paste_needs_the_same_field_and_five_minutes` ; `an_edited_dictation_is_the_target_when_plausible` ; `a_terminal_has_no_target` ; `two_dictations_target_the_last` ; `over_8000_chars_is_none`.
- [ ] Commit `feat(retouch): what a spoken instruction changes`.

### Tâche 3 : branchement
**Files :** Modify `actions.rs` (instruction vocale et `TransformAction`), `notices.rs`, traductions.
**Consomme :** `target`, `select_region`.
- [ ] Une nouvelle instruction ou dictée invalide les appels en cours (génération de session). Ordre : modèle d'abord, puis champ **et sélection** relus et comparés, `select_region`, dernier contrôle, collage. Toute erreur → presse-papiers et notice « La retouche est prête : colle-la où tu veux ».
- [ ] `LastPaste` mis à jour seulement après un collage lu et pour la génération courante ; `touch_used` n'est pas appelé ; suivi du plan 3 avec la source « retouche » (aucun mot de l'ASR appris).
- [ ] Notices : « Sélectionne le texte à modifier » (aucune cible), « Pas de retouche dans un terminal ».
- [ ] Tests : `the_field_is_checked_again_after_the_model` ; `a_changed_selection_is_not_replaced` ; `a_late_answer_never_overwrites_a_newer_one` ; `a_retouch_teaches_no_spoken_word` ; `a_refusal_changes_nothing` ; `a_retouch_can_be_retouched` ; Échap pendant l'appel → rien.
- [ ] Commit `feat(retouch): say it, the last dictation changes`.

### Tâche 4 : revue, build, installation
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation.
- [ ] Avec toi : partie F dans Gmail, ChatGPT et VS Code.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/07-v2.md`. Bugs déplacés au plan 0 : 2.5, 3.3, 3.5, 5.3.
**À corriger dans ce plan**
- Les bornes texte-entier du suivi ne doivent pas servir à retrouver la région : elles cassent si on corrige ailleurs (C1).
- Sélection UIA sous-spécifiée : texte unifié (U+202F/U+00A0), sélection asynchrone dans Chrome, portée limitée au champ, coût de la relecture (C2).
- Repli si `Select` échoue dans Gmail/VS Code (C3) ; vérifier que le collage est possible avant de sélectionner (C6).
- « Même cible pour Polish » est un chantier de 1,5–2 j (C4) ; « terminal : pas de retouche » trop grossier (C5).
- L'historique avant/après suppose un champ qui n'existe pas (C8).
- Estimation revue à la hausse (C10).
**Améliorations retenues**
- Sortie balisée : le modèle enveloppe la retouche, le reste est rejeté (B1, Willow).
- Résultat identique : pas de collage, notice « aucun changement » (B2).
- Attendre le relâchement des modificateurs avant Ctrl+V (B5) ; retour « Retouché · Ctrl+Z pour annuler » (B6).
- Consigne vide ou bruit : ne rien faire (A4) ; messages d'échec exacts (A5).
