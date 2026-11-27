# Plan 3 : suivi des corrections (2026-11-24)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : K0 (reste), K2, D1 à D8, W4, W5, H7, V10. Dépend du plan 2. Durée : 1,5 à 2 j. Rien n'est codé.
**Goal :** relire fidèlement ce que tu gardes et dire pourquoi sinon. **Architecture :** la boucle d'observation actuelle, avec une garde de plausibilité, un motif par dictée et des leçons stockées. **Tech Stack :** Rust, SQLite, DPAPI, React.

## A. But

Handy relit ce que tu as vraiment gardé, ne confond jamais avec un autre texte, et dit pourquoi quand il n'a rien appris.

## B. Constats

- 110 textes finals relus sur 254 dictées (24/11, 19 h 21).
- 115 textes gardés à 23 h 15, dont **17 faux** : 7 vides (n° 185, 186, 187, 189, 190, 214, 254), 5 fois « Ask Codex to do anything » (238, 263, 265, 266, 267), 4 restes d'effacement (183, 184, 188, 237), et le n° 313 (mail entier, réparé par le plan 1).
- Le n° 315 (mail largement réécrit) est **juste** et doit rester.
- Journal depuis 22 h 32 (relevé en écrivant ce plan) : 15 suivis terminés, dont 10 « champ vidé », 2 « texte autour changé », 1 « dictée suivante » et 2 « introuvable », ces deux-là dans le terminal.
- Une lecture ratée suffit à tout perdre ; 60 s fixes ; aucun motif enregistré quand le suivi échoue.

## C. Conception

| # | Décision (mienne, corrigeable) | Règle |
|---|---|---|
| 1 | Champ suivi = celui du relâchement (plan 2), pas celui qui a le focus après le collage | D6 |
| 2 | Lecture toutes les 300 ms pendant 10 s et 10 s après chaque changement, puis 2 s, jusqu'à 5 min | W4, D5 |
| 3 | 3 lectures ratées d'affilée avant d'abandonner ; une seule lecture en vol | D2 |
| 4 | Texte plausible, sinon rien n'est gardé : longueur entre 0,25 et 4 fois le texte collé, au moins 35 % de ses mots, et pas un simple début du texte (effacement en cours) | W5, D3, D4 |
| 5 | Terminal : espaces et retours à la ligne ramenés à un seul espace avant de chercher le texte | D8 |
| 6 | Curseur inconnu → suivi si le texte collé n'apparaît qu'une fois | D6 |
| 7 | Lecture jusqu'à 50 000 caractères (15 000 aujourd'hui) | D6 |
| 8 | Texte autour changé → appris seulement si la dictée est encore là, intacte ou plausible | défaut de `finish` (`bounded`) |
| 9 | Motif de fin enregistré pour **chaque** dictée (colonne `learning_end`), sans texte | D7 |
| 10 | Historique : « Appris : X », « Rien à apprendre », ou la raison (« champ illisible », « texte introuvable », « autre texte », « application exclue »…) | H7 |
| 11 | Leçons en attente chiffrées dans `learned.bin`, reprises au démarrage, 7 jours et 50 au plus | K0 |

**Règle 4 vérifiée** sur les 115 textes : elle écarte les 17 faux, aucun autre, et garde le n° 315. Limite : ses seuils ont été choisis en regardant ces mêmes textes.

## D. Découpage du code

- `learning.rs` : `plausible(inserted: &str, kept: &str) -> bool`, `collapse_spaces(text: &str) -> String`, boucle de `observe`, `finish`, `End` (une variante par motif), leçons en attente dans `Memory`.
- `judge.rs` : les essais lisent et effacent les leçons stockées.
- `managers/history.rs` : colonne `learning_end`, `set_learning_end(id: i64, end: &str)`, champ dans `HistoryEntry`.
- `context/windows.rs` : lecture à 50 000, cible du plan 2.
- `HistorySettings.tsx`, `bindings.ts`, traductions.

## E. Tes réponses (24/11)

- **Q3 : oui, Entrée seule.** `GetAsyncKeyState(VK_RETURN)`, interrogée toutes les 50 ms pendant un suivi seulement. Jamais une autre touche (donc pas Maj), jamais de crochet clavier : c'est l'effet sur le champ qui dit si c'était un envoi.
- **Q4 : effacer les 16 textes faux** (`kept_text` à NULL), après sauvegarde.

Règle d'Entrée :
| Situation | Résultat |
|---|---|
| Entrée, puis champ vidé ou remplacé en moins de 2 s | envoyé : la dernière lecture plausible est gardée |
| Entrée, texte toujours là (retour à la ligne, e-mail) | le suivi continue |
| Champ vidé sans Entrée | gardé seulement si la dernière lecture est plausible (bouton Envoyer) |
| Lecture finale illisible après un envoi | la dernière lecture est gardée dans l'historique, mais n'apprend rien (règle R7) |

## F. Acceptation

| Cas | Attendu |
|---|---|
| correction à 3 min | apprise |
| deux lectures ratées puis une bonne | le suivi continue |
| champ vide, dictée, envoi, puis autre texte tapé | l'autre texte n'est jamais attribué à la dictée |
| Codex dans le terminal, texte replié sur deux lignes | texte retrouvé |
| mail réécrit en grande partie | gardé comme exemple |
| Groq refuse, Handy fermé puis relancé | leçon reprise |
| chaque dictée | un motif visible dans l'historique |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `fix/suivi-corrections`, créée depuis la branche du plan 2.

## H. Review Focus

1. Correction d'un seul mot dans une dictée de deux mots (« Cloud code » → « Claude code ») : plausible (un mot sur deux). → tâche 1.
2. Dictée d'un seul mot corrigé (« Grok » → « Groq ») : aucun mot commun, mais très proche : gardée. → tâche 1.
3. Texte collé deux fois dans le même champ : celui du curseur, sinon rien. → tâche 2.
4. Nouvelle dictée avant la fin du suivi : l'ancienne se ferme avec une lecture, sans toucher à la nouvelle. → tâche 2.
5. `learned.bin` d'avant ce plan : relu sans perte. → tâche 4.

## I. Tâches

### Tâche 1 : textes plausibles
**Files :** Modify `src-tauri/src/learning.rs`. Mots : séparés par espaces, apostrophes et traits d'union, en minuscules, lettres et chiffres seulement ; longueurs en caractères.
**Produit :** `pub fn plausible(inserted: &str, kept: &str) -> bool`. Règle 4, plus : dictée de 3 mots ou moins → distance de caractères (`distance`) ≤ 0,6 suffit à la place des 35 %.
- [ ] Test `plausible_keeps_corrections_and_refuses_other_texts` :

| Collé | Gardé | Attendu |
|---|---|---|
| Je veux. | un mail de 70 mots | faux |
| Et pourquoi as-tu besoin d'un workflow ? | Ask Codex to do anything | faux |
| Alors je vais utiliser Kubernetes. | a | faux |
| Je vais utiliser Langfuse pour ça. | Je vais utiliser | faux |
| Comment va Yonathan ? | (vide) | faux |
| Cloud code | Claude code | vrai |
| Grok | Groq | vrai |
| dis-moi quels modes existent | dis-moi quels mods existent | vrai |
| Bonjour Stéphane, je parle des clients et du prix. | Bonjour Stefan, je parle des clients et des tarifs élevés. | vrai |

- [ ] Vérification hors dépôt sur les 115 textes réels : 17 refusés, n° 315 gardé.
- [ ] Commit `fix(learning): another text is never taken for the dictation`.

### Tâche 2 : boucle de suivi
**Files :** Modify `learning.rs` (`observe`, `finish`, `End`), `context/windows.rs` (`watch_destination`, 50 000 caractères), `context/mod.rs`, `actions.rs:1614`.
**Consomme :** `plausible` ; `Finished.element` et `Finished.id` (plan 2), passés à `observe(app, field, inserted, history_id, app_name, terminal, element, id)`.
**Produit :** `struct Watch<R: Fn() -> Option<String>, K: Fn() -> bool, C: Fn() -> Instant>` pilotable en test ; `enum End { Window, NextDictation, Sent, Emptied, AroundChanged, OtherText, NotFound, Unreadable, NotWatched(&'static str) }` ; `fn interval(since_start: Duration, since_change: Duration) -> Duration`.
- [ ] Tests avec horloge et lectures factices : `two_failed_reads_then_a_good_one_goes_on` ; `three_failed_reads_end_unreadable` ; `a_fix_at_three_minutes_is_kept` ; `enter_then_empty_is_sent` ; `emptied_by_hand_keeps_nothing` (dernière lecture = début du texte) ; `text_typed_after_sending_is_never_kept` ; `an_unknown_caret_needs_one_single_copy` ; `interval` : 300 ms à 5 s, 2 s à 30 s sans changement, 300 ms à 30 s si changé à 25 s.
- [ ] Autre élément focalisé au démarrage que celui du relâchement → `NotWatched("focus")`. Chaque suivi garde **son** élément et sa génération (plus de `WATCHED` unique) ; un démarrage ou une lecture d'une génération dépassée est refusé. Test `an_old_watch_never_reads_the_new_field`.
- [ ] Entrée : `K: Fn() -> bool` rend vrai si Entrée a été pressée depuis le dernier appel, alimenté par le sondage à 50 ms.
- [ ] Commit `fix(learning): the field is watched for five minutes, failed reads tolerated`.

### Tâche 3 : terminal
**Files :** Modify `learning.rs` (`collapse_spaces`, `locate_tail`, `region_near`).
**Produit :** `pub fn collapse_spaces(text: &str) -> String`. Elle sert **seulement à chercher** : `find_collapsed(text: &str, wanted: &str) -> Option<Range<usize>>` rend la position dans le texte d'origine. La région garde donc ses vraies lignes (`region_near` inchangée : la sortie affichée dessous n'y entre jamais).
- [ ] Tests : `a_wrapped_terminal_line_is_found` (« … le plan\n  correctement » contre « … le plan correctement ») ; `the_agent_prompt_is_not_the_dictation` ; tests terminal existants gardés.
- [ ] Commit `fix(learning): wrapped terminal text is found`.

### Tâche 4 : motif par dictée et leçons stockées
**Files :** Modify `managers/history.rs`, `learning.rs` (`Memory`), `judge.rs`, `actions.rs`, `lib.rs`.
**Produit :** colonne `learning_end TEXT` ; `set_learning_end(id: i64, end: &str)` ; `HistoryEntry.learning_end: Option<String>` (avec `ENTRY_COLUMNS`, la lecture des lignes, `tray.rs:976` et les autres constructeurs ; chaque changement émet `Updated`, comme `set_entry_app`) ; `Lesson { inserted, kept, app, at: i64, tries: u32 }` ; `Memory.lessons: Vec<Lesson>` (`#[serde(default)]`) ; `pub fn resume_lessons(app: &AppHandle)` au démarrage.
- [ ] Tests : `an_old_memory_loads_without_lessons` ; `lessons_expire_after_7_days_and_cap_at_50` ; `a_lesson_leaves_once_answered` ; `clearing_rules_clears_lessons` ; colonne ajoutée deux fois sans erreur ; motif enregistré quand rien n'est suivi (application exclue, mot de passe **sans** entrée d'historique → aucun motif).
- [ ] Journal et motifs sans texte. Retirer le commentaire `shortcut:` des essais en mémoire.
- [ ] Commit `feat(learning): every dictation says how it ended, lessons survive a restart`.

### Tâche 5 : historique à l'écran
**Files :** Modify `HistorySettings.tsx`, `bindings.ts`, 26 traductions (fr et en traduits, autres en anglais).
- [ ] Une ligne discrète par dictée, sans couleur seule ; textes : « Appris : X », « Rien à apprendre », « Pas relu : champ illisible », « Pas relu : texte introuvable », « Pas relu : autre texte dans le champ », « Pas relu : fenêtre changée », « Non suivi : application exclue », « Leçon en attente ». Entrées anciennes : aucune ligne.
- [ ] Playwright (simulé) : chaque motif a un texte ; motif inconnu → aucune ligne.
- [ ] Commit `feat(history): why a dictation taught something or nothing`.

### Tâche 6 : revue, build, installation, nettoyage
- [ ] Revue Terra, corrections, tests, build `fast`, sauvegarde, installation.
- [ ] Nettoyage en une transaction, Handy Flow fermé : `kept_text = NULL` pour 183, 184, 185, 186, 187, 188, 189, 190, 214, 237, 238, 254, 263, 265, 266, 267, après contrôle que chaque texte est encore celui relevé en B. Attendu : 16 lignes, aucune autre.
- [ ] Avec toi : cas de la partie F, puis 30 dictées. Objectif : un motif sur 30/30 ; au moins 60 % relues (43 % aujourd'hui, 115 sur 269).


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/03-v2.md`.
**À corriger dans ce plan (défauts qui expliquent les faux du constat)**
- Champ vide à la première lecture : attendre que le collage atterrisse avant de conclure « vidé » ; les 10 « champ vidé » en viennent peut-être (A1).
- Texte indicatif (placeholder, « Ask Codex… ») lu comme du texte : 5 des 17 faux (A2).
- Délai UIA = suivi tué : voie de lecture dédiée (`Lane::Watch`, plan 2) (A3, C1).
- « Curseur » plafonné à 1 000 caractères : mauvaise copie suivie (A4).
- Sans texte après la dictée, la frappe suivante entre dans la région ; terminal coupé trop tôt (A5, A6).
- Ordre : délimiter la région (alignement) puis juger la plausibilité (règle 4) (C3).
- Entrée puis texte toujours là : continuer, mais la région s'arrête à la nouvelle ligne (C4).
- Identité d'élément UIA instable dans Chrome : `NotWatched("focus")` risque de refuser à tort (C2).
- Bit faible de `GetAsyncKeyState` peu fiable : à confirmer (C6).
- Leçons stockées : relance au démarrage étalée (1 / 10 s), plafond 4 000 caractères (C8).
- Ajouter `Ambiguous` et `TooLong` aux fins (A9, C9).
**Améliorations retenues**
- Différence d'espaces seule = pas une correction (A7) ; plafond de taille vers le juge (A8).
- Échec de lecture mesuré en durée (B1) ; dernière lecture stable (B4) ; recherche tolérante aux blancs partout (B3).
- Cas T01–T07 de Willow comme tests ; lecture bloquée = un des 3 échecs (passe 1).

## K. Mise à jour du 27/11 (logs du 26–27/11)
**Ce que le journal corrige dans la section J**
- « Champ vidé » (16 suivis sur 32) **n'est pas une panne dans claude.ai** : c'est l'envoi du message. Le texte gardé y est bien enregistré (n° 392 à 396). L'hypothèse A1 (« collage pas encore arrivé ») ne vaut pas pour les applis de chat ; la garder seulement pour les champs qui se vident sans envoi.
- « Introuvable » (n° 411) : claude.ai retire les retours à la ligne du texte collé (`pasted U+000A vs field U+0045`). La recherche tolérante aux blancs (B3) règle ce cas : confirmé.

**À ajouter**
- **Journal** : un numéro de candidat sur les lignes « garde-fou » et « verdict », sans le mot, pour relier un rejet à sa raison (`learning.rs:511`, `learning.rs:1811`, `validator.rs:120`). 30 min.
- Le validateur passe à `clef-omni` (plan 4, N1) : délai 15 s suffit ; le juge de style reste un modèle de chat.
