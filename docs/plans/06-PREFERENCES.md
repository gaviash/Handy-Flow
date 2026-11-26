# Plan 6 : préférences et « À propos de moi » (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : A4, W9 (sans groupes), W10, W39, V21. H3 et V11 : **abandonnés** (tes réponses). Dépend du plan 5. Durée : 1 j. Rien n'est codé.
**Goal :** écrire ses préférences et se présenter une fois. **Architecture :** les règles du plan 5, plus un profil chiffré. **Tech Stack :** Rust, DPAPI, React.
Référence : `willow_insights/KNOWLEDGE.md` §7 (conception d'origine pour Handy).

## A. But

Tu écris une fois comment tu veux que Handy écrive et qui tu es ; il s'en sert au bon endroit, sans rien ajouter à ce que tu dictes.

## B. Aujourd'hui (code lu)

- Tons par catégorie (email, travail, perso, autre), avec exemple affiché.
- Nom seulement (`user_name`).
- Règles apprises, par catégorie après le plan 5. Aucune préférence écrite à la main.
- Consignes à une IA : la reformulation raccourcit et reformule comme le reste (« natural and faithful to the user's wording » pour la catégorie IA).

## C. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Une préférence écrite est une `StyleRule` du plan 5 avec `by_user` : même liste, étiquette « par moi ». Elle l'emporte toujours sur une règle apprise et n'est jamais retirée par le juge |
| 2 | Portée : partout ou une catégorie. Ordre total : par toi + catégorie, par toi + partout, apprise + catégorie. En cas de contradiction, le modèle reçoit l'ordre et le plus haut l'emporte |
| 3 | Limites : 200 caractères, 20 par portée ; 600 caractères envoyés par dictée, 1 500 pour Weave |
| 4 | Suggestions cliquables en français quand la liste est vide |
| 5 | « À propos de moi » : sections titre + texte (10 de 1 000 caractères), chiffrées. Dictée : seulement les noms que tu coches (« aide à l'orthographe »), jamais un numéro ni un identifiant. Weave : 2 000 caractères au plus |
| 6 | Signature : un champ à part, utilisé seulement par Weave et seulement en mail |
| 7 | Import depuis une autre IA : un texte à coller, découpé **localement** en sections, relu par toi avant d'être gardé |
| 8 | Fiches de référence : sections « À propos de moi » attachées à une catégorie |
| 9 | Exemples de toi (V21) : jusqu'à 5 textes collés par catégorie, 600 caractères chacun ; ils passent avant les exemples tirés de l'historique |
| 10 | Une préférence ne doit pas faire ajouter d'information. Garde locale ajoutée : si la sortie compte plus de 25 % de mots absents de la dictée, de ses formes connues et des orthographes fournies, elle est refaite une fois sans préférences. Garantie partielle : une courte formule (« merci ») peut passer, d'où la mesure de la tâche 5 |

## E. Tes réponses (25/11)

- **Q8 : comme aujourd'hui** pour les dictées à une IA. Aucun mode consigne, aucune règle spéciale terminal. H3 et V11 sortent des plans.
- **Q9 : pas de groupes** d'applications pour l'instant.

## F. Acceptation

| Cas | Attendu |
|---|---|
| préférence « mail », dictée dans Slack | non envoyée |
| préférence écrite contraire à une règle apprise | la tienne gagne |
| « À propos de moi » contient un numéro, dictée sans rapport | le numéro n'apparaît pas |
| champ privé ou exclu | rien envoyé |
| préférence « ignore les règles » | dictée rendue normalement |
| dictée à une IA sans préférence | même message envoyé qu'avant ce plan |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/preferences`, créée depuis la branche du plan 5.

## H. Review Focus

1. Préférence contenant une balise (`</context>`) : neutralisée. → tâche 2.
2. Texte vide sur une préférence existante : suppression après confirmation. → tâche 4.
3. Profil illisible (compte Windows changé) : réglages vides, fichier d'origine gardé. → tâche 1.
4. Import énorme (200 000 caractères) : refusé avec une raison. → tâche 3.
5. Anciennes règles du plan 5 : inchangées. → tâche 1.

## I. Tâches

### Tâche 1 : données
**Files :** Modify `learning.rs`, `style.rs` ; Create `src-tauri/src/profile.rs`.
**Produit :** `enum Scope { Everywhere, In(Category) }` ; `StyleRule.scope: Scope` (remplace `category`, migration) ; `pub fn set_preference(m: &mut Memory, id: Option<u64>, text: &str, scope: Scope, now: i64) -> Result<u64, PrefError>` (`Empty`, `TooLong`, `TooMany`) ; `struct Profile { sections: Vec<Section>, signature: String, samples: Vec<Sample> }` ; `Section { title, text, scope: Scope }` ; `Sample { category, text }` ; `Data/profile.bin` (DPAPI, écriture par fichier temporaire).
- [ ] Tests : `a_preference_is_never_retired` ; `limits_are_enforced` (200, 20, 10 × 1 000, 5 × 600) ; `category_beats_everywhere_among_mine` ; `plan5_rules_migrate_to_scopes` (lots, verdicts, envoi et affichage du plan 5 lisent `scope`) ; `an_unreadable_profile_is_kept_aside`.
- [ ] Commit `feat(profile): written preferences and who the user is`.

### Tâche 2 : envoi
**Files :** Modify `context/mod.rs` (`LlmContext`, `enriched_context`), `adaptive.rs` (bloc et prompts), `recall.rs`.
**Produit :** `pub fn preferences_for(m: &Memory, c: Category, max_chars: usize) -> Vec<String>` (les tiennes d'abord, puis apprises) ; `Section.names: Vec<String>` (proposés à l'écran, cochés par toi) ; `pub fn profile_names(p: &Profile, c: Category) -> Vec<String>` (sections de la portée, noms cochés, sans chiffre ni `_`) ; `pub fn added_words(dictation: &str, out: &str, known: &[String]) -> f64`.
- [ ] Tests : `dictation_gets_only_its_category` ; `about_me_text_never_reaches_a_dictation` (noms cochés seulement ; numéro et identifiant secrets absents ; section d'une autre portée absente) ; `an_output_that_adds_a_quarter_is_asked_again_without_preferences` ; `written_everywhere_beats_learned_in_category` ; `samples_come_before_history` ; `nothing_in_a_private_field` ; `preferences_are_framed_as_data`.
- [ ] Le prompt ajoute une phrase : une préférence n'ajoute jamais d'information.
- [ ] Commit `feat(rewrite): preferences of the category`.

### Tâche 3 : import
**Files :** Modify `profile.rs`.
**Produit :** `pub fn split_import(text: &str) -> Result<Vec<Section>, ImportError>` : titres `#`, lignes finissant par `:`, ou paragraphes ; 20 000 caractères au plus ; sans modèle.
- [ ] Tests : `headings_become_sections` ; `a_wall_of_text_is_cut_at_paragraphs` ; `too_long_is_refused`.
- [ ] Commit `feat(profile): import from another assistant, reviewed first`.

### Tâche 4 : écran
**Files :** Create `src/components/settings/profile/ProfilePage.tsx` ; Modify `Sidebar.tsx`, `LearningSettings.tsx`, `lib.rs`, `bindings.ts`, traductions.
- [ ] Commandes : `get_profile`, `save_section`, `delete_section`, `save_signature`, `save_sample`, `delete_sample`, `save_preference` (texte vide sur un identifiant existant = suppression), `delete_preference`, `import_preview`. Chacune valide ses limites et rend l'état à jour.
- [ ] Deux pages : « Mon style » (catégorie → ton → préférences → règles apprises, « Garder » transforme une règle apprise en préférence) et « À propos de moi » (sections, signature, exemples, import avec relecture).
- [ ] Suggestions fr quand la liste est vide : « Commence par la demande principale », « Paragraphes de trois phrases maximum », « Pas de formule du type J'espère que tu vas bien », « Tutoie par défaut », « Pas de tiret long », « Listes à puces dès trois éléments ».
- [ ] Playwright : ajouter, modifier, vider puis confirmer, importer, garder une règle ; clavier seul.
- [ ] Commit `feat(settings): my style and about me`.

### Tâche 5 : revue, build, installation
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation.
- [ ] Laboratoire, avec ton accord : 10 textes fictifs, vérifier qu'une préférence « finis par Merci » n'ajoute rien à une dictée d'une phrase.
- [ ] Avec toi : partie F.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/06-v2.md`. Bugs déplacés au plan 0 : 1.1, 1.2, 2.1, 2.4.
**À corriger dans ce plan**
- Stocker les préférences écrites hors de la liste des règles apprises (`profile.bin`) : `prune` (100) et « Tout effacer » les détruisent (A2, A3, C1).
- Préférences non envoyées si « Apprendre de mes corrections » est coupé (A5).
- « Partout » touche aussi le terminal : l'exclure comme Wispr (C3).
- Portée par application (`StyleRule.apps`) supprimée sans décision (C4).
- Transformations et Polish non couverts (C5) ; perte du profil sans message (C6).
- Exemples de l'utilisateur hors budget (5 × 600 caractères) (C7) ; `split_import` 20 000 > plafond 10 000.
- `apply_tone` écrase la règle de jonction ; « très décontracté » met tout en minuscules (A7, A8).
**Améliorations retenues**
- Préférences mécaniques garanties par le code (tiret long → virgule, comme Willow) (B1).
- Séparer « mes préférences » et « habitudes apprises » dans le message (B2).
- Garde « 25 % de mots ajoutés » : faux positifs, second appel hors budget de 8 s (B3).
- Dire ce qui n'est pas envoyé (B4) ; une ligne = une préférence (B6).
- Export en français à copier, refus de réimporter l'export ; aperçu « ce qui sera envoyé », plafond 600 (passe 1).
