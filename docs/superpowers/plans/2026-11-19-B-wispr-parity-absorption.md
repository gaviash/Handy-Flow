# Plan B — Absorber les corrections comme Wispr Flow

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Handy Flow learns from the user's hand corrections the way Wispr Flow does: junction rules make case/punctuation corrections unnecessary, every candidate is decided by a strict LLM validator (active at first correction when accepted), observation ends with a final reading, and every decision is traceable.

**Architecture:** Two new pure modules — `junction.rs` (insertion rules, spec §5) and `validator.rs` (strict candidate validation, Wispr `extract_edited_words`) — plus a small `lexicon.rs`. `learning.rs` keeps alignment, candidates, memory and observation; its "misheard pending until seen twice" path is replaced by the validator. The existing judge (`judge.rs`) keeps only style rules and fillers.

**Tech Stack:** Rust (Tauri 2), `regex`, `serde_json`, `cargo test`; React for one stats line.

**Spec:** `docs/superpowers/specs/2026-11-19-wispr-parity-absorption.md` (requirements EX-1…EX-8, decisions in §6). Wispr evidence: `wispr_insights/IMPLEMENTATION_SPEC.md` §5, §8, §9 and rules R1–R11; `wispr_insights/fixtures/golden/cases.json`.

**Depends on:** Plan A (`2026-11-19-A-providers-per-feature.md`) — the validator resolves its provider with `features::feature_target(s, Feature::LearningValidator)`. Do Plan A first.

## Context an executor needs (no prior conversation required)

- **Repo / tests / fixtures / bindings:** same as Plan A's "Context" section (Rust test command with the DLL `PATH`, `UPDATE_FIXTURES=1`, `bun run tauri dev` regenerates `src/bindings.ts`, branch `feat/handy-flow-brand`).
- **How learning works today** (`src-tauri/src/learning.rs`):
  - After a paste, `actions.rs` calls `learning::observe(app, field, inserted, history_id, app_name, terminal)`. `observe` polls the field every 300 ms for 60 s (`WINDOW`), anchors the dictation (`locate` / `region`, or `locate_tail` / `region_near` in a terminal) and keeps `last`, the last valid region. On end it calls `learn(app, inserted, last)` (local candidates → memory) and spawns `judge::run` (LLM: misheard words + style rules), then stores `kept_text` in history.
  - `candidates(inserted, fin) -> Vec<Candidate>` (`learning.rs:~256`): alignment `align`, runs of 1–4 changed words bounded by unchanged ones, rewrite guard, then **local classification**: `Term` (capital not at sentence start, digit, `_`; distance ≤ 0.6) is active at once; `Misheard` (common word, distance ≤ 0.7, no meaning word, no grammar pair) is **pending until seen twice**; anything farther is rejected (`too_far`) before any LLM sees it.
  - `judge::run` (`judge.rs:234`) sends `JUDGE_PROMPT` (misheard / style_rule / filler / content_change / unclear) and `apply_verdicts` (`learning.rs:~800`) only promotes a pair that is already close (`close_pair`) — so `.env` ← « point oeuvre » can never be learned.
  - Memory: `Memory { words: Vec<Learned>, rules }`, DPAPI-encrypted `learned.bin`; `Learned { term, heard, count, last_seen, kind, active, used }`; `record_word`, `drop_form` (undo), `prune` (only pending words expire).
- **Measured on the user's PC** (spec §2): 28 short corrections in 155 dictations — 14 are only "first letter to lowercase" (junction, EX-1), 5 names learned, 3 technical words stuck pending (`specs`, `slop`, `ultrafast`), `.env` rejected twice, 4 meaning changes/rewrites correctly rejected.
- **Insertion today:** the text is pasted by `utils::paste` → `clipboard.rs`; `clipboard.rs:1018` appends a trailing space when `append_trailing_space` is on. No junction rule exists anywhere (`grep -rn junction src-tauri/src` finds nothing).
- **Capture available at paste time** (`actions.rs`, TranscribeAction): `field: Option<context::Field { before, after, selection, caret_known }>`, `terminal`, `sensitive`, `excluded`, `unchecked`; style/category via `context::llm_context` (`Content`, `Style`, `Tone`), known names in `capture.names` + learned terms.

## Global Constraints

- Privacy (spec R1): nothing from a password, excluded or unchecked field is sent, observed or learned — reuse the existing `sensitive || excluded || unchecked` flags.
- No model call in any automated test: LLM replies are fixed JSON strings in the tests.
- Junction applies to **every** target, terminal included (spec §6 Q3), and does nothing when `caret_known` is false.
- A validated candidate is active at its **first** correction (spec §6 Q2, Wispr E146); "pending until seen twice" survives only when no validator is reachable.
- Logs never contain dictated text: lengths, reasons and verdict kinds only.
- Lexicon licence — **accepted by the user on 2026-11-19**: use `hermitdave/FrequencyWords` 2018 `fr_50k.txt` / `en_50k.txt` (data CC BY-SA 4.0); add the attribution (source, author, licence, link) to `THIRD_PARTY_NOTICES.md` (create if missing) and state in its header that the two lexicon files stay under CC BY-SA 4.0.

## Review Focus

1. Junction in a terminal prompt where `before` is the shell prompt (`PS C:\> `) — must not lowercase a command the user dictates as a sentence start after a prompt symbol (`>` is not a letter: rule 2 doesn't fire). Test in Task 1.
2. A validator reply that lists a term not among the candidates, or twice, makes the whole reply void (E76) — no partial learning. Test in Task 3.
3. A correction ended by Enter in a chat (field emptied) learns from the final reading taken at the end event, not a reading up to 300 ms old (EX-3). Test in Task 4.
4. The validator unreachable (no provider, timeout 15 s, network error) never loses the candidate: local fallback decides (EX-2.5). Test in Task 3.
5. Undo of a learned word must stop the same pair from being learned again on the next identical correction (EX-6). Test in Task 6.

---

### Task 1: Junction rules (EX-1, spec §5)

**Files:** Create `src-tauri/src/junction.rs` (+ `mod junction;` in `lib.rs`); modify `actions.rs` (apply just before the paste thread, on `final_text`), `clipboard.rs:1018` (no extra trailing space when the junction already decided spacing).

**Interfaces — Produces:**
```rust
pub struct JoinContext<'a> { pub field: Option<&'a crate::context::Field>, pub style: crate::context::Tone,
                             pub content: Option<crate::context::Content>, pub known_names: &'a [String] }
pub struct Joined { pub text: String, pub spacing_decided: bool }
pub fn join(text: &str, ctx: &JoinContext) -> Joined
```
Rules in order (spec §5, copy the exact conditions): 0 guard (`field` None or `!caret_known` → unchanged, `spacing_decided=false`); 1 trim; 2 messaging final period by style (very_casual always, casual ≤ 10 sentences, default + work/personal chat ≤ 2, formal never; never with a selection or when `lb+selected+la` already has `.!?`); 4 selection (lowercase if the selection starts lowercase unless proper noun, drop final `.!?` if the selection doesn't end with one, carry its leading/trailing space); 5 continuation (`lb` ends with a letter/digit after skipping closing quotes/brackets → lowercase first letter unless the first word is in `known_names` or is an acronym / has an internal capital), following text (`la` starts lowercase or with `, . : ; ! ? % >` → drop final `.!?`), leading space (when `lb` non-empty and ends with a letter/digit or `. , ; : ! ?` or closing quote; never after `( [ " '`, a newline or CJK), trailing space (when `la` non-empty and starts with a letter/digit, or when `la` is empty and the text ends a sentence — J05/J06/J07/J10); 7 Notion-like placeholder (`before` ≤ 2 words or ends with `...`, nothing after, no selection → skip rules 4–5). `lb` = last line of `before`, `la` = first line of `after`.

- [ ] **Step 1: Failing tests** — one test per golden case J01–J10 with the `field`, `llm_text` and `expected` values from `cases.json` (J08 uses `expected`, Handy's choice), plus `terminal_prompt_symbol_keeps_the_capital` (`before = "PS C:\\> "`, text `"Liste les fichiers."` → `"Liste les fichiers."` with no leading space added after the space) and `known_name_keeps_its_capital` (`before = "merci "`, text `"Jev a validé."`, known `["Jev"]` → `"Jev a validé."`), `unknown_caret_changes_nothing`.
- [ ] **Step 2: Run** `cargo test --lib junction::` → FAIL.
- [ ] **Step 3: Implement** `join`; wire it in `actions.rs` for TranscribeAction only, after the learned-word fallback and the user's hook, never when `sensitive || excluded || unchecked`; pass `spacing_decided` so `clipboard.rs` skips `append_trailing_space` when true.
- [ ] **Step 4: Run** the whole Rust suite → PASS.
- [ ] **Step 5: Commit** `feat(paste): Wispr's junction rules match case, punctuation and spaces to the text around the cursor`

### Task 2: Frequency lexicon (EX-4)

**Files:** Create `src-tauri/resources/lexicon/fr.txt`, `en.txt` (one lowercase word per line, top 50 000, generated by `scripts/build-lexicon.ts` from the FrequencyWords lists — the script is committed, the source URLs in its header), `src-tauri/src/lexicon.rs` (`include_str!`, lazily parsed into a `HashSet`), `THIRD_PARTY_NOTICES.md`.

**Interfaces — Produces:** `pub fn is_common(word: &str) -> bool` (lowercased, apostrophe-split: `l'ultra-face` checks `ultra-face` and its parts; true if in fr **or** en list).

- [ ] **Step 1: Failing tests** — `quelle`, `cloud`, `projet`, `the` common; `kelmorine`, `zorblax`, `corvanti`, `.env` not common.
- [ ] **Step 2–4:** RED, implement, GREEN. Check the binary size grows by < 1 MB.
- [ ] **Step 5: Commit** `feat(learning): a fr/en frequency lexicon to tell rare words from common ones`

### Task 3: Strict LLM validator (EX-2)

**Files:** Create `src-tauri/src/validator.rs`; modify `learning.rs` (`candidates` no longer rejects on distance; `observe` calls the validator instead of `learn` + the misheard part of the judge), `judge.rs` (`JUDGE_PROMPT` keeps only `style_rule`, `filler`, `content_change`, `unclear`; drop `misheard`/`technical` handling in `apply_verdicts`).

**Interfaces:**
- Consumes: `features::feature_target(s, Feature::LearningValidator)` (Plan A, returns the reasoning level too), `llm_client::send_chat_completion_with_schema(..., reasoning)` (Plan A Task 2b — pass the validator's level, never a hard-coded `Off`; timeout 15 s when `Off`, 45 s otherwise), `lexicon::is_common` (Task 2), `learning::{candidates, record_word, Memory}`.
- Produces:
```rust
pub enum Local { Candidate(Candidate), Guarded(&'static str) }      // in learning.rs
pub fn local_candidates(inserted: &str, fin: &str) -> Vec<Local>       // replaces the classification in candidates()
pub const VALIDATOR_PROMPT: &str = "...";                              // validator.rs
pub enum Accepted { Name, Technical }                                  // "common" is refused, like Wispr (E222)
pub fn parse_reply(reply: &str, sent: &[Candidate]) -> Option<Vec<(Candidate, Accepted)>>  // None = reply void
pub fn fallback(c: &Candidate) -> Decision
pub enum Decision { Active, Pending, Rejected(&'static str) }
pub async fn validate(app: &AppHandle, inserted: &str, kept: &str, cands: Vec<Candidate>) -> Vec<(Candidate, Decision)>
```
- Guards that stay local and final (`Guarded`): meaning words (`MEANING_WORDS`, `FR_COMMON_NAMES`), grammar pairs / verb endings, name truncation, rewrite, case-only, possessive/contraction, already active with the same heard form. Everything else is a `Candidate`, however far it sounds (`.env` ← « point oeuvre »).
- Prompt content (write it in English, keep it short): input JSON `{"pasted": …, "kept": …, "candidates": [{"heard","term"}…]}`; accept only proper names, rare or technical terms, and words the speech recognizer distorts (`specs` ← spikes, `.env` ← point oeuvre, PostgreSQL ← postgéré); refuse grammar, agreement, synonyms, style, meaning changes; output `{"accepted":[{"heard","term","kind":"name"|"technical"}]}`, nothing else.
- `parse_reply`: any item whose `(heard, term)` is not exactly one sent candidate, or a duplicate → `None` (whole reply void, E76).
- Decision: accepted → `Active` at once; refused → nothing; reply void, timeout (15 s), no provider → `fallback`: `Active` if the term has a capital not at sentence start, a digit or `_`, or is not `is_common`, **and** distance ≤ 0.6; otherwise `Pending` (today's second-sighting rule).

- [ ] **Step 1: Failing tests** (fixed replies, no network): golden L01–L08 through `local_candidates` + `parse_reply(<fixed JSON>)` + decision → `expected_handy`; `point_oeuvre_env_is_active_when_validated_technical`; `an_invented_candidate_voids_the_reply`; `a_duplicate_voids_the_reply`; `unreachable_validator_falls_back_locally` (`specs` ← spikes → `Pending`, `Kelmorine` ← corvanti-like rare → `Active`); `judge_prompt_no_longer_asks_for_misheard_words`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** and rewire `observe` (validator runs in the background like the judge does today; `record_word` with `active` from the decision; announce news as today). **Step 4: Run** whole suite → PASS (update `learning_e2e_tests.rs` expectations that assumed "pending until twice" for validated words).
- [ ] **Step 5: Commit** `feat(learning): a strict LLM validator decides each correction, learned at first sight like Wispr`

### Task 4: Reliable end of observation (EX-3)

**Files:** `learning.rs` (`observe`).

**Interfaces — Produces:** `pub enum End { Window, NextDictation, Emptied, AroundChanged, NotFound, Unreadable }` and `pub fn learnable(end: End, final_read_ok: bool, intact: bool) -> bool`.

Rules: on every end event read the field **once more immediately** (no 300 ms wait) and use that reading if its anchors hold; if that read fails, keep the last reading and mark `final_uncertain` → `learnable` is false (spec R7). `AroundChanged` learns only when the last region was bounded by both anchors or the pasted text is still present unchanged (Wispr `anchor_mismatch_transcript_intact`). `NotFound` / `Unreadable` never learn.

- [ ] **Step 1: Failing tests** on `learnable` and on a refactored pure `fn finish(last: Option<String>, final_read: Option<String>, anchors, end) -> Option<String>` (the region to learn from): Enter-send case (O02/L08: the final read before emptying holds the fix → learned); final read failed → `None`; O01, O03, O06, O08 from `cases.json` as unit cases of `finish`.
- [ ] **Step 2–4:** RED, implement, GREEN.
- [ ] **Step 5: Commit** `fix(learning): end an observation with a final reading and never learn from an uncertain one`

### Task 5: Re-evaluate pending words (EX-5)

**Files:** `learning.rs`, `validator.rs`, `lib.rs` (call once after startup when settings allow the validator).

**Interfaces — Produces:** `pub async fn revalidate_pending(app: &AppHandle)` — one validator call with every pending word as a candidate (`pasted`/`kept` empty, the prompt states they come from past corrections); accepted → active + news; refused → removed; void/unreachable → unchanged, retried at next start. A `revalidated_at: i64` in `Memory` stops it once nothing is pending.

- [ ] **Step 1: Failing test** with a fixed reply: pending `specs`, `slop`, `devra correspondre` → reply accepts `specs`, `slop` → those two active, the third removed.
- [ ] **Step 2–5:** RED, implement, GREEN, commit `feat(learning): words still pending are re-validated once`

### Task 6: Source of each entry and a permanent undo (EX-6)

**Files:** `learning.rs` (`Learned.source: Source` `#[serde(default)]`, `Memory.refused: Vec<(String,String)>`), `recall.rs` (sort `Manual` first — custom words already come first in `enriched_context`; keep that), the undo command (find it: `grep -rn "drop_form" src-tauri/src`).

**Interfaces — Produces:** `pub enum Source { UserEdits, AppContext }` (default `UserEdits`; `Manual` stays `settings.custom_words`); undo pushes `(heard, term)` lowercased into `refused`; `local_candidates` guards a refused pair (`Guarded("undone")`).

- [ ] **Step 1: Failing test** `an_undone_pair_is_never_learned_again`; `a_chain_keeps_both_entries` (L06) still green.
- [ ] **Step 2–5:** RED, implement, GREEN, commit `feat(learning): an undone correction is never learned again`

### Task 7: Traceability and counters (EX-7)

**Files:** `learning.rs` / `validator.rs` (one `log::debug!` per candidate: `Learning: candidate <len> chars → <Guarded reason | validator kind | fallback decision>`), `Memory.stats: LearningStats` (rolling 30 days: observed, learned, pending, rejected by reason), command `get_learning_stats() -> LearningStats`, one status line in `src/components/settings/post-processing/LearningSettings.tsx` (« 30 derniers jours : {observed} corrections observées · {learned} mots appris · {pending} en attente · {rejected} refusés »), i18n `en`/`fr` + copies.

- [ ] **Step 1: Failing tests** on the stats update (pure fn) and a Playwright check that the line renders from a mocked `get_learning_stats`.
- [ ] **Step 2–5:** RED, implement, GREEN (regenerate bindings), commit `feat(learning): every learning decision is logged and counted`

### Task 8: Replay of the user's real corrections

**Files:** `src-tauri/src/learning_e2e_tests.rs`.

Add `replay_of_the_measured_corrections` with the 28 correction shapes from spec §2, **people's names replaced by fictional ones** (no real name in the repo; keep technical terms such as `.env`, `specs`, `slop`, `ultrafast`, `SOM`). Each item: `before` (cursor context), `pasted`, `kept`, the fixed validator reply, the expected outcome. Expected totals: the 14 lowercase-only corrections are produced by `junction::join` (so `pasted` after junction == `kept`, nothing to learn); 5/5 names active; `.env`, `specs`, `slop`, `ultrafast` active; 0 learned from the 4 meaning changes/rewrites.

- [ ] **Step 1–3:** write the test, run (it may expose gaps in Tasks 1–3: fix there, not in the test), GREEN.
- [ ] **Step 4: Commit** `test(learning): replay of measured real corrections`

### Task 9: Whole-branch check and real use

- [ ] Rust suite, Playwright, lint, translations green; `cargo clippy` clean on touched files.
- [ ] Manual (`bun run tauri dev`, real validator provider from Plan A): in Notepad and in Windows Terminal (Claude Code), type `regarde ` then dictate « Le log du serveur. » → pasted `le log du serveur` (junction). Dictate « ouvre le point oeuvre », correct to `.env`, dictate again → `.env` written (validator, first correction). Log shows one `Learning: candidate …` line per candidate.
- [ ] Fresh review of the whole branch with Codex (command in Plan A Task 5).
