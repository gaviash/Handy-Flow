import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";
import { PALETTE_IDS } from "../src/lib/themes";

test("a hidden overlay can be changed using the keyboard", async ({ page }) => {
  await mockTauri(page, { settings: { overlay_style: "none" } });
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  const pill = page.getByRole("radio", { name: "Pill", exact: true });
  await expect(pill).toHaveAttribute("tabindex", "0");
  await pill.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("radio", { name: "Wave", exact: true }),
  ).toHaveAttribute("aria-checked", "true");
});

test("palette previews paint their own surface", async ({ page }) => {
  await mockTauri(page, { settings: { theme: "light", palette: "rose" } });
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  await expect(page.locator('span[data-palette="ocean"]').first()).toHaveCSS(
    "background-color",
    "rgb(244, 247, 251)",
  );
});

test("history actions appear on keyboard focus and hover", async ({ page }) => {
  await mockTauri(page, {
    results: {
      get_history_entries: {
        entries: [
          {
            id: 1,
            file_name: "test.wav",
            timestamp: 1700000000,
            saved: false,
            title: "Test",
            transcription_text: "A saved dictation",
            post_processed_text: null,
            post_process_prompt: null,
            post_process_requested: false,
            source_name: null,
            language: "en",
            language_source: null,
            app: "Editor",
          },
        ],
        has_more: false,
      },
    },
  });
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  const copy = page.getByRole("button", {
    name: "Copy transcription to clipboard",
    exact: true,
  });
  const actions = copy.locator("..");
  await expect(actions).toHaveCSS("opacity", "0");
  await copy.focus();
  await expect(actions).toHaveCSS("opacity", "1");
  await page.getByRole("button", { name: "History", exact: true }).focus();
  await page.getByText("A saved dictation", { exact: true }).hover();
  await expect(actions).toHaveCSS("opacity", "1");
});

test("French appearance controls fit the minimum window", async ({ page }) => {
  await mockTauri(page, { settings: { app_language: "fr" } });
  await page.setViewportSize({ width: 720, height: 600 });
  await page.goto("/");
  await page.getByRole("button", { name: "Apparence", exact: true }).click();
  const wave = page.getByRole("radio", { name: "Onde", exact: true });
  await wave.scrollIntoViewIfNeeded();
  await expect(wave).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(
    720,
  );
  await expect(page).toHaveScreenshot("appearance-fr-minimum.png");
});

test("each palette applies and persists", async ({ page }) => {
  const backend = await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  for (const id of PALETTE_IDS) {
    const card = page
      .getByRole("radio")
      .filter({ has: page.locator(`[data-palette="${id}"]`) });
    await card.click();
    await expect(page.locator("html")).toHaveAttribute("data-palette", id);
    await expect(card).toHaveAttribute("aria-checked", "true");
    expect(await backend.calls()).toContainEqual({
      cmd: "change_palette_setting",
      args: { palette: id },
    });
  }
});

test("mode, headings, density and overlay controls apply with the keyboard", async ({
  page,
}) => {
  const backend = await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  const system = page.getByRole("radio", { name: "System", exact: true });
  await system.focus();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("radio", { name: "Light", exact: true }),
  ).toBeFocused();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await page.keyboard.press("ArrowRight");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByRole("radio", { name: "Modern", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-headings", "modern");
  await page.getByRole("radio", { name: "Compact", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("data-density", "compact");
  await page.getByRole("radio", { name: "Wave", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute(
    "data-overlay-label",
    "false",
  );
  expect(await backend.calls()).toEqual(
    expect.arrayContaining([
      { cmd: "change_overlay_style_setting", args: { style: "minimal" } },
      { cmd: "change_overlay_show_label_setting", args: { enabled: false } },
    ]),
  );
});

test("palette picker supports arrow keys and a reload keeps the choice", async ({
  page,
}) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  await page.getByRole("radio", { name: "Rose", exact: true }).focus();
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("ArrowRight");
  await page.keyboard.press("Enter");
  await expect(page.locator("html")).toHaveAttribute("data-palette", "prune");
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Appearance", exact: true }),
  ).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-palette", "prune");
});

for (const palette of PALETTE_IDS)
  for (const theme of ["light", "dark"]) {
    test(`visual ${palette} ${theme}`, async ({ page }) => {
      await mockTauri(page, { settings: { palette, theme } });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.goto("/");
      await expect(
        page.getByRole("checkbox", { name: "Mute While Recording" }),
      ).toBeVisible();
      await expect(page).toHaveScreenshot(`general-${palette}-${theme}.png`);
      await page
        .getByRole("button", { name: "Appearance", exact: true })
        .click();
      await expect(
        page.getByRole("radio", { name: "Compact", exact: true }),
      ).toBeVisible();
      await expect(page).toHaveScreenshot(`appearance-${palette}-${theme}.png`);
    });
  }
