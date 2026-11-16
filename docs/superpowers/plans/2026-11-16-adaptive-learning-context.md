# Apprentissage adaptatif et contexte enrichi — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal :** apprendre les mots mal entendus et les règles de style de l'utilisateur, et envoyer au LLM un contexte riche et pertinent. Envoyer aussi du contexte aux modèles ASR qui l'acceptent, après l'avoir mesuré, et de façon paramétrable. Enfin, corriger l'icône jaune de la notice « mots appris ».

**Architecture :** la couche locale (`learning.rs`) apprend les noms, termes et mots mal entendus. Un juge LLM (`judge.rs`) analyse en arrière-plan chaque correction et en tire des règles de style. Un module de rappel (`recall.rs`) choisit, à chaque dictée, les mots appris et les dictées passées pertinents, que `adaptive::house_user_message` ajoute au bloc `<context>`.

**Tech Stack :** Rust (Tauri 2, rusqlite, serde, tokio), React/TypeScript (zustand, i18next), Groq via `llm_client`.

**Spec :** `docs/superpowers/specs/2026-11-16-adaptive-learning-context-design.md`

## Global Constraints

- Les logs ne contiennent jamais de texte dicté : longueurs, compteurs et motifs seulement.
- `learned.bin` reste chiffré DPAPI, écrit dans un fichier temporaire puis renommé.
- Le nouveau code reste dans les fichiers adaptatifs. Les fichiers d'upstream (`actions.rs`, `managers/history.rs`, `lib.rs`) ne reçoivent que le câblage minimal.
- Plafonds : `WORDS_MAX = 2000`, `RULES_MAX = 100`. Une entrée en attente expire après 30 jours. Au plus 40 termes, 5 dictées passées (300 caractères chacune, moins de 30 jours), 1500 caractères avant le curseur et 500 après.
- Juge : Groq, modèle `adaptive::GROQ_SPARE_MODEL`, délai max 15 s, aucun retry. Sans clé Groq, il ne tourne pas.
- Le contexte enrichi ne concerne que le prompt maison. Les dictées passées et les règles ne partent que si `context_send_enabled && learning_enabled`.
- Tout texte d'interface passe par i18n : clé dans `en` puis dans toutes les locales (`bun run check:translations`).
- Tests Rust : `cd src-tauri && cargo test --lib <filtre>`. `bindings.ts` se régénère en lançant l'app en debug (`bun run tauri dev`).

## Review Focus

1. **Ancien `learned.bin`** (tableau JSON de la version portable) : il doit se charger sans perte. Test dans la tâche 2.
2. **Correction de sens** (« mardi » → « jeudi », « oui » → « non ») : rien ne doit être appris, ni par la couche locale ni par le juge. Tests dans les tâches 3 et 7.
3. **Réponse du juge invalide, vide, en retard ou erreur 429** : rien n'est appris, et la dictée suivante n'est pas ralentie. Test dans la tâche 7.
4. **Historique vide ou sans dictée de la même app** : le bloc `<context>` ne contient pas de section vide. Test dans la tâche 6.
6. **Audio sans parole envoyé à un ASR qui a reçu du contexte** : le modèle récite le vocabulaire ou le prompt au lieu de rendre un texte vide. Ce cas doit être détecté comme un écho. Test dans la tâche 11.
5. **Titre de fenêtre contenant un e-mail ou un numéro** : il passe par `redact()` comme le texte autour du curseur. Test dans la tâche 6.

---

### Task 1 : notice « mots appris » en vert dans la fenêtre principale

Cause : [App.tsx:115](../../../src/App.tsx) envoie `level: "warning"`, [NoticePanel.tsx:30](../../../src/components/notices/NoticePanel.tsx) ne connaît que `error` et `warning`, et `NoticeChip` compte tout ce qui n'est pas une erreur comme un avertissement. La pilule de l'overlay, elle, est déjà correcte (`CheckBadgeIcon`, `#22c55e`).

**Files :**
- Modify : `src/App.tsx:115`, `src/components/notices/NoticePanel.tsx:30-34`, `src/components/footer/NoticeChip.tsx:26-27`

- [ ] **Step 1 :** dans `App.tsx`, passer `level: "warning"` à `level: "info"` pour l'événement `learned-words`.
- [ ] **Step 2 :** dans `NoticeRow`, quand `level === "info"`, afficher `CheckBadgeIcon` (importé de `@/components/icons/NoticeBadgeIcon`) en vert `text-[#22c55e]`, la même couleur que l'overlay. Sinon, garder le comportement actuel.
- [ ] **Step 3 :** dans `NoticeChip`, exclure `level === "info"` du calcul : `warnings = notices.filter(n => n.level === "warning").length`. Si `errors + warnings === 0`, la puce ne s'affiche pas.
- [ ] **Step 4 :** lancer `bun run lint` et `bunx tsc --noEmit`. Attendu : aucune erreur.
- [ ] **Step 5 :** vérification manuelle avec `bun run tauri dev`, fenêtre des réglages ouverte. Corriger un nom dans un champ observé, par exemple Bloc-notes « Quen » → « Qwen ». Attendu : un toast avec une coche verte, sans puce « 1 avertissement » dans le pied de page.
- [ ] **Step 6 :** commit `fix(adaptive): learned-words notice shows the green check in the main window`.

---

### Task 2 : nouveau format de mémoire (mots, règles, état en attente)

**Files :**
- Modify : `src-tauri/src/learning.rs` (struct `Learned`, `with_memory`, `save`, commandes `get_learned_words`, `delete_learned_word`, `clear_learned_words`, `export_learned_words`, `count`, `terms`)

**Interfaces :**
- Produces :
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
  #[serde(rename_all = "lowercase")]
  pub enum WordKind { #[default] Term, Misheard }
  // Learned gagne : #[serde(default)] pub kind: WordKind, #[serde(default = "yes")] pub active: bool
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
  pub struct StyleRule { pub rule: String, pub count: u32, pub active: bool, pub last_seen: i64, #[serde(default)] pub apps: Vec<String> }
  #[derive(Debug, Clone, Default, Serialize, Deserialize)]
  pub struct Memory { pub words: Vec<Learned>, #[serde(default)] pub rules: Vec<StyleRule> }
  pub const WORDS_MAX: usize = 2000;
  pub const RULES_MAX: usize = 100;
  fn parse_memory(json: &[u8]) -> Memory          // accepte aussi l'ancien tableau de Learned
  pub(crate) fn prune(m: &mut Memory, now: i64)   // expiration 30 j des inactifs, puis plafonds
  fn with_memory<T>(app: &AppHandle, f: impl FnOnce(&mut Memory) -> T) -> T
  ```
- Les commandes existantes gardent leur signature et opèrent sur `m.words`.

- [ ] **Step 1 : écrire les tests en échec** dans `learning::tests` :
  ```rust
  #[test]
  fn old_memory_array_still_loads() {
      let m = parse_memory(br#"[{"term":"Claude","heard":["Cloud"],"count":2,"last_seen":0}]"#);
      assert_eq!(m.words.len(), 1);
      assert_eq!(m.words[0].kind, WordKind::Term);
      assert!(m.words[0].active);
      assert!(m.rules.is_empty());
  }
  #[test]
  fn prune_expires_pending_and_caps() {
      let day = 86_400;
      let mut m = Memory::default();
      m.words.push(Learned { term: "log".into(), heard: vec!["gag".into()], count: 1, last_seen: 0, kind: WordKind::Misheard, active: false });
      m.words.push(Learned { term: "Qwen".into(), heard: vec!["Quen".into()], count: 1, last_seen: 0, kind: WordKind::Term, active: true });
      prune(&mut m, 31 * day);
      assert_eq!(m.words.iter().map(|w| w.term.as_str()).collect::<Vec<_>>(), vec!["Qwen"]);
      m.words = (0..WORDS_MAX + 5).map(|i| Learned { term: format!("w{i}"), heard: vec![], count: 1, last_seen: i as i64, kind: WordKind::Term, active: true }).collect();
      prune(&mut m, 0);
      assert_eq!(m.words.len(), WORDS_MAX);
      assert!(!m.words.iter().any(|w| w.term == "w0")); // le moins utilisé et le plus ancien part
  }
  ```
- [ ] **Step 2 :** `cargo test --lib learning::` → FAIL (types absents).
- [ ] **Step 3 :** implémenter les types et `parse_memory` (essayer `Memory`, puis `Vec<Learned>`, sinon `Memory::default()`). Implémenter `prune` : retirer les entrées inactives dont `last_seen < now - 30 j`, puis, au-dessus du plafond, retirer par ordre croissant `(count, last_seen)`. Faire passer `with_memory` sur `Memory` ; `save` sérialise `Memory` et appelle `prune` avant d'écrire. Adapter les commandes à `m.words`. `terms()` ne renvoie que les mots `active`.
- [ ] **Step 4 :** `cargo test --lib learning::` → PASS, y compris les tests existants.
- [ ] **Step 5 :** commit `feat(adaptive): learned memory holds word kinds, pending words and style rules`.

---

### Task 3 : la couche locale apprend les mots mal entendus et logge ses refus

**Files :**
- Modify : `src-tauri/src/learning.rs` (`candidates`, `learn`, tests existants de `candidates`)

**Interfaces :**
- Consumes : `WordKind`, `Memory` (tâche 2).
- Produces :
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct Candidate { pub term: String, pub heard: String, pub kind: WordKind }
  pub fn candidates(inserted: &str, fin: &str) -> Vec<Candidate>
  pub fn is_misheard_pair(heard: &str, term: &str) -> bool   // aussi utilisée par le juge (tâche 7)
  pub(crate) fn record_word(m: &mut Memory, c: &Candidate, now: i64) -> bool  // true si le mot devient actif ou gagne une forme
  fn learn(app: &AppHandle, inserted: &str, fin: &str) -> Vec<Candidate>       // retourne ce qui a été vu
  ```

- [ ] **Step 1 : écrire les tests en échec** :
  ```rust
  #[test]
  fn misheard_common_word_is_learned_after_two_corrections() {
      let c = candidates("regarde le gag du serveur", "regarde le log du serveur");
      assert_eq!(c, vec![Candidate { term: "log".into(), heard: "gag".into(), kind: WordKind::Misheard }]);
      let mut m = Memory::default();
      assert!(!record_word(&mut m, &c[0], 0));
      assert!(!m.words[0].active);
      assert!(record_word(&mut m, &c[0], 1));
      assert!(m.words[0].active);
  }
  #[test]
  fn meaning_changes_are_never_learned() {
      assert!(candidates("la réunion est mardi", "la réunion est jeudi").is_empty());
      assert!(candidates("je dis oui merci", "je dis non merci").is_empty());
      assert!(candidates("on part à deux", "on part à trois").is_empty());
      assert!(!is_misheard_pair("mardi", "jeudi"));
      assert!(is_misheard_pair("gag", "log"));
  }
  ```
  Remplacer les tuples `(term, heard)` des tests existants par `Candidate { …, kind: WordKind::Term }`.
- [ ] **Step 2 :** `cargo test --lib learning::` → FAIL.
- [ ] **Step 3 :** implémenter.
  - Dans `candidates`, quand `looks_like_term` est faux, la paire devient `Misheard` si le bloc compte de 1 à 3 mots, si `!affix` et si `is_misheard_pair(&hk, &tk)`.
  - `is_misheard_pair` : `distance ≤ 0.7` et aucun mot des deux côtés n'est dans `MEANING_WORDS`. Cette liste regroupe les jours et les mois (réutiliser `adaptive::FR_COMMON_NAMES`, à rendre `pub(crate)`), les nombres en lettres de zéro à vingt, cent, mille (fr et en), oui/non/si/yes/no, ne/pas/jamais/rien/plus/not/never, et les pronoms personnels fr et en.
  - `Term` reste actif tout de suite (comportement actuel). `Misheard` est actif à partir de `count >= 2`.
  - Chaque bloc rejeté logge `log::debug!("Learning: rejected ({reason})")`, avec `reason` parmi `rewrite`, `no_substitution`, `too_long`, `unbounded`, `affix`, `too_far`, `meaning_word`, `not_a_term`. Aucun texte dans le log.
  - `learn` n'annonce (`announce`, événement `learned-words`) que les mots devenus actifs ou ayant gagné une forme.
- [ ] **Step 4 :** `cargo test --lib learning::` → PASS.
- [ ] **Step 5 :** commit `feat(adaptive): learn misheard common words after two corrections, log why a fix is not learned`.

---

### Task 4 : l'historique garde le texte final de l'utilisateur

**Files :**
- Modify : `src-tauri/src/managers/history.rs:40` (`ADDED_COLUMNS`), nouvelles méthodes près de `set_entry_app` (l. 468)
- Modify : `src-tauri/src/actions.rs:1280-1296` (récupérer l'id), `:1384` (appel `observe`)
- Modify : `src-tauri/src/learning.rs` (`observe`)

**Interfaces :**
- Produces :
  ```rust
  // managers/history.rs
  pub struct PastDictation { pub text: String, pub app: Option<String>, pub timestamp: i64 }
  pub fn set_kept_text(&self, id: i64, text: &str) -> Result<()>
  pub fn past_dictations(&self, since: i64) -> Result<Vec<PastDictation>>  // text = COALESCE(kept_text, post_processed_text, transcription_text), plus récentes d'abord, au plus 500
  // learning.rs
  pub fn observe(app: &AppHandle, field: &Field, inserted: &str, history_id: Option<i64>, app_name: String)
  ```

- [ ] **Step 1 : écrire le test en échec** dans `managers/history.rs` (base SQLite en mémoire, comme les tests existants du fichier) : `set_kept_text` puis `past_dictations(0)` renvoie le texte gardé. Une entrée sans `kept_text` renvoie son `post_processed_text`. Une entrée plus ancienne que `since` est exclue.
- [ ] **Step 2 :** `cargo test --lib history::` → FAIL.
- [ ] **Step 3 :** ajouter `"kept_text"` à `ADDED_COLUMNS` et implémenter les deux méthodes. Dans `actions.rs`, garder `entry.id` du `save_entry` réussi dans une variable `history_id: Option<i64>`, et passer `history_id` et `app_label` à `observe`. Dans `observe`, en fin de fenêtre, si `last != inserted` et `history_id` existe, appeler `set_kept_text` via `app.state::<Arc<HistoryManager>>()`.
- [ ] **Step 4 :** `cargo test --lib history::` → PASS, puis `cargo build`.
- [ ] **Step 5 :** commit `feat(adaptive): history keeps the text the user kept after a dictation`.

---

### Task 5 : rappel des mots et des dictées pertinents

**Files :**
- Create : `src-tauri/src/recall.rs` (déclaré dans `lib.rs` à côté de `mod learning;`)
- Modify : `src-tauri/src/learning.rs` (`terms` est remplacé par `terms_for`, ajout de `active_rules`)

**Interfaces :**
- Consumes : `Learned`, `StyleRule` (tâche 2), `PastDictation` (tâche 4).
- Produces :
  ```rust
  // recall.rs
  pub fn relevant_terms(words: &[Learned], transcription: &str, max: usize) -> Vec<String>
  pub fn similar_dictations(past: &[PastDictation], transcription: &str, app: &str, now: i64) -> Vec<String>
  // learning.rs
  pub fn terms_for(app: &AppHandle, transcription: &str) -> Vec<String>          // relevant_terms(.., 40)
  pub fn active_rules(app: &AppHandle, app_name: &str) -> Vec<String>           // actives, sans apps ou liées à app_name
  ```

- [ ] **Step 1 : écrire les tests en échec** dans `recall::tests` :
  ```rust
  #[test]
  fn a_term_heard_in_the_dictation_comes_first() {
      let w = |t: &str, h: &str, c| Learned { term: t.into(), heard: vec![h.into()], count: c, last_seen: 0, kind: WordKind::Term, active: true };
      let words = vec![w("Postgres", "PostGer", 9), w("Claude", "Cloud", 1)];
      let out = relevant_terms(&words, "ouvre cloud code", 1);
      assert_eq!(out, vec![r#"Claude (heard as "Cloud")"#]);
  }
  #[test]
  fn similar_dictations_pick_by_words_then_same_app() {
      let d = |t: &str, a: &str, ts| PastDictation { text: t.into(), app: Some(a.into()), timestamp: ts };
      let now = 40 * 86_400;
      let past = vec![
          d("le log du serveur est vide", "Slack", now - 10),
          d("acheter du pain", "Code", now - 5),
          d("très vieux log serveur", "Slack", 0), // plus de 30 jours
      ];
      let out = similar_dictations(&past, "regarde le log du serveur", "Code", now);
      assert_eq!(out, vec!["le log du serveur est vide", "acheter du pain"]);
  }
  ```
- [ ] **Step 2 :** `cargo test --lib recall::` → FAIL.
- [ ] **Step 3 :** implémenter.
  - `relevant_terms` : mots actifs seulement. D'abord ceux dont une forme entendue est à une distance `≤ 0.34` (`learning::distance` sur `learning::key`) d'un mot ou d'un bigramme de la transcription. Ensuite les autres, triés par `(count desc, last_seen desc)`. Format `term (heard as "a", "b")`, comme `terms()` aujourd'hui.
  - `similar_dictations` : seulement les dictées de moins de 30 jours. D'abord les 3 meilleurs scores de Jaccard sur les mots en minuscules (hors mots vides fr/en courts, score > 0). Ensuite les 2 plus récentes de la même app non encore prises. Chaque texte est tronqué à 300 caractères sur une limite de mot, et le résultat compte au plus 5 dictées.
- [ ] **Step 4 :** `cargo test --lib recall::` → PASS.
- [ ] **Step 5 :** commit `feat(adaptive): recall the learned words and past dictations relevant to a dictation`.

---

### Task 6 : contexte enrichi et prompt

**Files :**
- Modify : `src-tauri/src/context/mod.rs:399-443` (`LlmContext`, constantes, `llm_context`)
- Modify : `src-tauri/src/adaptive.rs` (`house_user_message`, `HOUSE_PROMPT` règles 2 et 10)
- Modify : `src-tauri/src/actions.rs:1165-1190` (câblage)

**Interfaces :**
- Consumes : `terms_for`, `active_rules`, `similar_dictations`, `past_dictations`.
- Produces : `LlmContext` gagne `pub app: String, pub window: String, pub site: Option<String>, pub past: Vec<String>, pub rules: Vec<String>`. `BEFORE_CHARS = 1500`, `AFTER_CHARS = 500`.

- [ ] **Step 1 : écrire les tests en échec** dans `adaptive::tests` :
  ```rust
  #[test]
  fn context_carries_app_window_past_and_rules() {
      let mut c = test_context(); // helper du module de test : LlmContext minimal
      c.app = "slack.exe".into(); c.window = "général - Slack".into();
      c.past = vec!["le log du serveur est vide".into()];
      c.rules = vec!["Supprimer « donc voilà » en début de phrase.".into()];
      let m = house_user_message("regarde le gag", Some(&c));
      assert!(m.contains("app: slack.exe"));
      assert!(m.contains("window: «général - Slack»"));
      assert!(m.contains("past dictations:\n- «le log du serveur est vide»"));
      assert!(m.contains("style rules:\n- Supprimer"));
  }
  #[test]
  fn empty_sections_are_left_out() {
      let m = house_user_message("bonjour", Some(&test_context()));
      assert!(!m.contains("past dictations"));
      assert!(!m.contains("style rules"));
      assert!(!m.contains("window:"));
  }
  ```
  Dans `context::tests`, ajouter un test : `llm_context` sur une `Capture` dont le titre contient `jean@exemple.fr` renvoie un `window` sans l'adresse, comme le fait `redact()`.
- [ ] **Step 2 :** `cargo test --lib adaptive:: context::` → FAIL.
- [ ] **Step 3 :** implémenter.
  - `llm_context` remplit `app` (`capture.process`), `window` (`redact(&capture.title)`, 200 caractères maximum) et `site` (`capture.host`).
  - `house_user_message` ajoute `app:`, `window: «…»`, `site:`, puis `past dictations:` et `style rules:`, une ligne `- ` par élément, en sautant les sections vides. Appliquer `unframe` partout.
  - Dans `actions.rs`, remplacer `learning::terms(&ah)` par `learning::terms_for(&ah, &transcription)`. Si `learning_enabled`, remplir aussi `c.past` (`recall::similar_dictations(&hm.past_dictations(now - 30 j)?, …)`) et `c.rules` (`learning::active_rules`).
  - Ajouter à la règle 10 de `HOUSE_PROMPT` :
    > "app"/"window"/"site" tell where the text goes: adapt the register to them. "past dictations" are texts the user wrote before: use them to recognize the user's words and turns of phrase, never copy them. A dictated word that matches a form listed as "heard as" MUST become that term. "style rules" are the user's own preferences: apply them unless they change the meaning of the dictation.
  - Ajouter à la règle 2 :
    > Verbal tics used as crutches ("donc voilà", "en fait", "du coup", "genre", "tu vois", "you know") are removed when they carry no meaning.
- [ ] **Step 4 :** `cargo test --lib` → PASS.
- [ ] **Step 5 :** commit `feat(adaptive): send the app, window, past dictations and style rules to the rewrite`.

---

### Task 7 : juge LLM en arrière-plan

**Files :**
- Create : `src-tauri/src/judge.rs` (déclaré dans `lib.rs`)
- Modify : `src-tauri/src/learning.rs` (fin de `observe`, `apply_verdicts`)

**Interfaces :**
- Consumes : `Candidate`, `is_misheard_pair`, `record_word`, `Memory`, `StyleRule`.
- Produces :
  ```rust
  // judge.rs
  pub const JUDGE_PROMPT: &str;   // système : classer chaque changement entre texte collé et texte gardé
  #[derive(Debug, Deserialize, PartialEq)] #[serde(rename_all = "snake_case")]
  pub enum VerdictKind { Misheard, StyleRule, Filler, ContentChange, Unclear }
  #[derive(Debug, Deserialize, PartialEq)]
  pub struct Verdict { pub kind: VerdictKind, pub heard: Option<String>, pub term: Option<String>,
                       pub rule: Option<String>, pub same_as: Option<usize>, pub consistent_with_history: bool }
  pub fn judge_message(inserted: &str, kept: &str, app_name: &str, rules: &[String], past: &[String], already: &[Candidate]) -> String
  pub fn parse_verdicts(reply: &str) -> Vec<Verdict>     // {"items":[…]} ; invalide → vec![]
  pub async fn run(app: AppHandle, inserted: String, kept: String, app_name: String, already: Vec<Candidate>)
  // learning.rs
  pub(crate) fn apply_verdicts(m: &mut Memory, verdicts: &[Verdict], app_name: &str, now: i64) -> Vec<String>  // règles ou mots devenus actifs
  ```
  `rules` dans `judge_message` liste toutes les règles, en attente comprises, numérotées à partir de 0. `same_as` renvoie à ce numéro.

- [ ] **Step 1 : écrire les tests en échec** dans `judge::tests` et `learning::tests` :
  ```rust
  #[test]
  fn invalid_replies_learn_nothing() {
      assert!(parse_verdicts("").is_empty());
      assert!(parse_verdicts("not json").is_empty());
      assert!(parse_verdicts(r#"{"items":[{"kind":"weird"}]}"#).is_empty());
  }
  #[test]
  fn verdicts_respect_the_guards() {
      let v = |kind, heard: Option<&str>, term: Option<&str>, rule: Option<&str>, same_as, ok| Verdict {
          kind, heard: heard.map(Into::into), term: term.map(Into::into), rule: rule.map(Into::into), same_as, consistent_with_history: ok };
      let mut m = Memory::default();
      // un changement de sens n'apprend rien, même proposé comme misheard
      apply_verdicts(&mut m, &[v(VerdictKind::Misheard, Some("mardi"), Some("jeudi"), None, None, true),
                               v(VerdictKind::ContentChange, None, None, None, None, true)], "Code", 0);
      assert!(m.words.is_empty() && m.rules.is_empty());
      // une règle isolée et non cohérente reste en attente, puis s'active à la 2e occurrence
      apply_verdicts(&mut m, &[v(VerdictKind::StyleRule, None, None, Some("Préférer « de manière claire et concise »."), None, false)], "Code", 0);
      assert!(!m.rules[0].active);
      let on = apply_verdicts(&mut m, &[v(VerdictKind::StyleRule, None, None, Some("…"), Some(0), false)], "Code", 1);
      assert!(m.rules[0].active && m.rules[0].count == 2 && on.len() == 1);
      // une règle cohérente avec l'historique s'active tout de suite
      apply_verdicts(&mut m, &[v(VerdictKind::Filler, None, None, Some("Supprimer « donc voilà »."), None, true)], "Code", 2);
      assert!(m.rules[1].active);
  }
  ```
- [ ] **Step 2 :** `cargo test --lib judge:: learning::` → FAIL.
- [ ] **Step 3 :** implémenter.
  - `apply_verdicts` :
    - `Misheard` passe par `is_misheard_pair` puis `record_word` avec `kind: Misheard`, ce qui impose aussi les 2 occurrences ;
    - `StyleRule` et `Filler` : si `same_as` désigne une règle existante, `count += 1` ; sinon une nouvelle règle est créée avec `count: 1`. Elle est active si `count >= 2 || consistent_with_history` ;
    - `ContentChange` et `Unclear` sont ignorés.
  - `run` :
    - fournisseur `groq` pris dans `settings.post_process_providers` ; clé `post_process_api_keys["groq"]`, retour immédiat si elle est vide ;
    - appel de `llm_client::send_chat_completion_with_schema` avec `GROQ_SPARE_MODEL`, un schéma JSON `{"items":[Verdict]}` et `disable_reasoning = true`, dans `tokio::time::timeout(15 s)` ;
    - en cas d'erreur ou de délai dépassé, `log::debug!("Learning: judge skipped ({reason})")` puis retour ;
    - sinon `apply_verdicts` dans `with_memory`, `save`, et `announce` des éléments activés.
  - Fin d'`observe` : si le texte a été modifié, `learning_enabled` et `context_send_enabled`, appeler `tauri::async_runtime::spawn(judge::run(app, inserted, fin, app_name, candidats_vus))`.
  - `JUDGE_PROMPT` décrit les 5 catégories. Une règle est une instruction courte et générale, en français, réutilisable. Un changement d'information est un `content_change`. Dans le doute, la réponse est `unclear`.
- [ ] **Step 4 :** `cargo test --lib` → PASS.
- [ ] **Step 5 :** commit `feat(adaptive): an LLM judge learns style rules from the user's corrections`.

---

### Task 8 : interface des règles de style

**Files :**
- Modify : `src-tauri/src/learning.rs` (commandes), `src-tauri/src/lib.rs:728` (enregistrement)
- Modify : `src/components/settings/post-processing/LearningSettings.tsx`
- Modify : `src/i18n/locales/*/translation.json` (bloc `settings.postProcessing.learning`, l. 753 dans `en`)

**Interfaces :**
- Produces : `#[tauri::command] get_style_rules(app) -> Vec<StyleRule>` et `delete_style_rule(app, rule: String)`, enregistrées à côté de `learning::get_learned_words`.

- [ ] **Step 1 :** ajouter les deux commandes et les enregistrer. `clear_learned_words` vide aussi `rules`.
- [ ] **Step 2 :** lancer `bun run tauri dev` une fois pour régénérer `bindings.ts`. Attendu : `commands.getStyleRules` et `commands.deleteStyleRule` existent, et le type `Learned` contient `kind` et `active`.
- [ ] **Step 3 :** dans `LearningSettings`, sous la liste des mots, ajouter une section « Règles de style » avec la même présentation et la suppression unitaire. Marquer « en attente » les mots et règles inactifs. Nouvelles clés : `learning.rules.title`, `learning.rules.empty`, `learning.pending`, d'abord dans `en`, puis traduites dans chaque locale.
- [ ] **Step 4 :** lancer `bun run lint`, `bunx tsc --noEmit` et `bun run check:translations`. Attendu : aucune erreur.
- [ ] **Step 5 :** commit `feat(adaptive): style rules list in learning settings`.

---

### Task 9 : banc d'intégration ASR avec et sans contexte (mesure préalable)

Ce banc tourne **avant** le câblage ASR (tâches 10 à 12). Il dit quels modèles gagnent à recevoir du contexte. Spec §7.

**Files :**
- Create : `src-tauri/tests/asr_context_bench.rs` (test d'intégration `#[ignore]`, lancé à la main)
- Create : `src-tauri/tests/fixtures/asr/` : `cases.json` (`[{ "wav": "...", "expected": "...", "terms": ["log", "Claude"] }]`), plus 3 WAV courts enregistrés pour l'occasion avec les termes « log », « Claude Code » et « Handy Flow »
- Create : `adaptive/bench/asr-cases.ps1`, qui construit un `cases.json` local à partir de `data/recordings` et du texte gardé de `history.db`. Ce fichier sort dans le scratchpad et n'est **jamais** commité, car il contient des dictées réelles.

**Interfaces :**
- Consumes : l'API publique `transcribe_cpp` (`Model::load`, `Model::supports(Feature::…)`, `RunOptions`, `WhisperRunOptions`).
- Produces : un rapport texte par modèle (`model`, `features`, `wer_without`, `wer_with`, `terms_ok_without`, `terms_ok_with`, `ms_without`, `ms_with`).

- [ ] **Step 1 :** écrire `fn wer(reference: &str, hypothesis: &str) -> f64`, un Levenshtein sur les mots normalisés avec `learning::key`, recopié dans le test pour ne pas exposer `learning`, et son test unitaire non ignoré : `wer("le log du serveur", "le gag du serveur") == 0.25`.
- [ ] **Step 2 :** écrire `asr_context_bench` (`#[ignore]`). Il lit `HANDY_ASR_MODELS` (chemins GGUF séparés par `;`) et `HANDY_ASR_CASES` (par défaut, les fixtures). Pour chaque modèle et chaque cas, il lance deux runs :
  - sans contexte ;
  - avec le contexte du cas, formaté selon les capacités : `vocabulary` si `Feature::Vocabulary`, `prompt` si `Feature::ContextPrompt`, `WhisperRunOptions::initial_prompt` si `Feature::InitialPrompt`. Ce dernier ne se combine jamais avec `vocabulary` ou `prompt`.
  Le test affiche le rapport. Il n'échoue que sur une erreur d'exécution.
- [ ] **Step 3 :** lancer `cargo test --test asr_context_bench -- --ignored --nocapture` avec parakeet-ultra et Qwen3-ASR 0.6B (déjà dans `data/huggingface`), puis Whisper small si on le télécharge. Attendu : parakeet ne montre aucune différence (aucune capacité) ; les autres modèles montrent l'effet du contexte.
- [ ] **Step 4 :** consigner le rapport dans `adaptive/bench/asr-context-results.md`, avec la règle du spec : un modèle n'est recommandé que si les termes s'améliorent, que le WER ne se dégrade pas de plus d'un point et que la latence n'augmente pas de plus de 15 %.
- [ ] **Step 5 :** commit `test(adaptive): ASR context bench, with and without context, per model`.

---

### Task 10 : module `asr_context` (pur, sans Tauri)

**Files :**
- Create : `src-tauri/src/asr_context.rs` (déclaré dans `lib.rs`)
- Modify : `src-tauri/src/recall.rs` (ajout de `frequent_expressions`)

**Interfaces :**
- Produces :
  ```rust
  #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
  pub struct AsrCaps { pub vocabulary: bool, pub context_prompt: bool, pub initial_prompt: bool }
  #[derive(Debug, Clone, Default)]
  pub struct AsrSources { pub custom_words: Vec<String>, pub learned: Vec<String>, pub expressions: Vec<String>,
                          pub app: Option<String>, pub before: Option<String>, pub language: Option<String> }
  #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
  pub struct AsrContextSettings { pub enabled: bool, pub learned_words: bool, pub expressions: bool,
                                  pub screen: bool, pub max_terms: usize, pub capture_wait_ms: u64 }
  // Default : true, true, true, false, 50, 150
  #[derive(Debug, Clone, Default, PartialEq)]
  pub enum AsrPrompt { #[default] None, Context(String), Initial(String) }
  #[derive(Debug, Clone, Default, PartialEq)]
  pub struct AsrContext { pub vocabulary: Vec<String>, pub prompt: AsrPrompt }
  pub const PROMPT_MAX_CHARS: usize = 600;
  pub fn build(caps: AsrCaps, src: &AsrSources, cfg: &AsrContextSettings) -> AsrContext
  // recall.rs
  pub fn frequent_expressions(past: &[PastDictation], now: i64, min_count: usize, max: usize) -> Vec<String>  // appelée avec (3, 20)
  ```

- [ ] **Step 1 : écrire les tests en échec** dans `asr_context::tests` :
  ```rust
  fn src() -> AsrSources { AsrSources { custom_words: vec!["Handy".into()], learned: vec!["log".into(), "Claude".into()],
      expressions: vec!["de manière claire et concise".into()], app: Some("slack.exe".into()),
      before: Some("on regarde le serveur".into()), language: Some("fr".into()) } }
  #[test] fn no_capability_builds_nothing() {
      assert_eq!(build(AsrCaps::default(), &src(), &AsrContextSettings::default()), AsrContext::default()); }
  #[test] fn vocabulary_lists_custom_then_learned_words_capped() {
      let cfg = AsrContextSettings { max_terms: 2, ..Default::default() };
      let c = build(AsrCaps { vocabulary: true, ..Default::default() }, &src(), &cfg);
      assert_eq!(c.vocabulary, vec!["Handy", "log"]); assert_eq!(c.prompt, AsrPrompt::None); }
  #[test] fn context_prompt_cites_terms_and_expressions_but_not_the_screen_by_default() {
      let c = build(AsrCaps { context_prompt: true, ..Default::default() }, &src(), &AsrContextSettings::default());
      let AsrPrompt::Context(p) = c.prompt else { panic!() };
      assert!(p.contains("Claude") && p.contains("de manière claire et concise"));
      assert!(!p.contains("slack") && !p.contains("serveur"));
      assert!(p.chars().count() <= PROMPT_MAX_CHARS); }
  #[test] fn whisper_initial_prompt_never_comes_with_vocabulary() {
      let c = build(AsrCaps { vocabulary: true, initial_prompt: true, ..Default::default() }, &src(), &AsrContextSettings::default());
      assert!(matches!(c.prompt, AsrPrompt::Initial(_))); assert!(c.vocabulary.is_empty()); }
  #[test] fn disabled_or_sources_off_respected() {
      let caps = AsrCaps { context_prompt: true, vocabulary: true, ..Default::default() };
      assert_eq!(build(caps, &src(), &AsrContextSettings { enabled: false, ..Default::default() }), AsrContext::default());
      let c = build(caps, &src(), &AsrContextSettings { learned_words: false, expressions: false, ..Default::default() });
      assert_eq!(c.vocabulary, vec!["Handy"]); }
  ```
  Dans `recall::tests` : sur 3 dictées récentes contenant « de manière claire » et une vieille de 40 jours, `frequent_expressions(.., 3, 20)` renvoie `["de manière claire"]`. Les n-grammes faits uniquement de mots vides sont exclus.
- [ ] **Step 2 :** `cargo test --lib asr_context:: recall::` → FAIL.
- [ ] **Step 3 :** implémenter.
  - Termes = `custom_words`, puis `learned` si `learned_words`, dédoublonnés sans tenir compte de la casse, tronqués à `max_terms`.
  - Si `initial_prompt` est vrai, le résultat est `Initial(prompt)` et `vocabulary` reste vide (contrainte de `WhisperRunOptions`).
  - Sinon : `vocabulary` si la capacité existe, et `Context(prompt)` si `context_prompt`.
  - Prompt en français si `language == fr`, en anglais sinon : `"Vocabulaire : A, B. Expressions : « x », « y »."`. Ajouter `"Application : …. Texte précédent : …"` seulement si `screen`. Tronquer à `PROMPT_MAX_CHARS` sur une limite de mot.
- [ ] **Step 4 :** `cargo test --lib asr_context:: recall::` → PASS.
- [ ] **Step 5 :** commit `feat(adaptive): build ASR context (vocabulary and prompt) per model capability`.

---

### Task 11 : câblage dans la transcription et réglages

**Files :**
- Modify : `src-tauri/src/engine_supervisor/worker.rs:347-349` et `protocol.rs:158-162` (`supports_context_prompt: model.supports(Feature::ContextPrompt)`)
- Modify : `src-tauri/src/managers/transcription.rs:1332-1395` (`run_setup`), `:1613-1650` (`vocabulary_for_model` est remplacé par `asr_context::build`, `echoes_vocabulary` vérifie aussi les mots du prompt), `:986` (`transcribe_recording`)
- Modify : `src-tauri/src/settings.rs` (champ `asr_context: AsrContextSettings` avec `#[serde(default)]`)
- Modify : `src-tauri/src/actions.rs:1007-1104` (sources ASR avant `transcribe_recording`)

**Interfaces :**
- Consumes : `asr_context::{build, AsrCaps, AsrSources, AsrContext, AsrPrompt}`, `learning::terms_for`, `recall::frequent_expressions`.
- Produces :
  ```rust
  pub fn transcribe_recording(&self, samples: Vec<f32>, asr: &AsrSources) -> Result<Transcription>
  fn run_setup(info: &LoadedInfo, settings: &AppSettings, language: LanguageDecision, asr: &AsrSources) -> RunSetup
  ```
  Les autres appelants de `transcribe_recording` (fichier, CLI) passent `&AsrSources::default()`.

- [ ] **Step 1 : écrire les tests en échec** dans `transcription::tests`, à côté de `fuzzy_correction_is_skipped_exactly_when_vocabulary_was_sent` :
  - `run_setup` sur un `LoadedInfo` sans capacité donne des `RunOptions` sans `vocabulary`, sans `prompt` et sans `family` ;
  - avec `supports_context_prompt`, il remplit `prompt` ;
  - avec `supports_initial_prompt`, il met `family = Some(RunExtension::Whisper(WhisperRunOptions { initial_prompt: Some(_), .. }))` et laisse `vocabulary` vide ;
  - `echoes_vocabulary("Vocabulaire : log, Claude.", …)` sur la sortie d'un audio vide est détecté.
- [ ] **Step 2 :** `cargo test --lib transcription::` → FAIL.
- [ ] **Step 3 :** implémenter.
  - `run_setup` appelle `asr_context::build` avec les `AsrCaps` du modèle chargé et `settings.asr_context`. Il logge `asr_context: vocabulary_terms={n}, prompt_chars={n}, kind={none|context|initial}`, sans texte.
  - La correction floue des mots personnalisés est sautée quand `vocabulary` **ou** le prompt contient ces mots (`custom_words_prompted`).
  - Dans `actions.rs`, construire `AsrSources` avant `transcribe_recording` :
    - `learned` = `terms_for` sans la forme « heard as » (le terme seul) ;
    - `expressions` = `frequent_expressions(&hm.past_dictations(now - 30 j), now, 3, 20)`, gardé en cache 5 min ;
    - si `asr_context.screen`, attendre `capture_task` au plus `capture_wait_ms` (`tokio::time::timeout`) pour l'app et les 200 derniers caractères avant le curseur. Si le délai est dépassé, on continue sans.
  - Le tout uniquement si `learning_enabled`, sinon seulement les mots personnalisés, comme aujourd'hui.
- [ ] **Step 4 :** `cargo test --lib` → PASS, puis `cargo build`.
- [ ] **Step 5 :** commit `feat(adaptive): send learned words and frequent expressions to ASR models that take context`.

---

### Task 12 : réglages « Contexte de transcription »

**Files :**
- Modify : `src-tauri/src/learning.rs` ou `commands/` (commande `set_asr_context_settings(settings: AsrContextSettings)`, enregistrée dans `lib.rs`)
- Modify : `src/components/settings/post-processing/LearningSettings.tsx`, `src/i18n/locales/*/translation.json`

- [ ] **Step 1 :** ajouter la commande. Elle valide `max_terms` dans `1..=200` et `capture_wait_ms` dans `0..=1000`, sinon elle renvoie `Err`.
- [ ] **Step 2 :** dans `LearningSettings`, ajouter une section « Contexte de transcription » avec 4 interrupteurs (activé, mots appris, expressions, écran) et 2 champs numériques. Afficher sous le titre une ligne d'état selon le modèle chargé : « Le modèle actuel n'accepte pas de contexte » ou « … accepte : vocabulaire / prompt ». Nouvelles clés `learning.asr.*`, dans `en` d'abord puis dans toutes les locales.
- [ ] **Step 3 :** régénérer `bindings.ts` (`bun run tauri dev`). Lancer `bun run lint`, `bunx tsc --noEmit` et `bun run check:translations`. Attendu : aucune erreur.
- [ ] **Step 4 :** commit `feat(adaptive): ASR context settings`.

---

### Task 13 : vérification de bout en bout

- [ ] **Step 1 :** lancer `cd src-tauri && cargo test --lib && cargo clippy -- -D warnings`, puis `bun run format:check`. Attendu : tout passe.
- [ ] **Step 2 :** avec `bun run tauri dev`, dans Bloc-notes :
  - dicter « regarde le log », corriger « gag » en « log » deux fois. Attendu : « log » apparaît actif dans les réglages ;
  - une troisième dictée doit écrire « log » directement ;
  - dans le panneau de debug (`Ctrl+Shift+D`, contexte envoyé), on doit voir `app:`, `window:` et `past dictations:`.
- [ ] **Step 3 :** dicter une phrase avec des répétitions orales (« je pense que je pense que c'est bon »). Attendu : « Je pense que c'est bon. »
- [ ] **Step 4 :** relancer le bench `adaptive/bench` (`scenarios.ps1`) avant et après, et noter la latence médiane. Attendu : moins de 300 ms d'écart.
- [ ] **Step 5 :** relancer `asr_context_bench` (tâche 9) avec le câblage final, sur le modèle choisi par l'utilisateur. Attendu : les mêmes gains que la mesure préalable. Puis dicter « regarde le log » avec Qwen3-ASR, contexte ASR activé : les logs montrent `asr_context: … kind=context` et le texte est juste dès la transcription brute.
