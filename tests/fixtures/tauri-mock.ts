// A stand-in for the Tauri backend, so Playwright can drive the real frontend
// in a plain headless browser: no Rust, no microphone, no keyboard hooks.
import type { Page } from "@playwright/test";
import { readFileSync } from "node:fs";

export const DEFAULT_SETTINGS = JSON.parse(
  readFileSync(new URL("./app-settings.json", import.meta.url), "utf8"),
);

export type Call = { cmd: string; args: unknown };

/** An empty value of each command's declared return type (from bindings.ts),
 * so commands nobody stubbed still return something the UI can iterate. */
function emptyResults(): Record<string, unknown> {
  const src = readFileSync(
    new URL("../../src/bindings.ts", import.meta.url),
    "utf8",
  );
  const out: Record<string, unknown> = {};
  const re =
    /async \w+\([^)]*\) : Promise<(.+?)> \{\s*(?:try \{\s*)?return (?:\{ status: "ok", data: )?await TAURI_INVOKE\("([^"]+)"/g;
  for (const [, ret, cmd] of src.matchAll(re)) {
    const t = ret.startsWith("Result<")
      ? ret.slice(7, ret.lastIndexOf(",")).trim()
      : ret;
    out[cmd] =
      t.endsWith("[]") || t.startsWith("Array<")
        ? []
        : t === "boolean"
          ? false
          : t === "number"
            ? 0
            : t === "string"
              ? ""
              : null;
  }
  return out;
}

export interface MockOptions {
  /** Merged over AppSettings::default(). */
  settings?: Record<string, unknown>;
  /** Extra command results, by command name (raw data, as Rust returns it). */
  results?: Record<string, unknown>;
  /** Window label the page believes it is ("main" or "recording_overlay"). */
  label?: string;
}

/** Results that keep the app past onboarding with one ready model. */
const MODEL = {
  id: "qwen3-asr-1.7b",
  name: "Qwen3-ASR 1.7B",
  description: "",
  filename: "qwen3-asr-1.7b-q8_0.gguf",
  source: { type: "Catalog" },
  size_mb: 2100,
  is_downloaded: true,
  is_downloading: false,
  partial_size: 0,
  accuracy_score: 0.9,
  speed_score: 0.6,
  supports_translation: false,
  is_recommended: true,
  supported_languages: ["fr", "en"],
  supports_language_selection: true,
  is_custom: false,
  supports_streaming: false,
  supports_language_detection: true,
};

const BASE_RESULTS: Record<string, unknown> = {
  get_available_models: [MODEL],
  get_current_model: MODEL.id,
  has_any_models_available: true,
  has_any_models_or_downloads: true,
  is_portable: true,
  is_update_checks_locked: true,
  get_app_dir_path: "C:\\Handy Flow\\Data",
  get_history_entries: { entries: [], has_more: false },
  get_active_notices: [],
  check_custom_sounds: { start: false, stop: false },
  get_windows_microphone_permission_status: { supported: false },
  get_available_accelerators: { transcribe: ["auto"], gpu_devices: [] },
  get_available_microphones: [],
  get_retired_models: [],
  get_available_output_devices: [],
};

export async function mockTauri(page: Page, opts: MockOptions = {}) {
  // A user past onboarding, as on the machines Handy Flow is used on.
  const settings = {
    ...DEFAULT_SETTINGS,
    onboarding_completed: true,
    ...opts.settings,
  };
  const results = {
    ...emptyResults(),
    ...BASE_RESULTS,
    get_default_settings: DEFAULT_SETTINGS,
    ...opts.results,
    get_app_settings: settings,
  };
  await page.addInitScript(
    ({ results, label }) => {
      const w = window as any;
      const calls: { cmd: string; args: unknown }[] = (w.__calls = []);
      const callbacks = new Map<number, (data: unknown) => void>();
      const listeners = new Map<string, number[]>();
      let next = 1;
      w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
      w.__TAURI_OS_PLUGIN_INTERNALS__ = {
        platform: "windows",
        os_type: "windows",
        family: "windows",
        arch: "x86_64",
        version: "10.0.26300",
        eol: "\r\n",
        exe_extension: "exe",
      };
      w.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label },
          currentWebview: { windowLabel: label, label },
        },
        transformCallback(cb: (d: unknown) => void) {
          const id = next++;
          callbacks.set(id, cb);
          return id;
        },
        unregisterCallback: (id: number) => callbacks.delete(id),
        runCallback: (id: number, data: unknown) => callbacks.get(id)?.(data),
        convertFileSrc: (p: string) => p,
        async invoke(cmd: string, args: any) {
          if (cmd === "plugin:event|listen") {
            const list = listeners.get(args.event) ?? [];
            list.push(args.handler);
            listeners.set(args.event, list);
            return args.handler;
          }
          if (cmd === "plugin:event|unlisten") return null;
          if (cmd === "plugin:event|emit" || cmd === "plugin:event|emit_to") {
            for (const id of listeners.get(args.event) ?? [])
              callbacks.get(id)?.({
                event: args.event,
                id,
                payload: args.payload,
              });
            return null;
          }
          calls.push({ cmd, args });
          if (cmd in results) return structuredClone(results[cmd]);
          // change_*_setting(value) keeps get_app_settings in step.
          return null;
        },
      };
      w.__emit = (event: string, payload: unknown) => {
        for (const id of listeners.get(event) ?? [])
          callbacks.get(id)?.({ event, id, payload });
      };
    },
    { results, label: opts.label ?? "main" },
  );
  return {
    calls: () => page.evaluate(() => (window as any).__calls as Call[]),
    emit: (event: string, payload: unknown) =>
      page.evaluate(([e, p]) => (window as any).__emit(e, p), [
        event,
        payload,
      ] as const),
  };
}
