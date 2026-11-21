import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";
import { PALETTE_IDS } from "../src/lib/themes";

test("finding added models has an accessible name and description", async ({
  page,
}) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Models", exact: true }).click();
  await page.getByText("Choose or download a local model").click();
  const button = page.getByRole("button", { name: "Find added models" });
  await button.hover();
  await expect(button).toHaveAccessibleDescription(
    "Rescan for models added to the models folder or Hugging Face cache outside Handy Flow",
  );
});

test("dictation pill settings are grouped in appearance and linked from advanced", async ({
  page,
}) => {
  const backend = await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Advanced", exact: true }).click();
  await expect(
    page.getByRole("radio", { name: "Live", exact: true }),
  ).toHaveCount(0);
  await page
    .getByRole("button", { name: "Dictation pill → Appearance" })
    .click();
  const group = page.locator("#dictation-pill");
  await expect(group.getByRole("radio", { name: "Expressive" })).toBeVisible();
  await expect(group.getByRole("radio", { name: "Live" })).toBeVisible();
  await group.getByRole("radio", { name: "Hidden" }).click();
  await expect(group.getByRole("button", { name: "Bottom" })).toBeVisible();
  await expect
    .poll(async () =>
      (await backend.calls()).some(
        (call) =>
          call.cmd === "change_overlay_style_setting" &&
          (call.args as { style: string }).style === "none",
      ),
    )
    .toBe(true);
});

test("a hidden overlay can be changed using the keyboard", async ({ page }) => {
  await mockTauri(page, { settings: { overlay_style: "none" } });
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  const pill = page.getByRole("radio", { name: "Pill", exact: true });
  await page.getByRole("radio", { name: "Hidden", exact: true }).focus();
  await page.keyboard.press("ArrowRight");
  await expect(pill).toBeFocused();
  await page.keyboard.press("ArrowRight");
  await expect(
    page.getByRole("radio", { name: "Wave", exact: true }),
  ).toHaveAttribute("aria-checked", "true");
});

test("French shortcut choices explain how recording starts and stops", async ({
  page,
}) => {
  await mockTauri(page, { settings: { app_language: "fr" } });
  await page.goto("/");
  await page.getByRole("button", { name: /^Raccourcis/ }).click();
  await page
    .getByRole("button", {
      name: "Maintenez pour enregistrer ; appuyez pour démarrer ou arrêter",
    })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Enregistre tant que le raccourci est maintenu",
    }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: "Appuyez sur le raccourci pour démarrer l'enregistrement, appuyez à nouveau pour arrêter",
    }),
  ).toBeVisible();
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

test("saved and rewritten history shows its state and switches text", async ({
  page,
}) => {
  await mockTauri(page, {
    results: {
      get_history_entries: {
        entries: [
          {
            id: 1,
            file_name: "test.wav",
            timestamp: 1700000000,
            saved: true,
            title: "Test",
            transcription_text: "Original words",
            post_processed_text: "Rewritten words",
            post_process_prompt: null,
            post_process_requested: true,
            source_name: null,
            language: "en",
            language_source: null,
            app: null,
          },
        ],
        has_more: false,
      },
    },
  });
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "clipboard", {
      value: {
        writeText: (text: string) => {
          sessionStorage.setItem("copied", text);
          return Promise.resolve();
        },
      },
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  const star = page.getByRole("button", { name: "Remove from saved" });
  await expect(star).toHaveAttribute("aria-pressed", "true");
  await expect(star).toHaveCSS("opacity", "1");
  await expect(page.getByText("Rewritten", { exact: true })).toBeVisible();
  await expect(page.getByText("Rewritten words")).toBeVisible();
  await page
    .getByRole("button", { name: "Copy transcription to clipboard" })
    .click();
  expect(await page.evaluate(() => sessionStorage.getItem("copied"))).toBe(
    "Rewritten words",
  );
  await page.getByRole("button", { name: "View original" }).click();
  await expect(page.getByText("Original words")).toBeVisible();
  await page
    .getByRole("button", { name: "Copy transcription to clipboard" })
    .click();
  expect(await page.evaluate(() => sessionStorage.getItem("copied"))).toBe(
    "Original words",
  );
  await page.getByRole("button", { name: "View rewritten version" }).click();
  await expect(page.getByText("Rewritten words")).toBeVisible();
});

test("empty history shows the configured shortcut", async ({ page }) => {
  await mockTauri(page, {
    settings: {
      bindings: {
        transcribe: {
          id: "transcribe",
          name: "Dictate",
          description: "",
          default_binding: "ctrl+shift+k",
          current_binding: "ctrl+shift+k",
        },
      },
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(
    page.getByText("No transcriptions yet", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Start dictating with Ctrl + Shift + K"),
  ).toBeVisible();
});

test("history audio retains playback and keyboard seeking", async ({
  page,
}) => {
  await mockTauri(page, {
    results: {
      get_audio_file_path: "test.wav",
      get_history_entries: {
        entries: [
          {
            id: 1,
            file_name: "test.wav",
            timestamp: 1700000000,
            saved: false,
            title: "Test",
            transcription_text: "Audio test",
            post_processed_text: null,
            post_process_prompt: null,
            post_process_requested: false,
            source_name: null,
            language: null,
            language_source: null,
            app: null,
          },
        ],
        has_more: false,
      },
    },
  });
  await page.addInitScript(() => {
    Object.defineProperty(HTMLMediaElement.prototype, "duration", {
      get: () => 60,
    });
    HTMLMediaElement.prototype.play = function () {
      this.dispatchEvent(new Event("loadedmetadata"));
      this.dispatchEvent(new Event("play"));
      return Promise.resolve();
    };
    HTMLMediaElement.prototype.pause = function () {
      this.dispatchEvent(new Event("pause"));
    };
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(
    page.getByRole("slider", { name: "Playback position" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Play audio" }).click();
  const position = page.getByRole("slider", { name: "Playback position" });
  await expect(position).toBeVisible();
  await expect(page.getByText("1:00")).toBeVisible();
  await page.getByRole("button", { name: "Pause audio" }).click();
  await position.focus();
  await page.keyboard.press("ArrowRight");
  await expect(position).not.toHaveValue("0");
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
        page.getByRole("button", { name: /^Shortcuts/ }),
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

test("pill tone can be changed in appearance", async ({ page }) => {
  const backend = await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Appearance", exact: true }).click();
  await page.getByRole("radio", { name: "Expressive", exact: true }).click();
  await expect
    .poll(async () =>
      (await backend.calls()).some(
        (c) =>
          c.cmd === "change_pill_tone_setting" &&
          (c.args as { tone: string }).tone === "expressive",
      ),
    )
    .toBe(true);
});
