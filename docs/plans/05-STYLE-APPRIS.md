# Plan 5 : style appris et apprentissage depuis l'historique (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : K4, S1 à S6, W6, H4, H5, W23. Dépend des plans 3 et 4. Durée : 2 j. Rien n'est codé.
**Goal :** des règles de style générales, par catégorie, tirées de ce que tu gardes. **Architecture :** étude par lots en arrière-plan, règles avec preuves, exemples réels. **Tech Stack :** Rust, DPAPI, React.

## A. But

Après une semaine, tes textes te ressemblent sans que tu les corriges, et tu vois pourquoi.

## B. Constats

- 7 règles en 10 jours, toutes vues une fois, **aucune active**. Elles portent sur une phrase précise et ne peuvent pas se répéter.
- Le juge ne voit que les textes corrigés. Une fois le plan 3 nettoyé, il reste **98 textes gardés valides**, presque tous acceptés tels quels ou à peine corrigés : le signal est là, mais ignoré.
- Les exemples donnés à la reformulation sont surtout ses propres sorties : elle s'imite.
- Expressions fréquentes : sur 250 dictées sans fuite, les suites de 2 à 4 mots les plus dites sont banales (« ce que tu peux », « il n'y a pas », « s'il te plaît »). **Aucune expression personnelle n'en sort** par comptage.
- Tics que la reformulation enlève déjà : « et ensuite » (16 fois dit, 5 gardé), « par rapport » (9, 3), « et voilà » (9, 3), « enfin bref » (6, 1).
- VS Code et terminal : 195 dictées sur 254. Le style de tes consignes à une IA n'est pas celui de tes mails.

## C. Conception

| # | Décision (mienne, corrigeable) | Règle |
|---|---|---|
| 1 | Étude par lots de 10 textes **d'une même catégorie** (mail, travail, perso, IA et code, document, autre), gardés tels quels **et** corrigés, en arrière-plan avec les essais du plan 3 | S4, W6, coût |
| 2 | Le juge reçoit les règles actuelles de la catégorie et rend trois listes : à créer, à renforcer, **à retirer** (contredites) | S2, W6 |
| 3 | Une règle est générale : réutilisable sur un autre sujet, sans citer une phrase ; 160 caractères au plus ; 12 par catégorie au plus | S2 |
| 4 | Une règle garde l'identifiant de **tous** ses soutiens, et 2 extraits courts affichables, chiffrés et effaçables | S1 |
| 5 | Les 7 règles actuelles sont effacées : trop précises | S2 |
| 6 | Exemples pour la reformulation : uniquement des textes **réellement gardés** et valides, même catégorie ; 3 au plus ; jamais une sortie non relue | S5 |
| 7 | Réponse non conforme du juge (une des trois listes absente ou mal formée) → nouvel essai | S3 |
| 7 bis | Texte admissible = motif de fin du plan 3 qui permet d'apprendre. Anciennes entrées sans motif : exclues, sauf l'étude initiale, où `plausible` les trie et où tu donnes ton accord | S5 |
| 8 | Bouton « Apprendre de ça » sur une dictée de l'historique : avant, après, modifiable ; il nourrit mots (plan 4) et style | H4 |
| 9 | « Signaler » une dictée ratée : mots faux, manquants, en trop, mal formulé ; note facultative ; la version corrigée devient un exemple | W23 |
| 10 | Une fois à l'installation : étude des 98 textes valides, par catégorie, avec le compte de jetons affiché avant de lancer | H5 |
| 11 | Réglage et effacement : liste par catégorie, « Garder », « Retirer », tout effacer | — |

Coût estimé : 10 textes de 300 caractères font environ 1 500 jetons par lot, réponse et raisonnement compris à mesurer. Le lot initial : environ 10 lots.

## D. Découpage du code

- `style.rs` (nouveau) : `StyleBatch`, `study(batch, rules) -> Verdicts { create, reinforce, retire }`, files par catégorie stockées dans `learned.bin`.
- `judge.rs` : garde `ask` et les essais ; l'ancien juge par correction disparaît.
- `learning.rs` : `StyleRule { rule, category, evidence, count, active, last_seen }`, migration (anciennes règles effacées).
- `managers/history.rs` : `kept_examples(category, since)` ; `recall.rs` (`similar_dictations` sur ces seuls textes).
- `HistorySettings.tsx` (apprendre, signaler), `LearningSettings.tsx`, traductions, `bindings.ts`.

## E. Tes réponses (25/11)

- **Q6 : « dit → écrit »** : déplacé au plan 4. Le plan 5 ne compte plus d'expressions.
- **Q7 : appliquer et prévenir.** Une règle devient active à **3 textes différents** qui la soutiennent ; annonce avec « Annuler » ; une règle annulée ou retirée par toi ne revient jamais (texte normalisé retenu).

Les formules de mail et les tics que tu n'as pas cochés ne sont pas cherchés exprès ; une règle de style peut quand même en parler si 3 textes la soutiennent.

## F. Acceptation

| Cas | Attendu |
|---|---|
| 10 mails gardés, différents, tous finis par « Bien cordialement » | une règle pour la catégorie mail, rien ailleurs |
| règle contredite par les 10 textes suivants | retirée |
| texte faux (plan 3) | jamais envoyé au juge, jamais un exemple |
| champ privé ou exclu | rien appris |
| Groq refuse | lot repris, rien de perdu |
| bouton « Apprendre de ça » | mot et style proposés depuis ta version |
| mesure avant et après | 20 textes rejoués : moins d'écart avec ce que tu gardes, sinon la règle est retirée |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/style-appris`, créée depuis la branche du plan 4. Prompt du juge rédigé pour Handy, en anglais comme les autres ; règles rendues en français.

## H. Review Focus

1. Texte gardé contenant une consigne (« ignore les règles ») : donnée, jamais suivie. → tâche 2.
2. Textes presque identiques (distance de mots < 0,3) : un seul soutien. → tâche 2.
3. Règle qui change le sens ou ajoute du contenu (« signe Gavriel ») : refusée hors mail, et en mail seulement si tes textes gardés la portent. → tâche 2.
4. Effacement de l'historique : les preuves liées disparaissent. → tâche 3.
5. Catégorie changée d'application : la règle suit la catégorie, pas l'application. → tâche 1.

## I. Tâches

### Tâche 1 : données
**Files :** Create `src-tauri/src/style.rs` ; Modify `learning.rs`, `recall.rs`, `managers/history.rs`, `context/mod.rs`.
**Produit :**
```rust
pub enum Category { Email, Work, Personal, AiCode, Document, Other } // depuis Content
pub struct Evidence { pub history_id: i64, pub excerpt: String, pub at: i64 } // 120 caractères
pub struct StyleRule { pub id: u64, pub rule: String, pub category: Category, pub supports: Vec<i64> /* ids d'historique */, pub evidence: Vec<Evidence> /* 2 au plus */, pub by_user: bool, pub last_seen: i64 }
impl StyleRule { pub fn active(&self) -> bool } // by_user, ou 3 soutiens distincts
pub struct Memory { /* … */ pub rules: Vec<StyleRule>, pub refused_rules: Vec<String>, pub style_queue: Vec<i64> /* ids d'historique */ }
pub fn kept_examples(&self, category: Category, since: i64, max: usize) -> Result<Vec<PastDictation>>
```
Colonne `category` ajoutée à l'historique (écrite à la dictée ; anciennes entrées : déduite de `app`).
- [ ] Tests : `old_rules_are_dropped_once` ; `examples_are_kept_texts_of_the_category` (3 au plus, jamais `post_processed_text`, jamais plausible = faux) ; `category_from_an_old_app_label` (`chrome · mail.google.com` → Email, `code` → AiCode, `windowsterminal` → AiCode).
- [ ] Commit `feat(style): rules by category, with their evidence`.

### Tâche 2 : étude par lots
**Files :** Create `style.rs` (suite) ; Modify `judge.rs` (ancien `run` supprimé, `ask` gardé), `learning.rs` (`observe` met l'identifiant en file).
**Produit :** `pub struct Verdicts { create: Vec<NewRule>, reinforce: Vec<Support>, retire: Vec<Support> }` ; `NewRule { rule, texts: Vec<usize> }` ; `Support { rule: u64, texts: Vec<usize> }` (identifiant stable, pas une position) ; `pub fn batch_message(...) -> String` ; `pub fn apply(m: &mut Memory, v: Verdicts, batch: &Batch, now: i64) -> Vec<LearnedNews>` ; `BATCH = 10`, `ACTIVE_AT = 3`, `RULE_CHARS = 160`, `RULES_PER_CATEGORY = 12`.
- [ ] Tests : `three_distinct_texts_activate` ; `two_do_not` ; `near_identical_texts_count_once` ; `a_rule_deleted_during_the_call_stays_deleted` ; `a_text_deleted_during_the_call_gives_no_evidence` ; `a_reply_missing_a_list_is_asked_again` ; `a_contradicted_rule_is_retired` (3 textes contre, aucun pour dans le lot) ; `a_refused_rule_never_returns` (casse, accents, ponctuation ignorés) ; `a_rule_quoting_a_sentence_is_refused` (plus de 6 mots consécutifs d'un texte) ; `an_unknown_rule_or_text_voids_the_item` ; `by_user_rules_are_never_retired` ; `texts_are_framed_as_data`.
- [ ] Lot lancé à 10 textes d'une catégorie, ou au bout de 7 jours s'il y en a 5 ; un seul lot en vol ; essais du plan 3. Ordre : texte gardé écrit, puis identifiant mis en file ; retiré de la file une fois étudié seulement. `ask` reçoit un contrôle de forme `fn(&str) -> bool` à la place de la première liste du schéma.
- [ ] Laboratoire hors dépôt, avec ton accord le moment venu : 3 lots réels, pour régler le prompt (environ 3 appels).
- [ ] Commit `feat(style): learned in batches from what is kept`.

### Tâche 3 : usage, effacement, annonce
**Files :** Modify `adaptive.rs`, `context/mod.rs` (`enriched_context`), `learning.rs`, `managers/history.rs`.
- [ ] Reformulation : règles actives de la catégorie, 600 caractères au plus, règles de toi d'abord ; 3 exemples au plus. Transformations : pareil.
- [ ] Tests : `only_the_category_rules_are_sent` ; `deleting_a_dictation_removes_its_support` (suppression à la main, en lot, par rétention ou totale ; sous 3 soutiens, la règle redevient inactive) ; `undo_refuses_the_rule` ; `nothing_from_a_private_field`.
- [ ] Commit `feat(style): the category's own rules and examples`.

### Tâche 4 : historique et réglages
**Files :** Modify `HistorySettings.tsx`, `LearningSettings.tsx`, `VocabularySettings.tsx`, `lib.rs`, `bindings.ts`, traductions.
**Produit :** commandes `teach_from(id: i64, corrected: String)` (mots, plan 4, puis file de style) ; `report_dictation(id: i64, kind: Report, note: String)` (`Wrong`, `Missing`, `Extra`, `Phrasing`, colonne `report`) ; `study_history(dry_run: bool) -> StudyPlan { batches, tokens }`.
- [ ] « Apprendre de ça » : avant, après modifiable, aperçu des mots proposés ; « Signaler » : quatre choix et une note. Au clavier comme à la souris.
- [ ] Règles par catégorie : texte, « vu N fois », preuves, « Garder », « Retirer ».
- [ ] Tests : `teaching_a_private_entry_is_refused` (aucune entrée privée n'existe : identifiant inconnu → erreur) ; Playwright : les deux dialogues, focus rendu à la fermeture.
- [ ] Commit `feat(history): teach from a dictation, report a bad one`.

### Tâche 5 : revue, build, installation, étude initiale
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation.
- [ ] Avec ton accord : `study_history(dry_run)` affiche lots et jetons, puis lancement. Attendu : règles visibles avec preuves, aucune active sous 3 textes.
- [ ] Mesure : 20 textes rejoués avec et sans règles ; écart moyen avec le texte gardé (distance de mots). Sans amélioration, la règle en cause est retirée.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
**Plan à revoir avant de coder.** Détail : `willow/05-v2.md`. Bugs déplacés au plan 0 : 1.1, 2.4.
**Défaut de fond : l'apprentissage s'apprend lui-même**
- 73 % des textes « gardés » ne sont pas retouchés ; les règles actives réinjectées dans la reformulation comptent comme leurs propres soutiens (A3).
- Correction : stocker le texte réellement collé et un ratio d'édition (`pasted_text`, `edit_ratio`) à la fin du suivi (B1, comme Wispr §8.6 et Willow beforeEdit/afterEdit) ; ne compter que les soutiens non circulaires ; lots en paires (collé, gardé) (B2).
- 44/98 textes commencent par une minuscule à cause de `junction.rs` : remettre casse et ponctuation avant le juge (A2).
- 113/117 textes viennent de VS Code/terminal/ChatGPT : catégories mail presque vides (C2).
**Autres corrections**
- Le texte gardé absorbe la frappe après la dictée (A1, voir plan 3).
- Annuler une règle ne l'interdit pas ; l'égalité de texte ne suffit pas (A6, B4).
- Juge : envoyer seulement les phrases modifiées (A7, Willow/Wispr) ; exclure le texte jugé des dictées passées (A8).
- Similarité des exemples trop lâche : deux mots de sujet minimum (A9).
- Fermer l'observation au collage suivant, pas à l'appui (A11).
- Rétention 30 jours : les règles rares de mail se désactivent (C3).
- Taille 6 mots à 4 000 caractères ; doublon Dice ≥ 0,40 = simple filet (B3).

## K. Mise à jour du 27/11 (logs du 26–27/11)
- **Juge trop lent** : 9 appels, 7 délais dépassés (45 s) avec Cloudflare Gemma réflexion activée. Pour Cloudflare, « High » envoie seulement `enable_thinking: true` (`llm_client.rs:117`) : c'est la réflexion sans limite qui coûte. Le juge rédige des leçons, il reste un modèle de chat : **réflexion désactivée** par défaut, ou un modèle rapide.
- Les essais du juge ne vivent qu'en mémoire (5 s, 30 s, 2 min, 10 min, 1 h, 4 h, `judge.rs:136`) : quitter l'app perd la leçon. La file persistante du plan 3 (leçons stockées) doit couvrir aussi les leçons de style.
- « Corriger un mot depuis l'historique » est **avancé au plan 4** (N9). Le bouton « apprendre de ça » pour le style reste ici.
- Les signaux A et B du plan 4 (N10, N11) apprennent aussi de sorties du LLM : mêmes garde-fous contre l'apprentissage circulaire que ceux de la section J.
