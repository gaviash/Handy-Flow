# Handoff — Settings redesign (plan C)

Branch `feat/ui-refonte`, created from `feat/handy-flow-brand` (`6a7487c`). Not pushed.
Plan: `docs/superpowers/plans/2026-11-17-ui-refonte.md`.

## What changed

Each tab opens on a **status header** (one sentence: what will happen), then **cards** that summarise a topic and open a **sub-page** (back button, focus returned to the card). Rarely used settings are **folded** under "Réglages experts".

| Tab | Content |
|---|---|
| Général | status (shortcuts, mic) · Shortcuts · Languages (+ interface language) · Microphone · Sounds · Startup |
| Modèles | status (transcription + AI features) · Providers · Usage by feature · Local model · Expert engine |
| Amélioration | on/off switch in the header · status · Provider (summary → Models) · Tone (pills) · Transformations · Cleanup · Shortcut · Expert (delay, prompt) |
| Vocabulaire (new) | status · learning + name · Personal words · Learned words (+ counters) · Style rules · Learning validator · Snippets · Recognition aid · Expert |
| Contexte (new) | status (is anything leaving the PC) · read / send / screen · Exclusions · Last dictations · Expert (sources, limits, VS Code on Windows only) |
| Historique | Retention card at the top (count shown only for "keep the last N") |
| Avancé | Text insertion card · Expert audio capture |
| À propos | Updates · Diagnostics cards |
| Débogage | debug tools + Experimental + keyboard backend |

P1 fixes from the audit:
- The 30-day retention is accepted.
- A failed save rolls back only its own setting and shows a notice.
- Silent failures (history, prompts, models) now show a notice.
- A history deletion can be undone for 8 s.

Other fixes:
- An active learned word no longer shows an expiry date.
- A validator with no usable model says "local decision (lexicon)".
- Clicking the active tab returns to its overview.
- The links from Enhancement and Vocabulary open Models straight on "Usage by feature".
- FR: one name, "Amélioration". Each setting title names its field for screen readers.

## Checks run (Linux)

- `cargo test --lib`: 537 passed. `cargo fmt --check` passes. The clippy warnings all predate this branch.
- Playwright (`--ignore-snapshots`): all green.
- `bun run check:translations`: 25/25. `check:themes`: passes. `bun scripts/check-settings-coverage.ts`: 95/95.

## Default decisions applied (reversible)

- Appearance stays a tab. Snippets go to Vocabulary. FR tab label: "Amélioration".
- French copy uses "vous", like the rest of the FR locale (the plan used "tu").
- Counts are written as "Total : n", which avoids plural keys in 26 languages.
- The capture settings that used to sit behind "Expérimental" are now reachable without it.
- `experimental_enabled` no longer gates anything. Its switch is kept in Debug, not removed.

## Not done in this session (Windows only)

1. Regenerate `src/bindings.ts` with `bun run tauri dev`. It is still hand-edited since plan B.
2. The manual checks from plan B (Task 9).
3. The `fast` builds and replacing `Handy Flow.exe` (test on a **copy** of `Data\`).
4. The Playwright screenshot references `-win32.png` for `general-*` and `appearance-*` will no longer match, because General was redesigned. Regenerate them on Windows with `--update-snapshots` after checking them by eye.

## Five things to check by hand

1. Amélioration: the header switch turns the rewrite on and off, and the status sentence follows.
2. Vocabulaire: the Learning validator card shows the real model. "Change in Models" opens Usage by feature.
3. Historique: delete a dictation, click Undo. The dictation is still there after restarting the app.
4. Contexte: turning sending off changes the sentence to "Read on your PC, nothing is sent".
5. Every setting you use is still there with its value (General, Advanced, Debug).
