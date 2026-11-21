# Handoff — Windows checks (plans B and C) and the audio device list

You work on the user's **Windows PC**. Everything here could only be done on Windows: the real Tauri runtime, WASAPI audio, the `-win32` screenshots, the `fast` build and swapping the exe. Write to the user in French. Read `AGENTS.md` first.

## Context

- Repo `gaviash/Handy-Flow`: Windows fork of Handy, Tauri 2 (Rust in `src-tauri/src`, React 18 + Tailwind 4 + Zustand + i18next in `src/`).
- Branch to check: **`feat/handy-flow-brand`** (same content as `main`): plans A, B and C. Plan C (the settings redesign) was merged there directly.
- Read first:
  - `docs/superpowers/handoff/2026-11-17-ui-refonte.md`: what plan C changed, the default decisions, the deferred minor points.
  - `docs/superpowers/plans/2026-11-17-ui-refonte.md`: Global Constraints (machine, builds, never touch `Data\`) and Review Focus.
  - `docs/superpowers/plans/2026-11-19-B-wispr-parity-absorption.md`, Task 9: plan B's manual checks.
- Git rules:
  - Local git author: `gaviash`.
  - No model identifier in commits.
  - Commit messages: conventional, explaining the *why*.
  - Push only if the user asks.
- Never touch `C:\Users\gavri\Desktop\Handy Flow\Data\` or `Desktop\Handy`. Test on a **copy** of `Data\`.
- On this PC, `bunx` doesn't exist and Playwright launches it: put a `bunx.cmd` containing `@bun x %*` on the `PATH`, in a scratch folder, not in the repo.

State at handoff (Linux):
- `cargo test --lib`: 537 passed.
- Playwright (`--ignore-snapshots`): 113/113.
- `check:translations`: 25/25.
- `check:themes`: passes.
- `bun scripts/check-settings-coverage.ts`: 95/95.

## 1. Regenerate the bindings (≈10 min)

1. `git switch feat/handy-flow-brand && git pull && bun install`
2. `bun run tauri dev`, wait for the window, then close it.
3. `git diff src/bindings.ts`. Plan B edited it by hand (`Learned.source`, `Source`, `LearningStats`, `getLearningStats`).
   - Empty diff, or formatting only: commit `chore: regenerate bindings`.
   - Different types: fix the code that uses them, then `bun run build`.

## 2. Full test suite (≈20 min)

```
bun run lint && bun run build && bun run check:translations && bun run check:themes
bun scripts/check-settings-coverage.ts
bun run test:playwright
cd src-tauri && cargo test --release --lib && cargo clippy
```

- `bun run format:check` already failed on 22 files before plan C. Don't fix it in this pass. Only report it.
- **Screenshots:** the `general-*` references (16 files, General redesigned) and maybe `appearance-*` will fail.
  1. Open the diff images under `test-results/`.
  2. Check that only the redesign changed: General now shows a status header and cards.
  3. Run `bun run test:playwright -- --update-snapshots`.
  4. Commit only the `-win32.png` files.
- Anything else red: diagnose it, don't relax the test.

## 3. Audio device list: diagnosis to confirm

The user reports that refreshing the audio devices fails. Static analysis from Linux, not reproduced:

**How it works today**
- The list is read only (a) once after onboarding (`App.tsx:~221`) and (b) **when a menu opens** (`Dropdown` `onOpen` → `refreshAudioDevices` / `refreshOutputDevices`, `settingsStore.ts:~317`).
- There is no listener for plugging or unplugging a device.
- Rust: `get_available_microphones` → `handy_recorder::list_input_devices()` (shared WASAPI host, re-enumerated on every call). Output: `list_output_devices()` in `audio_toolkit/audio/device.rs`.

**Hypotheses, most likely first**
1. **Regression introduced by plan C (commit `fa42e58`).** `updateSetting` now treats a command's `Result` error as "not saved" and **puts the old value back** in the UI. But `set_selected_microphone` (`commands/audio.rs:~236`) **writes the setting to disk before** restarting the stream (`update_selected_device`). If the restart fails (device busy or just unplugged), the UI shows the old microphone and "Réglage non enregistré", while the new one is saved on disk. `set_selected_output_device` and `clamshell_microphone` probably do the same: check them.
2. **No hot-plug.** A device plugged in after launch only appears when you open the menu. The "Microphone" card in General shows the saved name, even if the device is gone. When the menu opens, it shows the old list until the reply arrives.
3. Enumeration that hangs or fails on the Windows side: `get_available_microphones` then sends back only "Default" (`settingsStore.ts:~329`), with no notice.

**Reproduction protocol (dev build, `bun run tauri dev`, debug log)**
1. Général → Microphone: open the menu, note the list.
2. Plug in a USB headset or turn on a Bluetooth one, then reopen the menu. Does it appear? How long does it take?
3. Pick it while another app is holding it (Teams or the recorder): is there a "Réglage non enregistré" notice? Which mic shows in the UI? What's in `settings_store.json` (the **copy**)? Then restart the app: which one is active?
4. Unplug the selected mic: what do the General card, the menu and the dictation show?
5. Same thing for Sons → Expert → output device.
6. In the log: look for `Failed to update selected device` and `Failed to list`.

**Fix if hypothesis 1 is confirmed** (test first, Playwright with the `errors` option of `tests/fixtures/tauri-mock.ts`)
- In `settingsStore.updateSetting`, on error, **read the real value back from Rust** (`commands.getAppSettings()` or the store's `refreshSettings()`) instead of assuming the old one. Fall back to `originalValue` only if that read fails.
- Keep the notice, but its text must not say "not saved" when the value was saved. Suggestion: a separate key for the error message. Ask the user for the wording.
- Don't break the rollback tests (`tests/ui-refonte.spec.ts`: "a failed save rolls back…", "only rolls back its own setting"). Adapt their mock so `get_app_settings` returns the old value.

**If hypothesis 2 is the cause:** refresh the lists when the window regains focus (`focus` event), or listen for a device-change event if `handy-recorder` exposes one (check the crate before writing an `IMMNotificationClient`). Keep it to the smallest change.

Send the user the diagnosis (confirmed or not, with evidence) **before** a large fix.

## 4. Plan B manual checks (≈15 min, the user does the dictation)

You must not drive the user's keyboard, focus or microphone yourself. Prepare, then ask the user to:
1. dictate into Notepad and into Windows Terminal, and check the join with the existing text (spaces, capital letters, punctuation);
2. dictate "point oeuvre", correct it to `.env`: it should be learned at the first correction;
3. you check that the log has one `Learning: candidate …` line per candidate.

## 5. Build and real-world test (≈30 min)

1. `bun run tauri build --no-bundle -- --profile fast` → `src-tauri/target/fast/handy.exe`. Never change the sources during a build.
2. Ask the user which folder to use. Make a **portable copy**: the exe, the DLLs from `src-tauri/transcribe-libs`, `resources\`, and a **copy** of `Data\`. Close the running `Handy Flow.exe` before replacing anything, and back up the old exe first.
3. Ask the user to run the 5 checks (one of them is a dictation, which they do themselves):
   - Amélioration: the header switch, the status sentence follows.
   - Vocabulaire: the Learning validator card shows the real model, and "Modifier dans Modèles" opens "Utilisation par fonction".
   - Historique: delete then Undo, the dictation is still there after a restart.
   - Contexte: turning sending off changes the sentence.
   - All their usual settings are there with their values (the copy of `Data\`).
4. Transformations tab:
   - "New transformation" creates a 4th one (name, instruction).
   - Assign it a shortcut and check that it transforms a selected text.
   - Delete it.
   - `cargo test --lib every_transformation_slot` must pass: 9 slots, and the binding test only runs on Windows.
   - The existing `settings_store.json` must receive `transform_4` to `transform_9` on load ("Adding missing binding" in the log).
5. Plan C Review Focus 7: change the Amélioration model in Modèles → Utilisation par fonction, then check the summary in Amélioration and the Validateur card (in Automatique) **without restarting**.

## 6. Optional (only if the user asks)

The deferred minor points from plan C:
- coming back from the Prompt sub-page loses focus;
- the Fournisseurs and Utilisation par fonction sub-pages show their title twice;
- "Au démarrage de Windows" appears on every OS. Not visible on Windows.

## Final report to the user (in French)

- For each step: ✅ passed, ❌ failed (with the evidence), ⏸️ blocked (why).
- The audio diagnosis: which hypothesis is confirmed, and the evidence.
- The commits made, not pushed.
