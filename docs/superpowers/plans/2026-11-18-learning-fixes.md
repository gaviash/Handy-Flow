# Learning system fixes — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the learning layer from damaging dictations (grammar learned as "misheard", forced substitutions, self-renewing bad words), let the judge work without a Groq key, and give the user exact control (clear, undo, export, judge switch).

**Architecture:** All learning logic stays in `src-tauri/src/learning.rs`, the recall in `recall.rs`, the judge in `judge.rs`, the rewrite prompt in `adaptive.rs`. The front changes only `LearningSettings.tsx`, `App.tsx` and `bindings.ts` (hand-edited, as in this repo). No new dependency.

**Tech Stack:** Rust (Tauri 2, serde, specta), React + TS, i18next (EN + FR only; other locales fall back to EN), Playwright with the Tauri mock.

**Spec:** the review agreed in chat on 2026-11-18 (Claude + Codex critique). Accepted: A1 (solutions 1+2: the 4-letter floor AND the grammar list), A2 (1+2), A3 (2), A4 (2), A5, C1, C2, C3, C4. Refused: B1. Not decided: B2, B3, B4.

## Global Constraints

- Branch `feat/handy-flow-brand`, in `C:\Users\gavri\Desktop\handy_remastered\Handy`. Never push to `upstream`.
- The hybrid stays: a name/technical `Term` is active after **1** correction; a common `Misheard` word after **2**; judge `technical` = immediate.
- A learned word unused for **30 days** is forgotten (unchanged).
- Rust tests: `cd src-tauri && PATH="$PWD/target/debug/build:$PATH" cargo test --lib -- <filter>` (the PATH holds the ggml DLLs).
- UI tests: `bun run test:playwright`. New UI strings in `src/i18n/locales/en` and `fr` only.
- One build at the end (`bun run tauri build --no-bundle -- --profile fast`), never edit sources during it; then replace `Desktop\Handy Flow` keeping `Data\`.

## Review Focus

1. A memory saved by 0.2.5 that already holds `est (heard as "et")`: after the update it must be gone without the user doing anything → Task 1 test `purge_drops_grammar_pairs_already_learned`.
2. A name learned from one correction (`Qwen` heard `Quen`) must still be sent and still be **forced**, while a common misheard word is only suggested → Task 2 test `misheard_words_are_hints_terms_are_forced`.
3. A learned word the user keeps saying but the ASR keeps mishearing (`Quen`) must stay alive → Task 3 test `saying_a_heard_form_keeps_the_word`.
4. A user with only an OpenAI key and no Groq key must get a judge → Task 4 test `judge_falls_back_to_the_active_provider`.
5. Undo on a notification that announced a **new form** of an old word must keep the old word → Task 5 test `undo_a_new_form_keeps_the_word`.

---

### Task 1: Grammar changes are never learned as misheard (A1)

**Files:**
- Modify: `src-tauri/src/learning.rs` — `candidates` (Misheard branch, ~line 333), `close_pair` (~line 517), `prune` (~line 189)

**Interfaces:**
- Produces: `fn is_grammar_change(heard: &str, term: &str) -> bool` (lowercase inputs), used by `candidates`, `close_pair` and `prune`.

- [ ] **Step 1: Write the failing tests** in `learning.rs` `mod tests`:

```rust
#[test]
fn grammar_changes_are_never_misheard() {
    for (a, b) in [
        ("Il et parti hier soir.", "Il est parti hier soir."),
        ("Il a pris ces affaires.", "Il a pris ses affaires."),
        ("Je vais le voir demain.", "Je vais la voir demain."),
        ("On se voit ce soir.", "On se voit se soir."),
        ("Je l'ai envoyer hier.", "Je l'ai envoyé hier."),
        ("I think their right.", "I think they're right."),
        ("It was better then that.", "It was better than that."),
    ] {
        assert!(super::candidates(a, b).is_empty(), "{a} -> {b}");
    }
    // A misheard common word of 4+ letters still counts.
    assert_eq!(super::candidates("ouvre le fichier conflig", "ouvre le fichier config").len(), 1);
    // Accepted trade-off: words under 4 letters are no longer learned ("gag" → "log").
    assert!(super::candidates("regarde le gag du serveur", "regarde le log du serveur").is_empty());
}

#[test]
fn purge_drops_grammar_pairs_already_learned() {
    let mut m = Memory::default();
    let now = 1_000_000;
    m.words.push(Learned { term: "est".into(), heard: vec!["et".into()], count: 2, last_seen: now, kind: WordKind::Misheard, active: true });
    m.words.push(Learned { term: "Qwen".into(), heard: vec!["Quen".into()], count: 1, last_seen: now, kind: WordKind::Term, active: true });
    prune(&mut m, now);
    assert_eq!(m.words.iter().map(|w| w.term.as_str()).collect::<Vec<_>>(), ["Qwen"]);
}
```

- [ ] **Step 2: Run** `cargo test --lib -- grammar_changes purge_drops` → both FAIL.

- [ ] **Step 3: Implement `is_grammar_change`** — true when any of:
  - either side has fewer than **4** letters (`letters(..).chars().count() < 4`);
  - both share a stem after stripping one French verb ending from `["er", "é", "ée", "és", "ées", "ez", "ai", "ais", "ait", "aient", "ant"]` (longest first), e.g. `envoyer`/`envoyé` → `envoy`;
  - the pair (either order) is in `const GRAMMAR_PAIRS: &[(&str, &str)]` = `("their","they're"), ("there","their"), ("there","they're"), ("then","than"), ("your","you're"), ("were","where"), ("quand","quant"), ("peut","peux"), ("voie","voix"), ("près","prêt"), ("leur","leurs"), ("quel","quelle")`.

  In `candidates`, in the `else if d <= MISHEARD_DISTANCE` branch, return `Err("grammar")` first when `is_grammar_change(&hk, &tk)`. In `close_pair`, add `&& !is_grammar_change(&h, &t)`. In `prune`, for `WordKind::Misheard` words drop heard forms where `is_grammar_change(form, term)`, then drop words left with no form.

- [ ] **Step 4: Run** `cargo test --lib -- learning` → all pass. `misheard_common_word_is_learned_after_two_corrections` and `meaning_changes_are_never_learned` use `gag`/`log`: move them to `conflig`/`config` (same assertions).

- [ ] **Step 5: Commit** `fix(learning): never learn grammar fixes as misheard words; purge those already learned`

---

### Task 2: Only relevant words reach the rewrite; only names are forced (A2)

**Files:**
- Modify: `src-tauri/src/recall.rs` — `relevant_terms`
- Modify: `src-tauri/src/adaptive.rs` — prompt rule at ~line 60 and the instruction prompt at ~line 74, plus their tests (~line 770)

**Interfaces:**
- `relevant_terms(words: &[Learned], transcription: &str, max: usize) -> Vec<String>` keeps its signature. Output lines: `Term` → `Qwen (heard as "Quen")`; `Misheard` → `dashboard (maybe heard as "crash board")`.

- [ ] **Step 1: Failing test** in `recall.rs` tests:

```rust
#[test]
fn misheard_words_are_hints_terms_are_forced() {
    let w = |term: &str, heard: &str, kind| Learned { term: term.into(), heard: vec![heard.into()], count: 3, last_seen: 0, kind, active: true };
    let words = [w("Qwen", "Quen", WordKind::Term), w("dashboard", "crash board", WordKind::Misheard), w("Kubernetes", "cube", WordKind::Term)];
    let got = relevant_terms(&words, "on teste quen sur le crash board", 40);
    assert_eq!(got, ["Qwen (heard as \"Quen\")", "dashboard (maybe heard as \"crash board\")"]);
    // Nothing heard: nothing sent (no more "most used" filler).
    assert!(relevant_terms(&words, "bonjour à tous", 40).is_empty());
}
```

- [ ] **Step 2: Run** `cargo test --lib -- misheard_words_are_hints` → FAIL.
- [ ] **Step 3: Implement** — filter `active` to `heard_now(l)` only, format by `l.kind`. In the rewrite prompt replace the sentence `A dictated word that matches a form listed as "heard as" MUST become that term.` with: `A dictated word that matches a form listed as "heard as" MUST become that term. A form listed as "maybe heard as" becomes that term only when the sentence makes no sense otherwise.` Mirror it in the instruction prompt (line ~74). Update the `adaptive.rs` test that asserts `terms: Claude (heard as` only if its fixture changes.
- [ ] **Step 4: Run** `cargo test --lib -- recall adaptive` → pass.
- [ ] **Step 5: Commit** `fix(learning): send only words heard in this dictation; force names, suggest common words`

---

### Task 3: The 30 days restart on what was said, not on what was pasted (A3)

**Files:**
- Modify: `src-tauri/src/learning.rs` — `mark_used`, `touch_used`
- Modify: `src-tauri/src/actions.rs` — keep the raw transcription (~line 1460, before `hm.save_entry` moves it) and pass it to `touch_used` (~line 1564) instead of `dictated`

**Interfaces:**
- `pub fn touch_used(app: &AppHandle, spoken: &str)` — `spoken` is the raw transcription, before any rewrite.
- `pub(crate) fn mark_used(m: &mut Memory, spoken: &str, now: i64) -> bool` — renews an active word when `spoken` contains, as a whole word (any case), its term **or one of its heard forms**.

- [ ] **Step 1: Failing test** next to `dictating_a_learned_word_restarts_its_30_days`:

```rust
#[test]
fn saying_a_heard_form_keeps_the_word() {
    let mut m = Memory::default();
    m.words.push(Learned { term: "Qwen".into(), heard: vec!["Quen".into()], count: 1, last_seen: 0, kind: WordKind::Term, active: true });
    assert!(mark_used(&mut m, "on compare quen et gemma", 500));
    assert_eq!(m.words[0].last_seen, 500);
    assert!(!mark_used(&mut m, "on compare queneau", 900)); // not a whole word
}
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (extract the whole-word check into a closure over `term` and each `heard`). In `actions.rs`, `let spoken = transcription.clone();` before `save_entry`, carry it into the paste thread, call `touch_used(&ah_clone, &spoken)`.
- [ ] **Step 4: Run** `cargo test --lib -- learning` and `cargo check`. **Step 5: Commit** `fix(learning): renew a word when it is said, not when the rewrite writes it`

---

### Task 4: The judge works without a Groq key; it can be switched off (A4 + C4 backend)

**Files:**
- Modify: `src-tauri/src/judge.rs` — `run`, new `judge_target`
- Modify: `src-tauri/src/settings.rs` — new `learning_judge_enabled: bool` (`#[serde(default = "default_true")]`, after `learned_sound_enabled`), default in `get_default_settings`; update `tests/fixtures/app-settings.json` with `UPDATE_FIXTURES=1`
- Modify: `src-tauri/src/learning.rs` — the judge spawn in `observe` (~line 1163) also requires `settings.learning_judge_enabled`
- Add command `get_judge_provider() -> Option<String>` (provider label) in `judge.rs`, register it in `lib.rs`, add to `bindings.ts`

**Interfaces:**
- `fn judge_target(s: &AppSettings) -> Option<(PostProcessProvider, String /*key*/, String /*model*/)>`: Groq with a non-empty key → `(groq, key, GROQ_SPARE_MODEL)`; else `s.active_post_process_provider()` with a non-empty key (or `custom`, which may run keyless) and `s.post_process_models[id]` non-empty; else `None`.

- [ ] **Step 1: Failing test** in `judge.rs`:

```rust
#[test]
fn judge_falls_back_to_the_active_provider() {
    let mut s = crate::settings::get_default_settings();
    s.post_process_provider_id = "openai".into();
    s.post_process_api_keys.insert("openai".into(), "sk-x".into());
    s.post_process_models.insert("openai".into(), "gpt-4.1-mini".into());
    let (p, _, model) = judge_target(&s).unwrap();
    assert_eq!((p.id.as_str(), model.as_str()), ("openai", "gpt-4.1-mini"));
    s.post_process_api_keys.insert("groq".into(), "gsk".into());
    assert_eq!(judge_target(&s).unwrap().0.id, "groq");
    s.post_process_api_keys.clear();
    assert!(judge_target(&s).is_none());
}
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**; `run` uses `judge_target` (skip reason `"no provider"`), keeps the 15 s timeout and the schema call (it already retries without schema). `get_judge_provider` returns `judge_target(..).map(|(p, ..)| p.label)` when `learning_judge_enabled`, else `None`.
- [ ] **Step 4: Run** `cargo test --lib -- judge default_settings_fixture` (with `UPDATE_FIXTURES=1` once). **Step 5: Commit** `feat(learning): judge uses Groq or the active provider, and can be turned off`

---

### Task 5: Clear, undo and export do exactly what they say (C1, C2, C3)

**Files:**
- Modify: `src-tauri/src/learning.rs` — `clear_learned_words` (words only), new `clear_style_rules`, new `remove_heard_form(term, heard)`, `export_learned_words` (writes `{ "words": [...], "rules": [...] }`), the two `learned-words` emits
- Modify: `src-tauri/src/lib.rs` (register commands), `src/bindings.ts`
- Modify: `src/App.tsx` (~line 150), `src/components/settings/post-processing/LearningSettings.tsx`
- Test: `tests/joy.spec.ts` (or a new `tests/learning.spec.ts`)

**Interfaces:**
- Event `learned-words` payload becomes `Vec<LearnedNews>`:
  `#[derive(Serialize, Clone, specta::Type)] #[serde(tag = "kind", rename_all = "snake_case")] pub enum LearnedNews { Word { term: String }, Form { term: String, heard: String }, Rule { rule: String } }`.
  `record_word` returns `Option<LearnedNews>` instead of `bool` (`Word` when a word is created active or becomes active, `Form` when an active word gains a form); `apply_verdicts` returns `Vec<LearnedNews>`.
- Commands: `clear_style_rules(app)`, `remove_heard_form(app, term: String, heard: String)` (drops the word if it has no form left).

- [ ] **Step 1: Failing tests.** Rust: `undo_a_new_form_keeps_the_word` — record `Qwen/Quen` (Term), then `Qwen/Quinn` returns `Some(LearnedNews::Form{..})`; `remove_heard_form` logic (pure helper `fn drop_form(m, term, heard)`) leaves `Qwen` with `["Quen"]`. `clearing_words_keeps_rules`. Playwright: `undo removes only what was announced` — emit `learned-words` with `[{kind:"form",term:"Qwen",heard:"Quinn"},{kind:"rule",rule:"Pas de point final."}]`, click Undo, expect calls `remove_heard_form` and `delete_style_rule`, and no `delete_learned_word`. `export failure shows a message` — mock `export_learned_words` to `{status:"error"}`… (the binding returns `Result`, assert a toast).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** Notification title lists `term` / `rule` as today. "Clear All" under Learned Words keeps its label and now clears words only; add the same button under Style Rules (`settings.postProcessing.learning.rules.clear` / `confirmClear`: EN "Clear All" / "Clear all style rules?", FR "Tout effacer" / "Effacer toutes les règles de style ?"), disabled when there is no rule. Export shows `notify({ title: t("settings.postProcessing.learning.words.exportError") })` on error (EN "Couldn't export the learned words.", FR "Impossible d'exporter les mots appris.").
- [ ] **Step 4: Run** `cargo test --lib -- learning` and `bun run test:playwright`. **Step 5: Commit** `fix(learning): clear, undo and export act on exactly what they name`

---

### Task 6: Judge switch and status in Settings (C4 front), build, install

**Files:**
- Modify: `src/components/settings/post-processing/LearningSettings.tsx` — a `ToggleSwitch` for `learning_judge_enabled` after the sound toggle; under it a muted line from `commands.getJudgeProvider()`.
- Strings (EN/FR): `learning.judge.label` "Correction Judge" / "Juge des corrections"; `learning.judge.description` "Sends your correction (before and after) to an AI to learn style rules and technical words." / "Envoie votre correction (avant et après) à une IA pour apprendre des règles de style et des mots techniques."; `learning.judge.status` "Uses {{provider}}" / "Utilise {{provider}}"; `learning.judge.none` "Inactive: no API key for Groq or your rewrite provider." / "Inactif : aucune clé d'API pour Groq ni pour votre fournisseur de réécriture."

- [ ] **Step 1: Failing Playwright test** `the judge shows which provider it uses`: mock `get_judge_provider: "OpenAI"` → text "Uses OpenAI" visible; mock `null` → the "Inactive" line.
- [ ] **Step 2–4:** implement, run `bun run test:playwright`, `bunx tsc --noEmit`, `bun run lint`.
- [ ] **Step 5: Commit** `feat(learning): show and switch the correction judge`
- [ ] **Step 6:** see Task 7 first; the build is its last step.

---

### Task 7: The judge's verdicts are checked against the texts (A5)

**Files:**
- Modify: `src-tauri/src/learning.rs` — `apply_verdicts` gets `inserted: &str, kept: &str`; `record_verdicts` passes them; `judge.rs::run` passes its `inserted`/`kept`

**Interfaces:**
- `pub(crate) fn apply_verdicts(m, verdicts, local, inserted: &str, kept: &str, now) -> Vec<LearnedNews>` (the unused `_app_name` parameter goes).

- [ ] **Step 1: Failing test** `judge_pairs_must_be_in_the_texts`: a `misheard` verdict `dashbord → dashboard`, technical, with `inserted = "le tableau est prêt"` → nothing learned; with `inserted = "le dashbord est prêt"`, `kept = "le dashboard est prêt"` → `Qwen`-style news `Word{term:"dashboard"}`. And `a_style_rule_needs_two_sightings`: a new `style_rule` with `consistent_with_history: true` stays inactive; the same rule again (`same_as`) activates it.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** — a whole-word, case-insensitive presence check (`heard` in `unify(inserted)`, `term` in `unify(kept)`; reuse the boundary closure of Task 3); a new rule is stored `active: false` whatever `consistent_with_history` says (the field stays in the schema, unused). Update `verdicts_respect_the_guards` if it asserted the immediate activation.
- [ ] **Step 4: Run** `cargo test --lib` (all). **Step 5: Commit** `fix(learning): trust a judge verdict only when the texts show it`
- [ ] **Step 6:** one fast build, replace `Desktop\Handy Flow` (back up `Data\` first), relaunch., replace `Desktop\Handy Flow` (back up `Data\` first), relaunch.
