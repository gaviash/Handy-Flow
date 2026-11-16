// Standalone assert check (no JS unit-test runner in this repo). Run with:
//   bun src/lib/utils/appearance.test.ts
import assert from "node:assert";

// A fake localStorage the module sees through globalThis.
const store = new Map<string, string>();
let storageThrows = false;
(globalThis as any).localStorage = {
  getItem: (k: string) => {
    if (storageThrows) throw new Error("denied");
    return store.get(k) ?? null;
  },
  setItem: (k: string, v: string) => {
    if (storageThrows) throw new Error("denied");
    store.set(k, v);
  },
};

const {
  applyAppearance,
  getStoredAppearance,
  DEFAULT_APPEARANCE,
  APPEARANCE_STORAGE_KEY,
} = await import("./appearance");

const fakeRoot = () => ({ dataset: {} as Record<string, string | undefined> });

// applies every attribute; system mode removes data-theme
{
  const root = fakeRoot();
  root.dataset.theme = "dark";
  applyAppearance(
    {
      theme: "system",
      palette: "sauge",
      heading_style: "modern",
      ui_density: "compact",
      overlay_show_label: false,
    },
    root as unknown as HTMLElement,
  );
  assert.equal(root.dataset.palette, "sauge");
  assert.equal(root.dataset.theme, undefined);
  assert.equal(root.dataset.headings, "modern");
  assert.equal(root.dataset.density, "compact");
  assert.equal(root.dataset.overlayLabel, "false");
  assert.equal(JSON.parse(store.get(APPEARANCE_STORAGE_KEY)!).palette, "sauge");
}

// light/dark set data-theme
{
  const root = fakeRoot();
  applyAppearance({ ...DEFAULT_APPEARANCE, theme: "dark" }, root as unknown as HTMLElement);
  assert.equal(root.dataset.theme, "dark");
}

// corrupt storage → defaults
store.clear();
store.set(APPEARANCE_STORAGE_KEY, "{oops");
assert.deepEqual(getStoredAppearance(), DEFAULT_APPEARANCE);

// unknown palette → rose, other fields kept
store.set(
  APPEARANCE_STORAGE_KEY,
  JSON.stringify({ ...DEFAULT_APPEARANCE, palette: "neon", ui_density: "compact" }),
);
assert.equal(getStoredAppearance().palette, "rose");
assert.equal(getStoredAppearance().ui_density, "compact");

// storage that throws → defaults, no throw; apply still works
storageThrows = true;
assert.deepEqual(getStoredAppearance(), DEFAULT_APPEARANCE);
applyAppearance(DEFAULT_APPEARANCE, fakeRoot() as unknown as HTMLElement);
storageThrows = false;

// legacy key handy.theme="dark" → theme dark, palette rose
store.clear();
store.set("handy.theme", "dark");
assert.deepEqual(getStoredAppearance(), { ...DEFAULT_APPEARANCE, theme: "dark" });

assert.equal(DEFAULT_APPEARANCE.palette, "rose");
assert.equal(DEFAULT_APPEARANCE.theme, "system");
console.log("appearance.test.ts: all assertions passed");
