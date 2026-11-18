# Judge provider choice — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** to do (written 2026-11-18, not started). Branch `feat/handy-flow-brand`, base `cf2b0c7d` or later.

**Goal:** Let the user pick which LLM provider (and optionally which model) the correction judge uses, instead of the fixed rule "Groq if keyed, else the rewrite provider".

**Architecture:** Two new settings, read by `judge_target` in `src-tauri/src/judge.rs`. One backend command lists the providers the judge can call, so the dropdown and the judge share one usability rule. The UI adds a dropdown (and an optional model field) under the existing "Correction Judge" switch in `LearningSettings.tsx`.

**Tech Stack:** Rust (Tauri 2, serde, specta), React + TS, i18next (EN + FR only; other locales fall back to EN), Playwright with the Tauri mock (`tests/fixtures/tauri-mock.ts`).

**Spec:** the chat of 2026-11-18. Today `judge_target` uses Groq with `GROQ_SPARE_MODEL` when a Groq key exists, else the active rewrite provider and its model while rewriting is on, OpenAI-format providers only. The user wants to choose the judge's provider without removing their Groq key. Choosing the model separately is Task 3, which the user has not confirmed yet: ask before doing it.

## Global Constraints

- Settings default to **Automatic** (`""`): current behavior, unchanged for existing users.
- The judge only calls OpenAI-format providers (`PostProcessApiFormat::OpenAi`): it posts to `/chat/completions`. Anthropic stays excluded.
- A provider is **usable** when it exists, is OpenAI-format, has a non-empty key (the `custom` provider may be keyless), and resolves a non-empty model.
- An explicitly chosen provider does **not** require rewriting to be on: choosing it is the user's consent to send corrections there. Automatic keeps the rule "non-Groq only while rewriting is on".
- A chosen provider that is no longer usable (key removed, model cleared) falls back to Automatic silently, and the status line shows what is actually used.
- `judge_allowed` (learning + judge switch + context sending) still gates everything.
- Settings are written through one command per key (see `change_learning_judge_enabled_setting` in `src-tauri/src/shortcut/mod.rs` and the mapping in `src/stores/settingsStore.ts`). `src/bindings.ts` is edited by hand in this repo.
- Rust tests: `cd src-tauri && PATH="$PWD/target/debug/build:$PATH" cargo test --lib -- <filter>`; fixture: run once with `UPDATE_FIXTURES=1`. UI tests: `bun run test:playwright`.
- One fast build at the end (`bun run tauri build --no-bundle -- --profile fast`), never edit sources during it; then replace `Desktop\Handy Flow`, keeping `Data\` (back it up first).

## Review Focus

1. A user who picks OpenAI while keeping a Groq key: the judge must use OpenAI, not Groq → Task 1 `judge_uses_the_chosen_provider`.
2. A user who picks a provider then deletes its key: the judge falls back to Automatic and Settings says so → Task 1 `an_unusable_choice_falls_back_to_automatic` and the Task 2 status test.
3. A user with rewriting off who explicitly picks OpenAI: the judge runs (explicit consent) → Task 1 `an_explicit_choice_runs_with_rewriting_off`.
4. Anthropic never appears in the dropdown and is never called → Task 1 `judge_choices_exclude_anthropic`.
5. Existing users (setting absent from their store) keep today's behavior → serde default `""` + the fixture test.

---

### Task 1: Settings and `judge_target` honor the chosen provider

**Files:**
- Modify: `src-tauri/src/settings.rs` — `learning_judge_provider_id: String` (`#[serde(default)]`, after `learning_judge_enabled`), default `String::new()` in `get_default_settings`.
- Modify: `src-tauri/src/judge.rs` — `judge_target`, new `usable`, new command `get_judge_choices`.
- Modify: `src-tauri/src/shortcut/mod.rs` — `change_learning_judge_provider_setting(app, provider_id: String)`.
- Modify: `src-tauri/src/lib.rs` (register both commands), `tests/fixtures/app-settings.json` (regenerate).

**Interfaces:**
- Produces: `fn usable(s: &AppSettings, id: &str, model: Option<&str>) -> Option<(PostProcessProvider, String /*key*/, String /*model*/)>`: the provider if OpenAI-format, keyed (or `custom`), with `model` or else its rewrite model (`GROQ_SPARE_MODEL` for `groq`) non-empty.
- Produces: `#[derive(Serialize, specta::Type)] pub struct JudgeChoice { pub id: String, pub label: String }` and `#[tauri::command] get_judge_choices(app) -> Vec<JudgeChoice>`: every provider for which `usable(.., None)` is `Some`, in `post_process_providers` order.
- `judge_target(s)`: when `s.learning_judge_provider_id` is non-empty and `usable(..)` is `Some`, return it; otherwise today's Automatic logic, unchanged.

- [ ] **Step 1: Failing tests** in `judge.rs` tests (start from `get_default_settings()`, `post_process_api_keys.clear()`, `post_process_enabled = true` as the existing tests do):
  - `judge_uses_the_chosen_provider`: keys for `groq` and `openai`, model `gpt-4.1-mini` for openai, `learning_judge_provider_id = "openai"` → `judge_target(..).0.id == "openai"`, model `gpt-4.1-mini`.
  - `an_unusable_choice_falls_back_to_automatic`: same, then remove the openai key → id `groq`.
  - `an_explicit_choice_runs_with_rewriting_off`: only an openai key, `post_process_enabled = false`, choice `openai` → `Some`; choice `""` → `None`.
  - `judge_choices_exclude_anthropic`: keys and models for `anthropic` and `openai` → the ids from the pure helper behind `get_judge_choices` are `["openai"]` (extract `fn choices(s: &AppSettings) -> Vec<JudgeChoice>` and test that).
- [ ] **Step 2: Run** `cargo test --lib -- judge` → FAIL (missing field and functions).
- [ ] **Step 3: Implement.** Rewrite the Automatic branch of `judge_target` on top of `usable` so both paths share it. Keep the "rewriting off ⇒ no non-Groq call" rule in Automatic only.
- [ ] **Step 4: Run** `UPDATE_FIXTURES=1 cargo test --lib -- default_settings_fixture`, then `cargo test --lib` → all pass.
- [ ] **Step 5: Commit** `feat(learning): the user can choose the judge's provider`

---

### Task 2: Dropdown and status in Settings

**Files:**
- Modify: `src/bindings.ts`: `JudgeChoice` type, `getJudgeChoices(): Promise<JudgeChoice[]>`, `changeLearningJudgeProviderSetting(providerId: string): Promise<Result<null, string>>`, `learning_judge_provider_id?: string` in `AppSettings`.
- Modify: `src/stores/settingsStore.ts`: map `learning_judge_provider_id` to the new command.
- Modify: `src/components/settings/post-processing/LearningSettings.tsx`: under the judge switch (only while it is on), a `SettingContainer` (grouped, tooltip description) holding a `Dropdown` (`src/components/ui/Dropdown.tsx`: `options`, `selectedValue`, `onSelect`). Options: `{ value: "", label: t("…judge.provider.auto") }` then `getJudgeChoices()`. Re-fetch choices and `getJudgeProvider()` when the switch or the selected provider changes.
- Modify: `src/i18n/locales/en|fr/translation.json`, under `settings.postProcessing.learning.judge`:
  - `provider.label`: EN "Judge Provider" / FR "Fournisseur du juge"
  - `provider.description`: EN "Which AI reads your corrections. Automatic uses Groq when it has a key, else your rewrite provider." / FR "L'IA qui lit vos corrections. Automatique utilise Groq s'il a une clé, sinon votre fournisseur de réécriture."
  - `provider.auto`: EN "Automatic" / FR "Automatique"
- Test: `tests/learning.spec.ts`.

- [ ] **Step 1: Failing Playwright tests:**
  - `the judge provider can be chosen`: mock `get_judge_choices: [{id:"groq",label:"Groq"},{id:"openai",label:"OpenAI"}]`, open Post Process, open the "Judge Provider" dropdown, pick "OpenAI" → a call to `change_learning_judge_provider_setting` with `{ providerId: "openai" }`.
  - `the status shows what is really used`: mock `learning_judge_provider_id: "openai"` in settings and `get_judge_provider: "Groq"` → "Uses Groq" visible (the fallback is shown, not the stale choice).
- [ ] **Step 2: Run** `bun run test:playwright -- learning.spec.ts` → the 2 new tests FAIL.
- [ ] **Step 3: Implement.** (Scroll into view before clicking in tests; the page is tall.)
- [ ] **Step 4: Run** `bunx tsc --noEmit`, `bun run lint`, `bun run test:playwright` → all pass.
- [ ] **Step 5: Commit** `feat(learning): choose the judge's provider in Settings`

---

### Task 3 (only if the user confirms "avec modèle"): the judge's own model

**Files:** `settings.rs` (`learning_judge_model: String`, `#[serde(default)]`), `judge.rs`, `shortcut/mod.rs` (`change_learning_judge_model_setting`), `lib.rs`, `bindings.ts`, `settingsStore.ts`, `LearningSettings.tsx`, en/fr strings, fixture.

**Interfaces:** `judge_target` passes `Some(&s.learning_judge_model)` to `usable` when it is non-empty and a provider is chosen; Automatic ignores it.

- [ ] **Step 1: Failing test** `a_chosen_model_overrides_the_rewrite_model`: choice `openai`, rewrite model `gpt-4.1`, `learning_judge_model = "gpt-4.1-mini"` → model `gpt-4.1-mini`; empty → `gpt-4.1`. Playwright `the judge model can be set`: typing in the field and leaving it calls `change_learning_judge_model_setting`.
- [ ] **Step 2–4:** run → FAIL; implement an `Input` (variant `compact`, placeholder = the provider's rewrite model, `onBlur` saves; strings EN "Judge Model" / "Empty: the rewrite model", FR "Modèle du juge" / "Vide : le modèle de réécriture"); run all tests.
- [ ] **Step 5: Commit** `feat(learning): the judge can use its own model`

---

### Task 4: Build and install

- [ ] `cd src-tauri && PATH="$PWD/target/debug/build:$PATH" cargo test --lib` and `bun run test:playwright` → all green.
- [ ] Fast build, back up `Desktop\Handy Flow\Data\`, replace `Handy Flow.exe` and the DLLs from `src-tauri/transcribe-libs`, relaunch.
- [ ] A release only if the user asks (see `docs/release-portable.md`; sign with `node node_modules/@tauri-apps/cli/tauri.js signer sign …`).
