# Plan 4 : mots et dictionnaire (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : K3, M1 à M5, W7, W8, W33, H8, V8. Dépend du plan 3. Durée : 1,5 à 2 j (« dit → écrit » ajouté). Rien n'est codé.
**Goal :** des mots appris une fois, gardés, utilisés à bon escient. **Architecture :** une seule liste `Learned` avec trois sortes (mot, mal entendu, dit → écrit). **Tech Stack :** Rust, DPAPI, React.

## A. But

Un mot que tu corriges est appris une fois, gardé, et utilisé seulement quand il faut. Tu peux aussi l'ajouter toi-même avec ses mauvaises écoutes.

## B. Constats

- 13 mots appris en 10 jours. Utilisations comptées : ADHD 3, Claude 4, Handy 3, Handy flow 3, Wispr flow 1, les autres 0.
- 22/11, 14 h 45 : 6 mots en attente refusés puis **supprimés**. Les sauvegardes des 16 et 17/11 en contiennent 3 : « polish » (polisses), « tests » (thèmes), « les expressions » (l'expression). Seul « polish » ressemble à un vrai terme mal entendu.
- « Stéphane → Stefan » perdu : une réécriture annule tout, y compris les noms.
- « C'est pas Gabriel, c'est Gavriel » devient « c'est pas Gavriel, c'est Gavriel » avec le vocabulaire (n° 314, essai du plan 1).
- Ton nom (`user_name`) va à la reformulation, **pas à Whisper**. Dans l'essai du plan 1, il était dans le vocabulaire : le plan 1 l'ajoute donc.
- Trois listes séparées : mots personnalisés (sans « entendu comme »), mots appris, nom.

## C. Conception

| # | Décision (mienne, corrigeable) | Règle |
|---|---|---|
| 1 | Réécriture → les paires isolées et proches restent candidates si elles ressemblent à un nom (majuscule hors début de phrase, chiffre, mot absent du lexique) ; le reste est ignoré | M1 |
| 2 | Un refus du validateur ne supprime plus un mot en attente : il reste en attente 30 jours ; deux corrections identiques l'activent, sauf mot grammatical | M2 |
| 3 | Forme entendue **et** mot corrigé tous deux courants (« modes → mods ») : jamais de remplacement automatique, seulement « maybe heard as » si la dictée contient la forme, dans l'application où elle a été corrigée (`apps` de la paire ; vide pour les anciens mots = partout) | M3, M4 |
| 4 | Une seule liste « Mots » : ajoutés par toi, appris, en attente. Ajout : mot juste + jusqu'à 10 formes entendues | W8, M5 |
| 5 | `custom_words` migrés vers cette liste, sans perte, une seule fois (`Memory.custom_migrated`), pour ne pas faire revenir un mot supprimé | W8 |
| 6 | Mot ajouté par toi : jamais oublié, jamais modifié par l'apprentissage, envoyé en premier | V8, H8 |
| 7 | Annoncé tout de suite, écrit quand l'annonce disparaît ; « Annuler » : aucun mot ajouté, seule la paire refusée est retenue. Fermeture avant : mot en attente | W7 |
| 8 | Paire contradictoire (A→B et B→A) : refusée, avec explication | W33 |
| 9 | Import et export, une ligne par mot : `juste ; forme1 | forme2` | W33 |
| 10 | Phrase qui oppose deux orthographes (« pas X, c'est Y ») : la reformulation garde les deux mots si l'ASR les donne. Quand l'ASR les confond déjà, rien à faire : limite écrite | n° 314 |
| 11 | « polish » (polisses) est restauré | M2 |
| 12 | **« Dit → écrit »** : une formule parlée s'écrit toujours d'une façon (« point MD » → « .md »). Ajout à la main ; appris quand la correction contient un point, une barre, un tiret bas, un chiffre, ou que des mots entiers deviennent un seul symbole ; validateur comme pour les mots | ta réponse, plan 5 |
| 13 | Application : la reformulation reçoit « dis : écris » ; sans reformulation, remplacement local en mots entiers. Formule d'un seul mot courant : jamais apprise, ajout à la main seulement | sécurité |

## D. Découpage du code

- `learning.rs` : `local_candidates` (réécriture), `apply_revalidation`, `record_word`, `Source::Manual`, `contradicts(words, heard, term) -> bool`, migration.
- `recall.rs` (`relevant_terms`), `replace_heard`, `validator.rs` (`fallback`).
- Consommateurs de `custom_words` : `actions.rs:1529`, `context/mod.rs:882`, `asr_context.rs`, `managers/transcription.rs:1302` et `:1984`, `transcription/job.rs:116`, `shortcut/mod.rs:995`, `CustomWords.tsx`, `VocabularySettings.tsx`.
- `LearningSettings.tsx`, traductions, `bindings.ts`.

## E. Tes réponses (25/11)

- **Q5 : rien de coché.** Les 13 mots sont gardés, « mods », « Band », « context » et « cache » compris. La règle 3 limite leur effet ; tu peux les supprimer dans Vocabulaire.
- **Expressions : « dit → écrit »** (règles 12 et 13). Les snippets existants (déclencheur dit seul → texte) ne changent pas.

## F. Acceptation

| Cas | Attendu |
|---|---|
| mail réécrit, « Stéphane » devenu « Stefan » | Stefan proposé, rien d'autre |
| mot refusé par le validateur, corrigé une 2ᵉ fois | actif |
| forme courante apprise dans VS Code, dite dans Gmail | reste telle quelle |
| « Claude » ajouté avec « Cloud, Klode » | une entrée, deux formes, jamais oubliée |
| ajout d'un mot déjà appris | fusion, aucune forme perdue |
| LLM indisponible, « Élo » | remplacé en mot entier, accents compris |
| « Annuler » pendant l'annonce | aucun mot ajouté ; la paire refusée est retenue |
| « crée un point MD », formule apprise | « crée un .md » |
| « un point important », même formule | inchangé |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/mots`, créée depuis la branche du plan 3. Compatibilité : `learned.bin` et `custom_words` d'avant relus sans perte.

## H. Review Focus

1. Formule incluse dans un mot (« point MDR ») : mots entiers seulement. → tâche 4.
2. Même mot ajouté à la main et appris : une seule entrée. → tâche 2.
3. Fermeture pendant l'annonce : mot en attente, pas perdu. → tâche 3.
4. Import d'un fichier mal formé : lignes valides prises, les autres comptées, rien d'effacé. → tâche 5.
5. Retour à une ancienne version : `custom_words` encore lisible (copié, pas déplacé). → tâche 2.

## I. Tâches

### Tâche 1 : candidats dans une réécriture, refus qui ne supprime plus
**Files :** Modify `learning.rs` (`local_candidates`, `apply_revalidation`, `record_decision`), `validator.rs`.
**Règle exacte :** réécriture = plus de 30 mots changés, ou plus de la moitié d'une dictée de 4 mots ou plus (seuils actuels). Alors seuls passent les runs d'**une** substitution, entourés de mots inchangés ou de bornes de ponctuation, `looks_like_term` vrai, `distance ≤ 0,6`. Refus du validateur : `Decision::Pending` si le mot est absent de la mémoire ou en attente, jamais `Rejected("refused")` avec effacement ; activation à `count ≥ 2`.
- [ ] Tests : `a_name_is_kept_from_a_rewritten_mail` (mail fictif, « Bérénisse → Bérénice » seul candidat) ; `a_full_rewrite_without_name_teaches_nothing` ; `a_refused_word_waits_and_a_second_fix_activates_it` ; `revalidation_never_deletes` ; `a_grammar_pair_never_activates`.
- [ ] Commit `fix(learning): a name survives a rewrite, a refused word waits`.

### Tâche 2 : une seule liste
**Files :** Modify `learning.rs`, `settings.rs`, les 9 consommateurs de la partie D.
**Produit :** `Source::{UserEdits, AppContext, Manual}` ; `WordKind::{Term, Misheard, Spoken}` ; `pub fn add_manual(m: &mut Memory, term: &str, heard: &[String], kind: WordKind, now: i64) -> Result<(), AddError>` (`Empty`, `TooLong`, `Contradicts(String)`) ; `pub fn manual_terms(app) -> Vec<String>` ; `fn migrate_custom_words(m: &mut Memory, words: &[String], now: i64) -> usize`.
`Learned.apps: Vec<String>` (`#[serde(default)]`) ; `relevant_terms(words, transcription, app, now, max)`.
- [ ] Tests : `custom_words_migrate_once` (deux passages, aucun doublon) ; `a_deleted_migrated_word_stays_deleted_after_restart` ; `a_common_pair_holds_in_its_app_only` ; `a_manual_word_is_never_pruned_or_renamed` ; `manual_terms_come_first` ; `adding_a_learned_word_merges_forms` ; `a_chain_back_is_refused` ; `an_older_memory_still_loads`.
- [ ] `custom_words` reste écrit tel quel dans les réglages (retour arrière), mais n'est plus lu après migration.
- [ ] Commit `feat(words): one list for added and learned words`.

### Tâche 3 : annonce puis écriture
**Files :** Modify `learning.rs` (`record_decisions`, `announce`), `notices.rs`, `overlay.rs`, `RecordingOverlay.tsx`, `lib.rs`, `bindings.ts`.
**Produit :** `Memory.announced: Vec<(Candidate, i64)>` ; `commit_announced(app)` (annonce disparue, remplacée, ou démarrage suivant) ; commande `undo_announced(term: String, heard: String)`.
- [ ] Tests : `undo_before_commit_leaves_no_word_and_refuses_the_pair` ; `an_announced_word_is_not_used_yet` ; `announced_words_commit_at_next_start`.
- [ ] La pastille d'annonce porte un bouton « Annuler » accessible à la souris ; durée 6 s.
- [ ] Commit `feat(learning): a word is written once its notice is gone`.

### Tâche 4 : dit → écrit et mots courants
**Files :** Modify `learning.rs`, `recall.rs`, `adaptive.rs` (prompt et bloc `spoken forms`), `actions.rs`.
**Produit :** `pub fn spoken_forms(words: &[Learned], transcription: &str) -> Vec<(String, String)>` ; `pub fn apply_spoken(text: &str, forms: &[(String, String)]) -> String` (mots entiers, insensible à la casse, plus longue d'abord, Unicode) ; `fn both_common(heard, term) -> bool`.
- [ ] Tests : « crée un point MD » → « crée un .md » ; « point MDR » et « un point important » inchangés ; « Élo » remplacé ; « modes » d'une autre application sans mention ; une formule à un mot courant refusée à l'apprentissage, acceptée à la main.
- [ ] Commit `feat(words): what is said one way and written another`.

### Tâche 5 : écran, import, export
**Files :** Modify `CustomWords.tsx` (devient la fenêtre d'ajout), `LearningSettings.tsx`, `VocabularySettings.tsx`, `learning.rs` (`parse_word_list`, `format_word_list`), traductions, `bindings.ts`.
- [ ] Champs : sorte (mot, dit → écrit), mot juste, formes entendues (+). 50 caractères par champ, 10 formes. Avertissement si une forme est un mot courant.
- [ ] Format : `juste ; forme1 | forme2`, `dit => écrit` pour les formules. Test `word_lists_round_trip`, `bad_lines_are_counted_not_fatal`.
- [ ] Commit `feat(words): add, edit, import and export in one page`.

### Tâche 6 : revue, build, installation
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation.
- [ ] Ajouter « polish » (polisses) par `add_manual`, sorte mot. Vérifier les 13 mots : tous présents, mêmes formes.
- [ ] Avec toi : partie F.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/04-v2.md`. Bugs du code actuel déplacés au plan 0 : 1.1, 4.1, 4.2, 4.3, 4.4.
**À corriger dans ce plan**
- Règle 7 « annoncé tout de suite, écrit quand l'annonce disparaît » : écrire tout de suite avec annulation exacte est plus simple ; l'écriture différée est un défaut chez Willow (C1).
- Règle 3 `both_common` par le lexique : se trompe sur les prénoms (Stéphane/Stefan) (C2, A11).
- Tâche 2 : supprimer `custom_words` après migration peut régresser ; garder une lecture de secours (C3).
- Le verdict du validateur écrase la sorte du mot : vraie cause de « mods/context/cache » (A4).
- Mots courants appris envoyés au prompt ASR : les exclure (A5).
- V8 à moitié planifié (C6) ; cas d'acceptation à ajouter (C7).
**Améliorations retenues**
- Remplacements déterministes autour de la reformulation (dit → écrit), tolérants à la ponctuation (B2, B3).
- Normalisation NFC à la lecture du champ (B5) ; casse des sigles et majuscules internes (B6).
- Ne pas perdre le suivi d'un mot juste avant la dictée suivante (B1).
- Cas glossaire et noms propres de Willow comme tests (passe 1).
**Question pour toi** : la règle 1 (candidats dans une réécriture) est plus audacieuse que Willow et Wispr (C5) : on la garde ?
