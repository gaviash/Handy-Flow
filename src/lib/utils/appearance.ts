import { type Theme } from "@/bindings";
import { isPaletteId, type PaletteId } from "@/lib/themes";

/**
 * Appearance: palette × mode, plus heading style, density and the overlay
 * pill's label. Applied as attributes on the document root:
 *  - `data-theme` (absent for `system`) pins color-scheme; palettes.css uses
 *    light-dark(), so the mode needs nothing else.
 *  - `data-palette`, `data-headings`, `data-density`, `data-overlay-label`.
 *
 * AppSettings is the source of truth. The last-applied value is mirrored to
 * localStorage so it can be applied synchronously on boot, before React
 * mounts, avoiding a flash of the wrong palette.
 */

export type HeadingStyle = "editorial" | "modern";
export type UiDensity = "comfortable" | "compact";

export type Appearance = {
  theme: Theme;
  palette: PaletteId;
  heading_style: HeadingStyle;
  ui_density: UiDensity;
  pill_tone: "discreet" | "expressive";
  overlay_show_label: boolean;
};

export const APPEARANCE_STORAGE_KEY = "handy.appearance";
const LEGACY_THEME_KEY = "handy.theme";

export const THEME_OPTIONS: Theme[] = ["system", "light", "dark"];

export const DEFAULT_APPEARANCE: Appearance = {
  theme: "system",
  palette: "rose",
  heading_style: "editorial",
  ui_density: "comfortable",
  pill_tone: "discreet",
  overlay_show_label: true,
};

const isTheme = (v: unknown): v is Theme =>
  v === "system" || v === "light" || v === "dark";

/** Keeps every valid field of `raw`, defaults the rest. */
export function normalizeAppearance(raw: unknown): Appearance {
  const r = (raw && typeof raw === "object" ? raw : {}) as Record<
    string,
    unknown
  >;
  const d = DEFAULT_APPEARANCE;
  return {
    theme: isTheme(r.theme) ? r.theme : d.theme,
    palette: isPaletteId(r.palette) ? r.palette : d.palette,
    heading_style: r.heading_style === "modern" ? "modern" : "editorial",
    ui_density: r.ui_density === "compact" ? "compact" : "comfortable",
    pill_tone: r.pill_tone === "expressive" ? "expressive" : "discreet",
    overlay_show_label:
      typeof r.overlay_show_label === "boolean"
        ? r.overlay_show_label
        : d.overlay_show_label,
  };
}

/** Apply an appearance to the document root and remember it for next launch. */
export function applyAppearance(
  appearance: Appearance,
  root: HTMLElement = document.documentElement,
): void {
  const a = normalizeAppearance(appearance);
  if (a.theme === "system") delete root.dataset.theme;
  else root.dataset.theme = a.theme;
  root.dataset.palette = a.palette;
  root.dataset.headings = a.heading_style;
  root.dataset.density = a.ui_density;
  root.dataset.pillTone = a.pill_tone;
  root.dataset.overlayLabel = String(a.overlay_show_label);
  try {
    localStorage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify(a));
  } catch {
    // Storage may be unavailable; AppSettings still persists the choice, so
    // this only costs a one-frame flash on the next boot.
  }
}

/** The last-applied appearance, for synchronous boot-time application. */
export function getStoredAppearance(): Appearance {
  try {
    const stored = localStorage.getItem(APPEARANCE_STORAGE_KEY);
    if (stored) return normalizeAppearance(JSON.parse(stored));
    const legacy = localStorage.getItem(LEGACY_THEME_KEY);
    if (isTheme(legacy)) return { ...DEFAULT_APPEARANCE, theme: legacy };
  } catch {
    // Corrupt JSON or storage denied: fall through to defaults.
  }
  return DEFAULT_APPEARANCE;
}

/** Runs `change` inside a View Transition crossfade when the platform has one
 * and the user has not asked for reduced motion. */
export function withThemeTransition(change: () => void): void {
  const doc = document as Document & {
    startViewTransition?: (cb: () => void) => unknown;
  };
  const reduced = window.matchMedia?.(
    "(prefers-reduced-motion: reduce)",
  ).matches;
  if (doc.startViewTransition && !reduced) doc.startViewTransition(change);
  else change();
}

/** Apply the persisted appearance from AppSettings (the source of truth). */
export async function syncAppearanceFromSettings(): Promise<void> {
  try {
    const { commands } = await import("@/bindings");
    const result = await commands.getAppSettings();
    if (result.status === "ok")
      applyAppearance(normalizeAppearance(result.data));
  } catch (e) {
    console.warn("Failed to sync appearance from settings:", e);
  }
}
