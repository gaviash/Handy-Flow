# Wispr-parity learning and vocabulary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The learned vocabulary reaches the rewrite and the ASR the way Wispr Flow's dictionary does, corrections are observed in every editable target (terminals included), and learning is no longer blocked by our own filters.

**Architecture:** Pure functions in `recall.rs`, `learning.rs` and `asr_context.rs` change their selection rules (each with unit tests from `wispr_insights/fixtures/golden/cases.json`); `actions.rs`, `managers/transcription.rs` and `managers/history.rs` change their wiring. No new module.

**Tech Stack:** Rust (Tauri 2), `cargo test` in `src-tauri`.

**Spec:** `wispr_insights/IMPLEMENTATION_SPEC.md` (§0 R10, §4, §6 rules 5–6, §8.7, §9), `wispr_insights/EVIDENCE_REGISTER.md` (E50, E78, E79, E140, E146, E226), `wispr_insights/RESEARCH_ADDENDUM.md` (E213–E226).

**Order:** Task 2 before Task 1 (Task 1 sorts by `Learned.used`), then 3 → 9.

## Global Constraints

- Rust sends no display strings; no new UI in this plan.
- Never send or store anything from a password field, a secure-input target or an excluded app/domain (spec R1); every new path reuses `capture.sensitive` / `capture.excluded`.
- History schema: new nullable columns go in `ADDED_COLUMNS` (`managers/history.rs:40`), never in `MIGRATIONS`.
- Learned memory stays DPAPI-encrypted; new `Learned` fields are `#[serde(default)]` so existing `learned.bin` files load.
- Run `cargo fmt` and `cargo clippy` before each commit; conventional commit prefixes.

## Review Focus

1. A learned word whose heard form is a real word ("Cloud" → "Claude") must not replace that word in an unrelated sentence: the prompts make "heard as" contextual (Task 1 step 4a), non-heard terms carry no mapping (Task 1 test `unheard_terms_have_no_mapping`), and the local fallback never touches plain-word forms (Task 7 test `a_heard_form_that_is_a_real_word_is_never_replaced_blindly`). Whether the LLM follows the context is checked by hand in Task 9 ("cloud" infra sentence vs "Claude" sentence).
2. A user who keeps typing after a dictation (region grows) must still have their single-word fix learned when it is followed by punctuation (Task 3 test `typing_after_the_dictation_is_not_a_rewrite`).
3. A full rewrite (golden A05) and a case-only edit (A04) must still learn nothing after the rewrite-ratio change (Task 3).
4. A Windows Terminal target whose buffer scrolls between readings must end the observation (anchor mismatch), never learn from unrelated output (Task 4 manual check).
5. The local fallback replacement must never fire when the LLM rewrite succeeded, and never on a pending or `Misheard` word (Task 7 tests).

---

### Task 1: Send the whole learned dictionary to the rewrite (Wispr `dictionary_context`)

Wispr sends its dictionary on every dictation (E50), ordered by source then recency bucket (≤ 24 h, ≤ 14 d, older) then frequency (E79), and the formatter applies it contextually (E226). Today `relevant_terms` sends only words heard within 0.34 of a dictated word (`recall.rs:26`).

**Files:**
- Modify: `src-tauri/src/recall.rs:23-54` (`relevant_terms`) and its tests
- Modify: `src-tauri/src/learning.rs:767-773` (`terms_for`: max 40 → 80, fix the doc comment)
- Modify: `src-tauri/src/adaptive.rs:57-61` (house prompt wording)
- Modify: `docs/prompts/GPT_better.txt` (the user's prompt)

**Interfaces:**
- Consumes: `Learned { used: u32 }` from Task 2 (sort key; until Task 2 lands, sort by `count`).
- Produces: `relevant_terms(words: &[Learned], transcription: &str, now: i64, max: usize) -> Vec<String>` — new `now` parameter for the recency buckets.

- [ ] **Step 1: Write the failing tests in `recall.rs`**

```rust
#[test]
fn every_active_term_is_sent_heard_ones_first() {
    // Claude heard now; Postgres not heard; a pending word never.
    let out = relevant_terms(&words, "ouvre cloud code", NOW, 80);
    assert_eq!(out[0], r#"Claude (heard as "Cloud")"#);
    assert!(out.contains(&"Postgres".to_string()));
    assert!(!out.iter().any(|t| t.starts_with("specs")));
}
#[test]
fn unheard_terms_have_no_mapping() {
    let out = relevant_terms(&words, "bonjour", NOW, 80);
    assert!(out.iter().all(|t| !t.contains("heard as")));
}
#[test]
fn unheard_terms_follow_wispr_recency_buckets() {
    // seen 2 h ago (used 1) before seen 3 days ago (used 9) before seen 20 days ago.
    assert_eq!(relevant_terms(&w, "", NOW, 80), vec!["Recent", "Week", "Old"]);
}
#[test]
fn an_unheard_misheard_word_is_not_sent() { /* "config (maybe heard as …)" only when heard */ }
```

- [ ] **Step 2: Run `cargo test recall::` in `src-tauri` — expect FAIL (signature, missing terms).**

- [ ] **Step 3: Implement.** Heard-now words keep today's format and order (count, then last_seen). Then append every other active `WordKind::Term` as the bare `term`, sorted by bucket (`now - last_seen` ≤ 86 400 → 0, ≤ 14 × 86 400 → 1, else 2), then `used` desc, then `count` desc. `Misheard` words only when heard. Truncate to `max`. Update the caller in `terms_for` (`chrono::Utc::now().timestamp()`, max 80).

- [ ] **Step 4a: Contextual, never blind (E226: "Jordan Peterson" stays Jordan).** In `adaptive.rs:60-61` replace "A dictated word that matches a form listed as \"heard as\" MUST become that term." by: a dictated word matching a "heard as" form becomes that term **only when the sentence is about that term**; otherwise keep what was said. Add two examples in the prompt: `Claude (heard as "Cloud")` → "demande à cloud de relire le code" → "demande à Claude de relire le code", but "on héberge ça dans le cloud" stays "cloud"; `Jordane (heard as "Jordan")` → "Jordan Peterson" stays. Same rule in `docs/prompts/GPT_better.txt`, replacing "STRICT ONE-WAY mapping … both … must output" by the contextual wording (one-way stays: never turn the term back into the heard form). Update the prompt assertions in `adaptive.rs` tests (~555) to look for the new wording.

- [ ] **Step 4: Prompts.** In `adaptive.rs:57-61` replace "products, files and code identifiers seen around the cursor or saved by the user: when words of the dictation sound like one of them, write that term exactly … otherwise ignore them" so it also says: a term without "heard as" is a spelling reference, used only when a dictated word sounds like it and the sentence fits. In `docs/prompts/GPT_better.txt` swap the example to the user's real entry: `Jev (heard as "Jeff")` means "Jeff" and "Jev" both output "Jev"; add the "maybe heard as" and bare-term lines with the same meaning as the house prompt.

- [ ] **Step 5: Run `cargo test` — PASS (including `adaptive.rs` prompt tests at lines ~555 and ~787).**

- [ ] **Step 6: Commit** `fix(learning): send the whole learned dictionary to the rewrite, mapped only when heard`

---

### Task 2: Active words never expire; count usage like Wispr `frequencyUsed`

E78: each final text containing a dictionary word (`\bword\b`, case-insensitive) increments its frequency. Wispr has no expiry. Today `prune` (`learning.rs:189`) deletes active words unused for 30 days and `count` only counts corrections.

**Files:**
- Modify: `src-tauri/src/learning.rs:123-135` (`Learned`), `:189-210` (`prune`), `:588-612` (`mark_used`, `touch_used`)
- Modify: `src-tauri/src/actions.rs:1565` (call site of `touch_used`)

**Interfaces:**
- Produces: `Learned.used: u32` (`#[serde(default)]`); `mark_used(m: &mut Memory, spoken: &str, kept: &str, now: i64) -> bool`; `touch_used(app: &AppHandle, spoken: &str, kept: &str)`.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn an_active_word_survives_a_year_without_use() {
    prune(&mut m, NOW + 365 * DAY); assert!(m.words.iter().any(|w| w.term == "Jev"));
}
#[test]
fn a_pending_word_still_expires_after_30_days() { /* active:false, last_seen NOW-31d → gone */ }
#[test]
fn the_term_in_the_pasted_text_counts_one_use() {
    mark_used(&mut m, "merci jeff", "Merci Jev.", NOW);
    assert_eq!(find(&m, "Jev").used, 1);
}
#[test]
fn a_heard_form_alone_refreshes_but_does_not_count() { /* spoken "jeff", kept "Merci Jeff." → last_seen = NOW, used 0 */ }
```

- [ ] **Step 2: `cargo test learning::` — FAIL.**
- [ ] **Step 3: Implement.** `prune`: retain `w.active || w.last_seen >= expired` (rules unchanged; keep `WORDS_MAX`, sorting by `used` then `count` then `last_seen`). `mark_used`: today's `last_seen` refresh from `spoken`, plus `used += 1` when `contains_word(kept, term)`; return `true` when either changed, so `touch_used` saves. Add `used: 0` to every existing `Learned { … }` literal (`record_word`, tests). `actions.rs`: `final_text` is moved into `paste` (~1550) — clone it before as `kept` and pass it to `touch_used`.
- [ ] **Step 4: `cargo test` — PASS.**
- [ ] **Step 5: Commit** `fix(learning): learned words no longer expire and count their uses`

---

### Task 3: Stop rejecting real corrections as rewrites

`candidates` (`learning.rs:252-261`) rejects when changed tags (including insertions) exceed half the words: a one-word dictation corrected is rejected, and text typed after the dictation (the region grows, `learning.rs:1286`) makes any fix a "rewrite". Spec §8.7: a rewrite is > 30 changed words; golden A05 is already rejected by the run rules (`too_long` / `unbounded`).

**Files:**
- Modify: `src-tauri/src/learning.rs:252-261` and tests

- [ ] **Step 1: Failing tests (golden cases + regressions)**

```rust
#[test]
fn golden_alignment_cases() {
    assert_eq!(candidates("Nous installons Corvanti ce soir.", "Nous installons Quorvantis ce soir."), vec![("Quorvantis".into(), "Corvanti".into())]); // A01
    assert_eq!(candidates("Le dossier est suivi par Aurélie Kenen.", "Le dossier est suivi par Aurélie Quennehen."), vec![("Quennehen".into(), "Kenen".into())]); // A03
    assert!(candidates("Je pars à Paris demain.", "Je pars à PARIS demain.").is_empty()); // A04
    assert!(candidates(A05_PASTED, A05_EDITED).is_empty()); // A05
}
#[test]
fn a_one_word_dictation_can_be_corrected() {
    // Internal capital: a term even at the start of the field.
    assert_eq!(candidates("Postgre", "PostgreSQL"), vec![("PostgreSQL".into(), "Postgre".into())]);
}
#[test]
fn golden_a02_a two_word_name_is_learned_as_one_phrase() {
    // Handy keeps phrase mode on (spec §9.2.0), unlike this Wispr account.
    assert_eq!(candidates("Je travaille avec Kazimierz Bignowski sur le projet.", "Je travaille avec Kasimir Zbignowski sur le projet."),
               vec![("Kasimir Zbignowski".into(), "Kazimierz Bignowski".into())]);
}
#[test]
fn typing_after_the_dictation_is_not_a_rewrite() {
    assert_eq!(candidates("Merci Jeff.", "Merci Jev. Et voici une longue suite tapée ensuite au clavier"), vec![("Jev".into(), "Jeff".into())]);
}
```
(`learning.rs` tests already compare `Vec<Candidate>` with `(term, heard)` tuples through their private helper; reuse it here. Fix the test name typo `golden_a02_a two_word…` → `golden_a02_a_two_word…`.)

```rust
#[test]
fn a_name_may_extend_what_was_heard_but_not_truncate_it() {
    // L01 (E218): Wispr learns Jordane←Jordan. E227/HANDOFF §4: a truncated name is refused.
    assert_eq!(candidates("Merci Jordan, à demain.", "Merci Jordane, à demain."), vec![("Jordane".into(), "Jordan".into())]);
    assert!(candidates("Merci Jordane, à demain.", "Merci Jordan, à demain.").is_empty());
    // A common word keeps the plural/conjugation guard (L05).
    assert!(candidates("Le projet avance bien.", "Le projets avance bien.").is_empty());
}
```

- [ ] **Step 2: `cargo test learning::` — FAIL on the regressions.**
- [ ] **Step 3: Implement.** Count only `Tag::Sub` and `Tag::Del` (words of the dictation that changed); reject as `rewrite` when that count > 30, or when it is more than half of `a.len()` and `a.len() >= 4`. In the `affix` check (`learning.rs:342-344`), for a `looks_like_term` candidate reject only a truncation (`tk` strictly shorter than `hk` and a prefix or suffix of it); keep both directions for common words.
- [ ] **Step 4: `cargo test` — PASS, existing `learns_the_users_vs_code_corrections` etc. unchanged.**
- [ ] **Step 5: Commit** `fix(learning): a short dictation, text typed after it, or a lengthened name is a real correction`

---

### Task 4: Observe corrections in terminals too

Wispr observes every verified paste (E70–E72). `actions.rs:1518` excludes `terminal` targets, so nothing typed in Windows Terminal (Claude Code, Codex) is ever learned.

Removing the exclusion is not enough: `read_watched` (`context/windows.rs:521-545`) reads the **first** 15 000 chars (`GetText(WATCH_CHARS)`), which in a terminal is the start of the scrollback, so a dictation at the bottom is never found; and `region` (`learning.rs:1168`) needs the whole field to start with `before` and end with `after`, which breaks as soon as the terminal prints a line.

**Files:**
- Modify: `src-tauri/src/actions.rs:1516-1519` (filter; pass `terminal` to `observe`)
- Modify: `src-tauri/src/context/windows.rs:521-545` (`read_watched`)
- Modify: `src-tauri/src/learning.rs:1168-1235` (`observe`, new `region_near`) and tests

**Interfaces:**
- Produces (see Step 3 for the final `region_near` signature with `lines`): `read_watched(tail: bool) -> Option<String>` (tail = the last `WATCH_CHARS`), in `context/windows.rs` **and** both relays in `context/mod.rs:~929` (Windows and the non-Windows stub); `observe(…, terminal: bool)`; `pub fn region_near<'a>(text: &'a str, before: &str, after: &str) -> Option<&'a str>`; `pub fn locate_tail(text: &str, inserted: &str) -> Option<(String, String)>` (anchors = up to 200 chars before / after the **only** occurrence of `inserted` in the tail read, `None` when it appears 0 or ≥ 2 times; the caret from the capture is not used, since its offsets don't map to a tail read); `fn can_observe(caret_known: bool, terminal: bool) -> bool` (= `caret_known || terminal`), replacing the `caret_known` guard at the top of `observe`.

- [ ] **Step 1: Failing tests in `learning.rs`**

```rust
#[test]
fn a_terminal_region_is_found_near_its_anchors_while_output_scrolls() {
    let before = "PS C:\\> claude\n> "; let after = "\n  ? for shortcuts";
    let field = format!("old output\nmore output\n{before}merci Jev{after}");
    assert_eq!(region_near(&field, before, after), Some("merci Jev"));
}
#[test]
fn an_ambiguous_terminal_anchor_ends_the_observation() {
    // the same before-anchor twice → None (spec R6 anchor_ambiguous)
}
#[test]
fn an_empty_after_anchor_takes_as_many_lines_as_were_pasted() {
    assert_eq!(region_near("out\n> merci Jev", "> ", "", 1), Some("merci Jev"));
    // Output printed after the dictation is never part of the region.
    assert_eq!(region_near("out\n> merci Jev\nCompiling handy…", "> ", "", 1), Some("merci Jev"));
}
#[test]
fn an_empty_before_anchor_takes_the_start_of_the_read() {
    assert_eq!(region_near("merci Jev\n  ? for shortcuts", "", "\n  ?"), Some("merci Jev"));
}
#[test]
fn the_terminal_dictation_must_appear_once_in_the_tail() {
    let (b, a) = locate_tail("out\n> merci Jeff", "merci Jeff").unwrap();
    assert_eq!((b.as_str(), a.as_str()), ("out\n> ", ""));
    // Twice (scrollback, or output printed after the paste): abandon, spec R6.
    assert_eq!(locate_tail("> merci Jeff\nok\n> merci Jeff", "merci Jeff"), None);
}
#[test]
fn two_empty_anchors_never_make_a_region() {
    assert_eq!(region_near("anything printed", "", ""), None);
}
#[test]
fn a_terminal_is_observed_without_a_known_caret() {
    assert!(can_observe(false, true));   // caret unknown, terminal
    assert!(!can_observe(false, false)); // caret unknown, other field: unchanged
}
```

- [ ] **Step 2: `cargo test learning::` — FAIL.**
- [ ] **Step 3: Implement.** `read_watched(true)`: clone `DocumentRange`, `MoveEndpointByRange(Start, range, End)` then `MoveEndpointByUnit(Start, TextUnit_Character, -WATCH_CHARS)`, `GetText(-1)`; `false` keeps today's read. In `observe`, when `terminal`: read with `tail = true`, anchor with `locate_tail` (not `locate`), and use `region_near`: the region starts after the single occurrence of a non-empty `before` (`None` if it occurs more than once) or at 0 when `before` is empty; it ends at the first occurrence of a non-empty `after` after that start, or, when `after` is empty, after `lines` lines (`lines` = line count of the pasted text), so output printed below is never included; `None` when both anchors are empty or the region would be empty. Signature: `region_near<'a>(text: &'a str, before: &str, after: &str, lines: usize) -> Option<&'a str>`; update the other `region_near` tests to pass `1`. Non-terminal targets keep `region` unchanged. In `actions.rs` remove `&& !terminal` (keep `!sensitive && !excluded`) and pass `terminal`.
- [ ] **Step 4: `cargo build` and `cargo test` — PASS.**
- [ ] **Step 3: Manual check in Windows Terminal (`bun run tauri dev`, `--debug`):** dictate "merci Jeff" into a Claude Code prompt, change "Jeff" to "Jev", wait 60 s. Expected log: `Learning: observation ended (the 60 s window closed); edited: true, learned: 1` (or the judge adds it). Then run a command that prints output while a dictation is observed: expected `the text around the dictation changed` and `learned: 0` from that output. Last, correct a word, then make the terminal print output before the 60 s end: expected the correction made **before** the output is learned (the last valid reading, as Wispr's `anchor_mismatch_transcript_intact`), nothing from the output. Record both results in the commit message.
- [ ] **Step 4: Commit** `fix(learning): observe corrections in terminals`

---

### Task 5: Screen names and terms reach the ASR (spec R10, E140)

Wispr's main lever is names from the window biasing the ASR (E140, E142). Today `asr_sources` (`managers/transcription.rs:1295`) passes only the app and the text before the cursor; `capture.names` / `capture.terms` go to the LLM only.

**Files:**
- Modify: `src-tauri/src/asr_context.rs` (`AsrSources`, `build`, `AsrContextSettings::default`)
- Modify: `src-tauri/src/managers/transcription.rs:287, 1289-1320` (`asr_screen`, `set_asr_screen`, `asr_sources`)
- Modify: `src-tauri/src/actions.rs:1056-1068, 1181, 1218, 1229` (the `screen` channel)
- Modify: `tests/fixtures/app-settings.json` if it pins `asr_context.screen`

**Interfaces:**
- Produces: `pub struct AsrScreen { pub app: String, pub before: String, pub terms: Vec<String> }` in `asr_context.rs`; `AsrSources.screen_terms: Vec<String>`; `set_asr_screen(&self, screen: Option<AsrScreen>)`.

- [ ] **Step 1: Failing tests in `asr_context.rs`**

```rust
#[test]
fn screen_terms_come_after_custom_words_and_before_learned() {
    // custom ["Handy"], screen ["Kasimir Zbignowski"], learned ["log", "Claude"]
    assert_eq!(build(caps_vocab(), &src, &cfg).vocabulary, vec!["Handy", "Kasimir Zbignowski", "log", "Claude"]);
}
#[test]
fn screen_terms_are_left_out_when_screen_is_off() { /* cfg.screen = false → no "Kasimir Zbignowski" */ }
#[test]
fn the_prompt_is_cut_at_a_whole_term() { /* 600-char budget never ends mid-term or on a comma */ }
```

- [ ] **Step 2: `cargo test asr_context::` — FAIL.**
- [ ] **Step 3: Implement.** In `build`, terms = custom words, then `screen_terms` when `cfg.screen`, then learned (dedupe case-insensitive, `max_terms`). `AsrContextSettings::default().screen = true` (Wispr uses context by default; capture wait stays 150 ms). In `actions.rs` the screen tuple becomes `AsrScreen { app, before, terms }` with `terms` = `capture.names` then `capture.terms`, sent only when the capture is not private: `!capture.sensitive && !capture.excluded && !unchecked` (same rule as `private_tx`, `actions.rs:1056`). Keep the received `AsrScreen` in a local before the speech-API call and re-set it from that local on the cloud → local fallbacks (`actions.rs:1218`, `:1229`), instead of rebuilding it from `report.app`/`report.before`, so the terms survive. Test `screen_terms_survive_the_cloud_fallback` at the `asr_sources` level. `cut` keeps the `PROMPT_MAX_CHARS` budget and trims back to the last `, ` separator inside the vocabulary part.
- [ ] **Step 4: `cargo test` — PASS.**
- [ ] **Step 5: Commit** `feat(context): screen names and terms bias the ASR`

---

### Task 6: Record what the rewrite was given (Wispr `additionalContext`)

History stores only the system prompt; the user can't see which terms were sent. Store the term list only (no screen text: spec "minimisation").

**Files:**
- Modify: `src-tauri/src/managers/history.rs:40-46` (`ADDED_COLUMNS` + `"context_terms"`), entry read/write
- Modify: `src-tauri/src/actions.rs` where `ProcessedTranscription` is saved (~1464)

**Interfaces:**
- Produces: `HistoryManager::set_context_terms(&self, id: i64, terms: &[String]) -> Result<()>` storing a JSON array; `HistoryEntry.context_terms: Option<String>`.

- [ ] **Step 1: Failing test in `history.rs`:** `set_context_terms` on an in-memory DB, then read back `["Jev (heard as \"Jeff\")","Claude"]`; a DB created before the column gains it on open.
- [ ] **Step 2: `cargo test history::` — FAIL.**
- [ ] **Step 3: Implement** (mirror `set_kept_text_on`). In `actions.rs`, after `save_entry`, call it with `llm_context.terms` + `llm_context.names` when post-processing ran and the context exists. Regenerate `bindings.ts` (run the app in debug) since `HistoryEntry` is exported.
- [ ] **Step 4: `cargo test` — PASS.**
- [ ] **Step 5: Commit** `feat(history): keep the terms each rewrite was given`

---

### Task 7: Dictionary replacements when the rewrite does not run (spec §6 rule 5, §9.4)

Wispr's formatter always runs server-side. Ours may not (plain transcribe binding, timeout, failure): then no learned word is applied at all with Parakeet. Spec §6 rule 5: on LLM failure paste the raw text **with the dictionary replacements**; §9.4: replace `observed_source` by `phrase` between word boundaries.

**Files:**
- Modify: `src-tauri/src/learning.rs` (new pure fn + wrapper)
- Modify: `src-tauri/src/actions.rs:770-830` (`process_transcription_output`: when `!post_process` or `post_process_failed`)

**Interfaces:**
- Produces: `pub(crate) fn replace_heard(text: &str, words: &[Learned]) -> String`; `pub fn apply_learned(app: &AppHandle, text: &str) -> String`.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn a_confirmed_term_replaces_a_heard_form_that_is_no_real_word() {
    assert_eq!(replace_heard("Ouvre RandyFlow.", &[handy_flow(count 2)]), "Ouvre Handy Flow.");
}
#[test]
fn a_heard_form_that_is_a_real_word_is_never_replaced_blindly() {
    // Without the LLM there is no context check: "cloud", "Jeff", "sum" stay.
    assert_eq!(replace_heard("On héberge ça dans le cloud.", &[claude(count 5)]), "On héberge ça dans le cloud.");
    assert_eq!(replace_heard("Merci Jeff.", &[jev(count 5)]), "Merci Jeff.");
}
#[test]
fn pending_misheard_or_once_seen_words_never_replace() { /* misheard "slop"←"slope"; Term count 1 → unchanged */ }
#[test]
fn only_whole_words_are_replaced() { /* "Jeffrey" unchanged */ }
```

- [ ] **Step 2: `cargo test learning::` — FAIL.**
- [ ] **Step 3: Implement.** Active `WordKind::Term` with `count + used >= 2`; only heard forms that can't be an ordinary word — an uppercase letter after the first char, a digit or `_` (`RandyFlow`, `BrokerName`, `jf2`) — replaced as a whole word (`contains_word` boundaries). Heard forms that are plain words (`Cloud`, `Jeff`, `sum`) are left to the LLM, which checks the context. `// shortcut: no lexicon (spec R11), so "plain word" = no inner capital, digit or _; a lexicon would let rare plain forms through.` Trigger on the **effective** result, not the flags: in `process_transcription_output`, apply when `post_processed_text` is `None` at the end (not requested, `Failed`, `Skipped`, `Rejected`, timeout) — never to a successful rewrite. Only for a dictation (TranscribeAction), never for a transform/instruction, and never when the capture is `sensitive`, `excluded` or unchecked (`password_checked == false` on Windows). Test `fallback_runs_after_a_rejected_rewrite` and `no_fallback_for_a_password_field`.
- [ ] **Step 4: `cargo test` — PASS.**
- [ ] **Step 5: Commit** `feat(learning): apply confirmed learned words when the rewrite doesn't run`

---

### Task 8: Automated end-to-end tests of the learning loop (golden cases, no audio)

The ASR and the LLM can't run in `cargo test`, but every deterministic step between them can: correction → candidate → memory → next dictation's rewrite context, ASR vocabulary and fallback replacement. These tests chain the real functions, so a break anywhere in the loop fails them.

**Files:**
- Create: `src-tauri/src/learning_e2e_tests.rs` (`#[cfg(test)] mod learning_e2e_tests;` in `lib.rs`)

**Interfaces:**
- Consumes: `candidates`, `record_word`, `apply_verdicts` (Task 2/3), `relevant_terms` (Task 1), `asr_context::build` + `AsrSources.screen_terms` (Task 5), `replace_heard` (Task 7), `adaptive::context_block`, `context::extract_terms`.

- [ ] **Step 1: Write the tests**

```rust
#[test]
fn e13_a_corrected_word_is_reused_everywhere_in_the_next_dictation() {
    // Dictation 1 pasted "Nous installons Corvanti ce soir.", user fixes it.
    let mut m = Memory::default();
    for c in candidates("Nous installons Corvanti ce soir.", "Nous installons Quorvantis ce soir.") { record_word(&mut m, &c, NOW); }
    // Dictation 2, ASR heard it again wrong:
    let terms = relevant_terms(&m.words, "nous installons corvanti demain", NOW, 80);
    assert_eq!(terms, vec![r#"Quorvantis (heard as "Corvanti")"#]);
    assert!(context_block(&ctx_with_terms(&terms)).contains("terms: Quorvantis (heard as"));
    assert!(build(caps_vocab(), &src_with_learned(&plain_terms_of(&m)), &cfg()).vocabulary.contains(&"Quorvantis".into()));
    assert_eq!(replace_heard("Nous installons Corvanti demain.", &m.words), "Nous installons Corvanti demain."); // seen once: no blind replace
}
#[test]
fn e13_bis_a_word_confirmed_twice_is_replaced_without_rewrite() { /* two corrections → replace_heard fixes "Corvanti" */ }
#[test]
fn golden_learning_filter_l01_to_l08() {
    // one row per case: (original sentence, corrected sentence, expected learned term or None)
    // L01 Jordan→Jordane Some; L02 Plurixan→Plorixane Some; L03 postgéré→PostgreSQL Some;
    // L04 mardi→jeudi None; L05 projet→projets None; L07 quelle→Kelmorine None (Handy choice);
    // L08 ombrevecs→Ombrevex Some. L06: existing Zorblax←Orblax + Zorblax→Zorblux → both entries kept.
}
#[test]
fn e02_e05_e15_screen_names_reach_both_asr_and_rewrite() {
    let terms = extract_terms(&["Cc : Kasimir Zbignowski".into(), "Zorblax, Quintavex, Merindor".into()], "", false, &[]);
    assert!(terms.contains(&"Kasimir Zbignowski".into()) && terms.contains(&"Zorblax".into()));
    let v = build(caps_vocab(), &AsrSources { screen_terms: terms.clone(), ..Default::default() }, &cfg()).vocabulary;
    assert!(v.contains(&"Kasimir Zbignowski".into()));
    // Rewrite side: the same names in the <context> block.
    assert!(context_block(&LlmContext { terms: terms.clone(), ..base_ctx() }).contains("Kasimir Zbignowski"));
    // E15: code identifiers from a .py file
    assert!(extract_terms(&["def compute_invoice_total(x):".into()], "billing_service.py - proj - Visual Studio Code", true, &[]).contains(&"compute_invoice_total".into()));
}
#[test]
fn a_full_rewrite_and_a_case_change_teach_nothing() { /* A04, A05 through the same chain: memory stays empty */ }
#[test]
fn the_learned_word_survives_a_year_and_its_uses_are_counted() { /* Task 2 chain: mark_used then prune(+365 d) */ }
```

Helpers to write at the top of the file, all from real types: `NOW: i64`, `caps_vocab() -> AsrCaps { vocabulary: true, ..Default::default() }`, `cfg() -> AsrContextSettings::default()`, `base_ctx() -> LlmContext` (category `Other`, everything else empty), `ctx_with_terms(t) -> LlmContext { terms: t.to_vec(), ..base_ctx() }`, `plain_terms_of(m) -> Vec<String>` (active terms, as `plain_terms` does), `src_with_learned(v) -> AsrSources { learned: v, ..Default::default() }`. Compare `Candidate` by `.term` / `.heard` fields (the tuple helper is private to `learning.rs`). L06 asserts Wispr's behaviour (E223): both `Zorblax←Orblax` and `Zorblux←Zorblax` stay; flagging the chain to the user is left out (needs UI, see end).

- [ ] **Step 2: `cargo test learning_e2e` — PASS** (every earlier task is in; any failure points at the broken link).
- [ ] **Step 3: Commit** `test(learning): end-to-end golden cases of the learning loop`

---

### Task 9: End-to-end check on a real PC (what cargo can't run)

Covers the golden cases that need audio, UIA or an LLM: E01–E15 output quality, O01–O08 observation end reasons, Windows Terminal.

- [ ] Build (`bun run tauri build`) and run on the target PC with `--debug`, house prompt and the user's prompt.
- [ ] Golden L01–L08 by hand in Notepad, one correction per dictation, closing each by the next dictation (E213–E214): L01/L02/L03/L08 learned, L04/L05 rejected, L07 rejected (Handy choice), L06 both entries kept.
- [ ] Context check (E226) with `Claude←Cloud` learned: "demande à cloud de relire le code" → "Claude"; "on héberge ça dans le cloud" → "cloud"; "J'ai lu Jordan Peterson" stays Jordan with `Jordane←Jordan` learned.
- [ ] E13/E14: the word is available from the dictation after the closing one, not in the closing one.
- [ ] Check the `context_terms` column of the last entries: heard words first, then the other active terms, at most 80 (only when context sending is on).
- [ ] Windows Terminal session (Task 4 manual check).
- [ ] O01–O08 in Notepad: read each `Learning: observation ended (…)` line against `expected_end` (60 s, emptied, next dictation, typing outside the region, paste not found).
- [ ] E02, E05, E15 with a speech API that takes a prompt (Groq Whisper): Settings → Context shows the screen names in the ASR prompt; the transcript spells them.

---

## Not in this plan (checked against the spec, deliberately left out)

- Capture at dictation start and on focus change (spec §3.1): Wispr needs it for streaming; Handy transcribes after release and already captures before the ASR. No learning impact.
- Keystroke categories to tell "sent" from "deleted" when a field empties (§8.5): today an emptied field keeps the last reading, as Wispr does after Enter (E224). Small false-learning risk after select-all-delete; revisit if seen.
- Learning from screen OCR (`app_context`, E77): Wispr does it only from OCR, off by default and untested.
- Style detection, Polish, Command Mode: outside learning.
- Flagging a correction chain (`Orblax→Zorblax→Zorblux`, golden L06 `expected_handy`): Wispr keeps both silently (E223); a warning needs a new notice and UI.
