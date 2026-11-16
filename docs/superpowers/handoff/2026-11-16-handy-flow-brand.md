# Handoff — Handy Flow brand (stopped at Task 10)

**Plan:** `docs/superpowers/plans/2026-11-16-handy-flow-brand.md` · **Spec:** `docs/superpowers/specs/2026-11-16-handy-flow-brand-design.md` · **Visual reference:** `docs/branding/pilote-handy-flow-v2.html`

**Where:** branch `feat/handy-flow-brand`, worktree `Handy/.claude/worktrees/handy-flow-brand`. Not pushed. Execution mode: inline (superpowers:executing-plans), with the `impeccable` skill for the UI tasks.

**Detailed ledger** (git-ignored, read it first): `.superpowers/sdd/2026-11-16-handy-flow-brand/progress.md`. It lists every task done and every ruling.

## Done (committed)

| Task | Result |
|---|---|
| 0 | Fonts: Figtree and Instrument Serif in woff2, plus `OFL.txt` |
| 1 | Name changed to Handy Flow (`productName`, `mainBinaryName`, titles, tray tooltip, CLI); updater forced off; `identifier` unchanged |
| 2 | 26 locales renamed; `bun run check:branding` |
| 3 | Windows autostart: removes the old `Handy` Run entry only if it points to this install |
| 4 | Icons per palette (`scripts/render-icons.ts`) and `tauri icon`. Build 1 OK: `target/fast/Handy Flow.exe`, ProductName "Handy Flow" |
| 5 | Rust settings `palette`, `heading_style`, `ui_density`, `overlay_show_label`, plus commands and the `appearance-changed` event; `bindings.ts` regenerated |
| 6 | `src/styles/palettes.css` (8 palettes, `light-dark()`), `theme.css` cut down to aliases; `bun run check:themes` (WCAG) |
| 7 | `src/lib/utils/appearance.ts` (+ test `bun run test:appearance`); `theme.ts` removed |
| 8 | Playwright harness with a mocked Tauri IPC (`tests/fixtures/tauri-mock.ts`, port 1430, installed Chrome); `tests/branding.spec.ts` 3/3; house prompt renamed "Handy Flow" (by id, idempotent) |
| 9 | `HandyFlowWordmark`, bundled fonts, sidebar redone (nav of buttons, Appearance section), `ThemeSelector` moved from About to Appearance (minimal stub) |

Last green state: Rust `cargo test --profile fast --lib` 418/418; check:branding, check:themes, check:translations, test:appearance, Playwright branding all pass; lint clean.

## In progress: Task 10 (UI primitives)

- `tests/ui.spec.ts` is written (WIP, committed with this note). It is in RED, as expected:
  - The "switch paints accent" test already passes, because the alias `background-ui` = `accent` comes from Task 6.
  - Three tests fail: on-accent button, density, and reduced-motion on the switch.
  - The last run (on-accent Donate button) was interrupted.
- What remains: implement what the plan's Task 10 says.
  - `Switch.tsx`: 40×24, `bg-accent` when on, knob `on-accent`, 250ms ease-glide.
  - `Button.tsx` primary: `text-on-accent bg-accent` (white on light pink fails contrast).
  - `SettingsGroup`/`SettingContainer`: class `row-setting`, padding following `data-density`, `bg-raised`, radius 14px.
  - New `Segmented.tsx` (its keyboard test goes in `appearance.spec`, Task 11).
  - Global `:focus-visible`, `::selection`, scrollbars, and global `prefers-reduced-motion` in `App.css`.
- Then Tasks 11 to 15 as the plan says.

## Rules (from the user — must follow)

- **NEVER replace or touch `Desktop\Handy Flow`.** Another version will be installed there. Every build stays in the worktree.
- No test that takes over the PC (keyboard, focus, audio). Playwright runs headless only.
- Builds: `bun run tauri build --no-bundle -- --profile fast`, one per increment. Never run `bun run build` or edit sources while a build is running.
- Do not touch the uncommitted cloud_asr work in the main checkout (`Handy/`, branch `feat/adaptive-learning-context`).

## Practical gotchas

- Rust tests: `PATH="$(pwd)/src-tauri/transcribe-libs:$PATH" cargo test --manifest-path src-tauri/Cargo.toml --profile fast --lib`. Without that PATH entry the test binary fails with STATUS_DLL_NOT_FOUND.
- `bindings.ts`:
  - `cargo build`, then `src-tauri/target/debug/handy.exe --list-devices` (headless; no window or shortcuts).
  - The export path is relative to the current directory (`../src/bindings.ts`). Run it **from `src-tauri/`**, otherwise the file lands in `.claude/worktrees/src/`.
- Settings fixture: `UPDATE_FIXTURES=1` + the cargo test `default_settings_fixture_is_current` → `tests/fixtures/app-settings.json`.
- Only run Prettier on the files you touched: it reformats others (line endings) that must be restored.
- The worktree guard refuses compound shell commands that contain `git`: run `git` on its own, with literal arguments.
- `src-tauri/target/fast` was seeded from the main checkout (stale CMake caches were removed). Incremental builds now take about 1–4 min.

## Also done during the session (user request)

193 branches deleted from the GitHub fork (`gaviash/Handy-Flow`). They were exact copies of `cjpais/Handy` branches. Kept: `main`, `feat/adaptive-learning-context`, `adaptive/etape-1`. The main checkout still has stale `origin/*` remote-tracking refs (`git fetch --prune` cleans them up).
