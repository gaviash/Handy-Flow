import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

test("settings titles align and locked updater has a clean footer", async ({
  page,
}) => {
  await mockTauri(page);
  await page.goto("/");
  const generalX = await page
    .locator("h1")
    .evaluate((el) => el.getBoundingClientRect().x);
  await page.getByRole("button", { name: "Models", exact: true }).click();
  const modelsX = await page
    .locator("h1")
    .evaluate((el) => el.getBoundingClientRect().x);
  await expect(page.getByText("Update Checking Disabled")).toHaveCount(0);
  await expect(
    page.getByText("Handy Flow", { exact: true }).last(),
  ).toBeVisible();
  await expect(page.getByText("v", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: /Local:/ }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(
    "Transcription models",
  );
  expect(modelsX).toBe(generalX);
});

test("settings pages do not skip heading levels", async ({ page }) => {
  await mockTauri(page, { settings: { post_process_enabled: true } });
  await page.goto("/");
  for (const section of [
    "General",
    "History",
    "Models",
    "Post Process",
    "Appearance",
    "Advanced",
    "About",
  ]) {
    await page.getByRole("button", { name: section, exact: true }).click();
    const levels = await page
      .locator("h1, h2, h3, h4, h5, h6")
      .evaluateAll((headings) => headings.map((h) => Number(h.tagName[1])));
    for (let i = 1; i < levels.length; i++)
      expect(levels[i] - levels[i - 1], section).toBeLessThanOrEqual(1);
  }
});
