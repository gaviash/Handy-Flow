// Checks every palette in src/styles/palettes.css: all 13 tokens present and
// the spec's WCAG contrast floors met in light and dark.
// Run with: bun scripts/check-themes.ts
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dir, "..");

export const TOKENS = [
  "bg", "ink", "accent", "on-accent", "accent-text", "side", "side-ink",
  "pill", "pill-ink", "pill-accent", "icon-bg", "icon-fg", "icon-dot",
] as const;

type Mode = "light" | "dark";
type Palette = { id: string; vars: Record<string, string> };

export function parsePalettes(css: string): Palette[] {
  const out: Palette[] = [];
  const block = /\[data-palette="([a-z]+)"\]\s*\{([^}]*)\}/g;
  for (const m of css.matchAll(block)) {
    const vars: Record<string, string> = {};
    for (const d of m[2].matchAll(/--([a-z0-9-]+)\s*:\s*([^;]+);/g)) vars[d[1]] = d[2].trim();
    out.push({ id: m[1], vars });
  }
  return out;
}

/** Resolves a token to a hex color for one mode (follows var() and light-dark()). */
export function resolve(p: Palette, token: string, mode: Mode, depth = 0): string {
  if (depth > 5) throw new Error(`${p.id}: --${token} loops`);
  const raw = p.vars[token];
  if (raw === undefined) throw new Error(`${p.id}: --${token} missing`);
  const ld = raw.match(/^light-dark\(\s*([^,]+?)\s*,\s*([^)]+?)\s*\)$/);
  const v = ld ? (mode === "light" ? ld[1] : ld[2]) : raw;
  const ref = v.match(/^var\(--([a-z0-9-]+)\)$/);
  if (ref) return resolve(p, ref[1], mode, depth + 1);
  if (!/^#[0-9a-f]{6}$/i.test(v)) throw new Error(`${p.id}: --${token} is not a hex color: ${v}`);
  return v.toLowerCase();
}

const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255);
const lin = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const lum = (hex: string) => {
  const [r, g, b] = rgb(hex).map(lin);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};
export function contrast(a: string, b: string): number {
  const [x, y] = [lum(a), lum(b)].sort((m, n) => n - m);
  return (x + 0.05) / (y + 0.05);
}

/** sRGB mix, close enough to the CSS oklab mix for a contrast floor. */
export function mix(a: string, b: string, pctA: number): string {
  const [ra, rb] = [rgb(a), rgb(b)];
  return "#" + ra.map((c, i) => Math.round((c * pctA + rb[i] * (1 - pctA)) * 255).toString(16).padStart(2, "0")).join("");
}

export const FLOORS: [string, string, number][] = [
  ["ink", "bg", 7],
  ["accent-text", "bg", 4.5],
  ["on-accent", "accent", 4.5],
  ["pill-ink", "pill", 4.5],
  ["side-ink", "side", 4.5],
  ["pill-accent", "pill", 3],
];

export function checkPalette(p: Palette): string[] {
  const errors: string[] = [];
  for (const t of TOKENS) if (!(t in p.vars)) errors.push(`${p.id}: --${t} missing`);
  if (errors.length) return errors;
  for (const mode of ["light", "dark"] as const) {
    const c = (t: string) => resolve(p, t, mode);
    for (const [fg, bg, floor] of FLOORS) {
      const r = contrast(c(fg), c(bg));
      if (r < floor) errors.push(`${p.id} ${mode}: ${fg}/${bg} = ${r.toFixed(2)} < ${floor}`);
    }
    // ink-2 on the raised surface (derived the way palettes.css derives them).
    const k = parseFloat(p.vars["ink-2-k"] ?? "72") / 100;
    const raised = mode === "light" ? mix("#ffffff", c("bg"), 0.7) : mix(c("ink"), c("bg"), 0.06);
    const ink2 = mix(c("ink"), c("bg"), k);
    const r = contrast(ink2, raised);
    if (r < 4.5) errors.push(`${p.id} ${mode}: ink-2/raised = ${r.toFixed(2)} < 4.5`);
  }
  return errors;
}

if (import.meta.main) {
  const css = readFileSync(join(ROOT, "src/styles/palettes.css"), "utf8");
  const palettes = parsePalettes(css);
  const { PALETTE_IDS } = await import(join(ROOT, "src/lib/themes.ts"));
  const errors = palettes.flatMap(checkPalette);
  const ids = palettes.map((p) => p.id).join(",");
  if (ids !== PALETTE_IDS.join(",")) errors.push(`palettes.css ids [${ids}] != themes.ts [${PALETTE_IDS.join(",")}]`);
  if (errors.length) {
    console.error(`Theme check failed (${errors.length}):\n  ${errors.join("\n  ")}`);
    process.exit(1);
  }
  console.log(`Themes OK: ${palettes.length} palettes × light/dark meet every contrast floor.`);
}
