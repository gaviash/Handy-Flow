# Plan 1 : transcription propre

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**État :** complet, à valider (2026-11-24). Rien n'est codé. Idées : K1, T1 à T6.

**Goal :** le texte transcrit ne contient que ce que tu as dit : aucun morceau du prompt, aucune phrase perdue.

**Architecture :** le prompt Whisper ne contient plus que du vocabulaire. Une fonction pure retire une recopie en tête de transcription. Si tout est recopié, l'appel Groq repart une fois sans prompt ; ensuite, modèle local, comme aujourd'hui.

**Tech Stack :** Rust (Tauri), React, i18next ; Groq `whisper-large-v3`.

**Spec :** parties A à D ci-dessous. Durée : 3 à 4 h (26 fichiers de langue à nettoyer ; première estimation 2–3 h).

---

## A. Cause (prouvée)

1. Handy envoie à Whisper le vocabulaire, puis les « expressions fréquentes » entre guillemets, puis l'application, puis le texte avant le curseur, coupés à 600 caractères.
2. Reconstruits, ces prompts font 624 à 680 caractères : la coupe tombe dans une expression, guillemet ouvert. Whisper continue la liste.
3. 18 dictées touchées sur 39, du 22/11 au 24/11 à 19 h 21. Depuis 22 h 32, une sur dix commence par un guillemet, sans preuve de fuite : le défaut dépend de la longueur du prompt.
4. L'anti-écho ne rejette que si **tout** le texte est dans le prompt.

## B. Essai réel (24/11)

20 enregistrements × 4 prompts = 80 appels, mêmes paramètres que Handy, aucun échec.

| Prompt | Début pollué | Guillemets « | Mots | Temps médian / max |
|---|---|---|---|---|
| actuel (reconstruit) | **10 sur 20** | 41 | 1072 | 0,79 s / 5,93 s |
| **vocabulaire seul** | **0** | 0 | 963 | 0,45 s / 1,36 s |
| vocabulaire + texte précédent | 0 | 0 | 953 | 0,48 s / 1,29 s |
| aucun | 0 | 0 | 991 | 0,40 s / 1,58 s |

Gains du vocabulaire seul sur « aucun » : iOS (au lieu de « us »), « commit » (au lieu de « comité »), CI/CD, Gavriel. Pertes : n° 299 (trois morceaux), n° 301, n° 314 (« c'est pas Gavriel, c'est Gavriel » : plan 4).
Limites : vocabulaire reconstruit depuis `context_terms` (celui de la reformulation, avec ton nom), pas de transcription de référence, un essai par cas, prompt « actuel » reconstruit, texte précédent approximé.
Matériel : `backups/handy-flow-20261124-222402/` (audio dans `Data/recordings`, résultats `ab.json`, scripts `abtest.py` et `sim.py`). **Données privées** : ne pas supprimer avant la fin du plan, ne jamais copier dans le dépôt `Handy/`.

## C. Décisions

| # | Décision | Par |
|---|---|---|
| 1 | Vocabulaire seul : ni expressions, ni application, ni texte précédent | toi (application) ; essai (le reste) |
| 2 | La reformulation garde application, site et texte autour du curseur | inchangé |
| 3 | Recopie totale → même audio relancé **sans prompt** ; échec → modèle local | toi |
| 4 | Recopie puis vraie phrase → seule la copie est retirée | moi, annoncé |
| 5 | Réparer 301, 306, 313 ; supprimer 286, 291 | toi |
| 6 | La fonction « expressions fréquentes » disparaît ; elle est remplacée par « dit → écrit » (plan 4), ta réponse du 25/11 | toi |

Approches écartées : baisser le plafond (la coupe reste) ; filtrer les mots-outils (la liste entre guillemets reste imitable) ; aucun prompt (perd les noms et les termes).

**Revue Terra du plan (24/11) :** 7 remarques, toutes traitées.

**Hors plan :** relancer le modèle local sans prompt (aucune fuite locale mesurée) ; « c'est pas Gavriel » (plan 4) ; lire l'écran plus tôt (plan 2).

## D. Règles communes (Global Constraints)

- Branche `fix/transcription-propre`, créée depuis `main` (`50e81e38`). Commits locaux, jamais poussés sans accord. Fin de message : `Claude-Session: https://claude.ai/code/session_012CZubH248GKPyG3p6tC2kK`.
- Le journal ne contient jamais de texte dicté : longueurs et verdicts seulement.
- Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib`. Référence avant le plan : 534 réussis, 0 échec.
- Un seul build **de l'application** (`bun run tauri build`), à la fin (profil `fast`) ; `bun run build` n'est qu'une vérification du frontend, permise à chaque tâche. Aucune édition pendant le build.
- Aucun test **automatique** ne prend le clavier, le focus ou le micro. Les essais « avec toi » sont manuels, faits par toi, quand tu le décides.
- `PROMPT_MAX_CHARS` reste à 600.

## E. Review Focus

1. Dictée qui commence vraiment par des mots du vocabulaire (« Claude, Handy et Groq… ») : gardée entière. → tâche 2.
2. Audio sans parole : le second appel peut rendre une phrase inventée par Whisper. Le VAD coupe le silence en amont ; risque restant accepté, à surveiller en tâche 6. → tâche 6.
3. Réglages enregistrés avec `expressions` : relus sans erreur, autres valeurs gardées. → tâche 1.
4. Échap pendant le second appel : rien n'est collé. → tâche 3.
5. Vocabulaire vide : aucun prompt envoyé, pas « Vocabulaire : . ». → tâche 1.

---

### Tâche 0 : base propre

- [x] **Étape 1 :** travail K0 commité (`4cb2f7a4`) et fusionné dans `main` le 25/11.
- [ ] **Étape 2 :** `git checkout -b fix/transcription-propre`. Attendu : `git status` propre.

### Tâche 1 : le prompt ne contient que du vocabulaire

**Files :**
- Modify : `src-tauri/src/asr_context.rs` (`AsrSources`, `AsrScreen`, `AsrContextSettings`, `build`, `prompt`, tests)
- Modify : `src-tauri/src/actions.rs:1048-1063` (construction de `AsrScreen`)
- Modify : `src-tauri/src/managers/transcription.rs` (`ExpressionsCache`, `asr_expressions`, `frequent_expressions`, `asr_sources`, `cloud_context`, rapports)
- Modify : `src-tauri/src/recall.rs` (supprimer `frequent_expressions`, `FUNCTION_WORDS` et leur test), `learning.rs` et `lib.rs` (supprimer la commande `get_frequent_expressions`)
- Modify : `src-tauri/src/context/mod.rs` (`AsrReport` perd `expressions`, `app`, `before`)

**Interfaces, produites :**
- `AsrSources { custom_words, learned, screen_terms: Vec<String>, language: Option<String> }`
- `AsrScreen { terms: Vec<String> }`
- `AsrContextSettings { enabled, learned_words, screen, max_terms, capture_wait_ms }`
- `pub fn build(caps: AsrCaps, src: &AsrSources, cfg: &AsrContextSettings) -> AsrContext` (signature inchangée)

- [ ] **Étape 1 : tests qui échouent** (remplacent ceux qui citent expressions, app, before)

```rust
#[test]
fn the_prompt_holds_only_vocabulary() {
    let caps = AsrCaps { initial_prompt: true, ..Default::default() };
    let AsrPrompt::Initial(p) = build(caps, &src(), &AsrContextSettings::default()).prompt else { panic!() };
    assert_eq!(p, "Vocabulaire : Handy, log, Claude.");
    assert!(!p.contains('«'));
}
#[test]
fn no_term_means_no_prompt() {
    let caps = AsrCaps { initial_prompt: true, ..Default::default() };
    assert_eq!(build(caps, &AsrSources::default(), &AsrContextSettings::default()), AsrContext::default());
}
#[test]
fn a_term_too_long_alone_makes_no_prompt() {
    let s = AsrSources { custom_words: vec!["x".repeat(700)], ..Default::default() };
    let caps = AsrCaps { initial_prompt: true, ..Default::default() };
    assert_eq!(build(caps, &s, &AsrContextSettings::default()).prompt, AsrPrompt::None);
}
#[test]
fn old_settings_with_expressions_still_load() {
    let s: AsrContextSettings =
        serde_json::from_str(r#"{"expressions":true,"max_terms":12}"#).unwrap();
    assert_eq!(s.max_terms, 12);
    assert!(s.enabled && s.screen);
}
```
Garder `the_prompt_is_cut_at_a_whole_term` et `settings_out_of_range_are_refused`.

- [ ] **Étape 2 :** lancer les tests. Attendu : `the_prompt_holds_only_vocabulary` échoue.
- [ ] **Étape 3 :** implémenter. Ajouter ton nom (`user_name`, s'il n'est pas vide) en tête de `custom_words` dans `asr_sources` : l'essai B l'avait dans le vocabulaire, l'application ne l'y met pas aujourd'hui ; test `the_users_name_is_in_the_vocabulary`. Anglais : `Vocabulary: a, b.` Supprimer `cut` si plus rien ne l'appelle.
- [ ] **Étape 4 :** tests. Attendu : 0 échec.
- [ ] **Étape 5 :** commit `fix(asr): the prompt holds only vocabulary`.

### Tâche 2 : retirer une recopie en tête

**Files :** Modify `src-tauri/src/asr_context.rs` (fonction et tests) ; `managers/transcription.rs` (`echoes_prompt` supprimée, `run_batch` et `finish_cloud_text` l'utilisent ; son test est déplacé).

**Interfaces, produites :**
```rust
pub enum Echo { None, Lead, All }          // Debug, Clone, Copy, PartialEq, Eq
pub fn strip_echo(text: &str, prompt: Option<&str>) -> (String, Echo)
```
Règle : comparer les mots en minuscules, ponctuation ignorée.
1. Tout le texte, 3 mots ou plus, est une suite du prompt → `("", All)` (règle actuelle).
2. Sinon, plus longue suite de mots en tête qui est aussi une suite du prompt. Elle est retirée seulement si elle commence par le mot d'étiquette (`vocabulaire` / `vocabulary`) **et** compte 3 mots ou plus. Sans l'étiquette, rien n'est retiré : des mots vraiment dits sont indiscernables.
3. Le reste perd la ponctuation de tête (`, ; : . » «` et espaces) ; vide → `All`, sinon `Lead`.

- [ ] **Étape 1 : tests qui échouent**

```rust
#[test]
fn a_copied_lead_is_removed_and_the_speech_stays() {
    let p = Some("Vocabulaire : GitHub, Claude, Handy, Groq, Qwen, ADHD, Cloudflare.");
    assert_eq!(strip_echo("Vocabulaire : GitHub, Claude. Je suis prêt.", p), ("Je suis prêt.".into(), Echo::Lead));
    // Without the label it may be speech: kept whole.
    let said = "Claude, Handy, Groq, Qwen, ADHD, Cloudflare. On commence.";
    assert_eq!(strip_echo(said, p), (said.to_string(), Echo::None));
    assert_eq!(strip_echo("Vocabulaire : GitHub, Claude, Handy.", p), (String::new(), Echo::All));
}
#[test]
fn speech_that_opens_with_known_words_is_kept() {
    let p = Some("Vocabulaire : GitHub, Claude, Handy, Groq, Qwen.");
    for t in ["Claude, Handy et Groq sont prêts.", "Claude, Handy, Groq, Qwen : lequel ?", "Vocabulaire à revoir demain.", "log Claude"] {
        assert_eq!(strip_echo(t, p), (t.to_string(), Echo::None));
    }
    assert_eq!(strip_echo("Vocabulaire : GitHub, Claude.", None).1, Echo::None);
}
```
- [ ] **Étape 2 :** tests. Attendu : erreur de compilation (`strip_echo` absente).
- [ ] **Étape 3 :** implémenter. `run_batch` : `All` ou `echoes_vocabulary` → texte vide et `echo_dropped = true` (inchangé) ; `Lead` → texte nettoyé et `echo_dropped = true`.
- [ ] **Étape 4 :** tests. Attendu : 0 échec.
- [ ] **Étape 5 :** commit `fix(asr): a copied prompt is cut off, the speech after it stays`.

### Tâche 3 : recopie totale, second appel sans prompt

**Files :** Modify `src-tauri/src/cloud_asr.rs` (fonction et tests), `actions.rs:1186-1230`, `managers/transcription.rs` (`finish_cloud_text`).

**Interfaces :**
- Consomme : `strip_echo`, `Echo` (tâche 2).
- Produit :
```rust
// cloud_asr.rs ; call(prompt) fait un appel. Err seulement si le premier appel échoue.
pub async fn without_echo<F, Fut>(prompt: Option<&str>, call: F) -> Result<(String, Echo), String>
where F: FnMut(Option<String>) -> Fut, Fut: std::future::Future<Output = Result<String, String>>;
// transcription.rs
pub fn finish_cloud_text(&self, text: String, language: Option<String>, prompted: bool) -> Transcription
```
`without_echo` : `None`/`Lead` → texte nettoyé. `All` → `call(None)` une seule fois ; sa réponse est gardée telle quelle (sans prompt, elle ne peut pas en être la recopie : « Claude, Handy, Groq » dit seul revient ainsi), sauf en erreur → `("", All)`. Le `Echo` rendu est celui du premier appel. `prompted` vaut `echo != All` : un texte du second appel n'a pas reçu les mots personnalisés, leur correction locale s'applique.

- [ ] **Étape 1 : test qui échoue** `a_fully_copied_prompt_is_asked_again_without_it`, avec de faux appels (`std::future::ready`) :
  - premier « Vocabulaire : GitHub, Claude. », second « Bonjour. » → `Ok(("Bonjour.", All))`, prompts vus : `[Some, None]` ;
  - premier « Bonjour. » → `Ok(("Bonjour.", None))`, un seul appel ;
  - « Claude, Handy, Groq. » deux fois (prompt « Vocabulaire : Claude, Handy, Groq. ») → `Ok(("Claude, Handy, Groq.", All))`, deux appels ;
  - second appel vide → `Ok(("", All))` ;
  - second appel en erreur → `Ok(("", All))` ;
  - premier appel en erreur → `Err`, un seul appel.
  Et `escape_stops_the_second_call` : `complete_unless_cancelled` autour d'un second appel qui n'aboutit jamais, annulé → `None`.
- [ ] **Étape 2 :** tests. Attendu : échec de compilation.
- [ ] **Étape 3 :** implémenter. Dans `actions.rs`, tout `without_echo` reste dans `complete_unless_cancelled` (Échap coupe les deux appels). `report.echo_dropped = echo != Echo::None`, recopié dans le rapport local en cas de repli (`l.echo_dropped |= r.echo_dropped`, `actions.rs:1243`). Le bloc `async` de l'appel possède son `Option<String>`. Texte vide → modèle local, comme aujourd'hui. Journal : `Speech API '…' read its prompt back (Lead|All)`.
- [ ] **Étape 4 :** tests. Attendu : 0 échec.
- [ ] **Étape 5 :** commit `fix(asr): a fully copied prompt is transcribed again without it`.

### Tâche 4 : interface et traductions

**Files :** Modify `src/components/settings/post-processing/AsrContextSettings.tsx`, `ContextReports.tsx`, `vocabulary/VocabularySettings.tsx:109`, `src/bindings.ts`, les 26 `src/i18n/locales/*/translation.json`, `tests/fixtures/app-settings.json`.

- [ ] **Étape 1 :** retirer l'interrupteur et la liste « Expressions fréquentes », et les lignes expressions, application et texte avant du rapport ASR. `bindings.ts` : mêmes champs et commande retirés.
- [ ] **Étape 2 :** clés retirées des 26 langues : `learning.asr.expressions`, `learning.asr.expressionsList`, et `reports.asr.expressions`, `.app`, `.before`. Textes fr (en équivalent, autres langues : texte anglais) :
  - `learning.asr.description` : « Les mots que le modèle de reconnaissance reçoit avant d'écouter. Seuls les modèles qui acceptent du contexte s'en servent. »
  - `learning.asr.screen` : « Noms lus à l'écran »
  - `reports.asr.screenLate` : « L'écran est arrivé trop tard : envoyé sans les noms lus. »
  - `reports.asr.echo` : « Le modèle a recopié son vocabulaire : la copie a été retirée. »
- [ ] **Étape 3 :** `bun run build && bun run lint && bun run check:translations`. Attendu : aucune erreur.
- [ ] **Étape 4 :** commit `feat(settings): the speech model is told words, nothing else`.

### Tâche 5 : revue, build, installation, réparation

- [ ] **Étape 1 :** revue Codex `gpt-6.1-terra` (moyenne) sur `git diff main`, sans lire `Desktop\Handy Flow`. Corriger les vrais défauts, relancer les tests.
- [ ] **Étape 2 :** `bun run tauri build --no-bundle -- --profile fast`. Attendu : `target/fast/Handy Flow.exe`.
- [ ] **Étape 3 :** fermer Handy Flow, sauvegarder exe, DLL, ressources et `Data` (hors `huggingface`, `models`, `webview`) dans `backups/handy-flow-<date>`. Vérifier que `history.db` sauvegardée s'ouvre.
- [ ] **Étape 4 :** réparer `history.db`, en une transaction, depuis `ab.json` :

| N° | `transcription_text` et `post_processed_text` | `kept_text` |
|---|---|---|
| 301 | résultat `aucun` | NULL |
| 306 | résultat `vocab` | inchangé (NULL) |
| 313 | résultat `vocab` | NULL |
| 286, 291 | ligne supprimée, après contrôle que le fichier audio n'existe plus | — |

  Avant d'écrire : contrôler `file_name` (286 `handy-1795467236.wav`, 291 `handy-1795468102.wav`, 301 `handy-1795471862.wav`, 306 `handy-1795475791.wav`, 313 `handy-1795543289.wav`) ; un seul écart → ne rien modifier. Attendu : 3 lignes modifiées, 2 supprimées, les autres identiques à la sauvegarde.
- [ ] **Étape 5 :** remplacer exe, DLL et ressources, relancer. Attendu : processus présent ; `provider-keys.bin`, `settings_store.json`, `learned.bin` identiques avant relance (SHA-256).

### Tâche 6 : acceptation

- [ ] **Étape 1 (avec toi) :** demander une capture ou une lecture de Contexte → « Prompt envoyé » (le journal n'a que des longueurs) : le prompt d'une dictée commence par « Vocabulaire : » et finit par un point.
- [ ] **Étape 2 (toi) :** 20 dictées normales. Attendu : 0 début pollué, 0 phrase perdue. Je le contrôle dans l'historique.
- [ ] **Étape 3 :** mettre à jour `PLAN_LOTS.md` (K1 fait) et `plans/00-LISTE-DES-PLANS.md`.

**Retour arrière :** refermer Handy Flow, recopier exe, DLL, ressources et `history.db` depuis la sauvegarde de l'étape 5.3. Les dictées faites entre-temps seraient perdues avec `history.db` : ne la restaurer qu'en cas de besoin.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/01-v2.md`. Bugs du code actuel déplacés au plan 0 : 3.6, 4.4, 4.5, 4.6.
**À corriger dans ce plan**
- Tâche 3 : `prompted = (echo != All)` saute les mots personnalisés si le prompt est vide → `prompt.is_some() && echo != All` (A1).
- `strip_echo` règle 2 exige l'étiquette « Vocabulaire » : à vérifier avec le nouveau prompt (C1).
- Un seul budget de temps pour les deux appels ; délais du second appel non cumulés (A10, C2).
- Tester plus d'un modèle (Groq + local) et plusieurs essais par cas (C3).
- Historique : 26 lignes portent « « », le plan n'en répare que 5 (C4).
- Budget du prompt : Whisper garde les 223 derniers jetons, les premiers termes sautent ; budget en jetons, termes prioritaires en fin (A5, C5). Mots au-delà du prompt : ni prompt ni correction (A6).
**Améliorations retenues**
- Prises < 500 ms ignorées au lieu d'être complétées à 1,25 s de zéros (A2 ; Willow, Wispr).
- Retirer les balises non verbales Whisper (`[BLANK_AUDIO]`…) (passe 1, B7).
- Vocabulaire classé par fraîcheur et priorité (B4).
**Reporté au plan 8** : délai cloud proportionnel et course avec le local, connexion chaude, envoi allégé (B1–B3).
**Question pour toi** : espace insécable devant « ? ! ; : » en français (hors terminal et code) ?
