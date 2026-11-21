// Every AppSettings field must still have a control somewhere in the settings
// UI, or be listed below as internal. Fails when a redesign drops a setting.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const bindings = readFileSync("src/bindings.ts", "utf8");
const start = bindings.indexOf("export type AppSettings = {");
const block =
  start < 0
    ? null
    : bindings
        .slice(start, bindings.indexOf("\nexport type ", start))
        .replace(/\/\*\*[\s\S]*?\*\//g, "");
if (!block) throw new Error("AppSettings not found in bindings.ts");
const fields = [...block.matchAll(/(?:;|\{)\s*(\w+)\??:/g)]
  .map((m) => m[1])
  .filter((f, i, all) => all.indexOf(f) === i);

/** Set by the app itself, or edited elsewhere than Settings (onboarding, tray, overlay). */
const INTERNAL = new Set([
  "bindings", // ShortcutInput by id
  "onboarding_completed",
  "selected_model",
  "debug_mode",
  "log_level",
  "post_process_prompts", // PromptEditor through its own commands
  "post_process_selected_prompt_id",
  "post_process_providers", // ProvidersCard through its own commands
  "post_process_api_keys",
  "post_process_models",
  "post_process_provider_id",
  "features",
  "provider_models",
  "selected_language", // CloudAsrSettings / language selector
  "settings_schema_version", // migration markers
  "adaptive_settings_version",
  "features_migrated",
  "post_process_fallback_provider_id", // legacy, migrated into `features`
  "learning_judge_provider_id",
  "custom_filler_words", // settings file only, never had a control
]);

const files: string[] = [];
const walk = (dir: string) => {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p);
    else if (/\.tsx?$/.test(name)) files.push(readFileSync(p, "utf8"));
  }
};
walk("src/components");
const ui = files.join("\n");

const missing = fields.filter(
  (f) => !INTERNAL.has(f) && !ui.includes(`"${f}"`),
);
if (missing.length) {
  console.error(`Settings with no UI: ${missing.join(", ")}`);
  process.exit(1);
}
console.log(`✓ ${fields.length} settings: all have a control or are internal.`);
