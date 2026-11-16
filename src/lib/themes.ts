/** Palette ids, in display order. Must match src/styles/palettes.css and the
 * Rust `Palette` enum (both checked by tests). */
export const PALETTE_IDS = [
  "rose",
  "sauge",
  "prune",
  "encre",
  "ocean",
  "ambre",
  "lime",
  "contraste",
] as const;

export type PaletteId = (typeof PALETTE_IDS)[number];

export const isPaletteId = (value: unknown): value is PaletteId =>
  typeof value === "string" &&
  (PALETTE_IDS as readonly string[]).includes(value);
