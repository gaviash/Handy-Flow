// Keeps the rename to "Handy Flow" from regressing: no translation may call
// the app "Handy" alone, and both HTML entry points carry the new title.
// Run with: bun scripts/check-branding.ts
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");
const LOCALES = join(ROOT, "src/i18n/locales");

/** Keys that name the original project on purpose (credit). */
export const ALLOW = new Set(["about.forkOf"]);

const BARE_HANDY = /\bHandy\b(?! Flow)/;

function flatten(obj: unknown, prefix = "", out: [string, string][] = []) {
  if (typeof obj === "string") out.push([prefix, obj]);
  else if (obj && typeof obj === "object")
    for (const [k, v] of Object.entries(obj))
      flatten(v, prefix ? `${prefix}.${k}` : k, out);
  return out;
}

export function bareHandyKeys(json: unknown): string[] {
  return flatten(json)
    .filter(([key, value]) => !ALLOW.has(key) && BARE_HANDY.test(value))
    .map(([key]) => key);
}

if (import.meta.main) {
  const failures: string[] = [];
  for (const locale of readdirSync(LOCALES)) {
    const file = join(LOCALES, locale, "translation.json");
    const json = JSON.parse(readFileSync(file, "utf8"));
    for (const key of bareHandyKeys(json)) failures.push(`${locale}:${key}`);
  }
  for (const html of ["index.html", "src/overlay/index.html"]) {
    if (
      !readFileSync(join(ROOT, html), "utf8").includes(
        "<title>Handy Flow</title>",
      )
    )
      failures.push(`${html}: <title>Handy Flow</title> missing`);
  }
  if (failures.length) {
    console.error(
      `Bare "Handy" found (${failures.length}):\n  ${failures.join("\n  ")}`,
    );
    process.exit(1);
  }
  console.log("Branding OK: every locale says Handy Flow.");
}
