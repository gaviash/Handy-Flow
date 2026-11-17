import { test, expect, type Page } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

/** A CSS color resolved by the browser, for comparing tokens and paints. */
const resolve = (page: Page, css: string) =>
  page.evaluate((c) => {
    const probe = document.createElement("div");
    probe.style.color = c;
    document.body.appendChild(probe);
    const out = getComputedStyle(probe).color;
    probe.remove();
    return out;
  }, css);

const muteSwitch = (page: Page) =>
  page.getByRole("checkbox", { name: "Mute While Recording" });

test("a checked switch paints the palette accent", async ({ page }) => {
  await mockTauri(page, { settings: { mute_while_recording: true } });
  await page.goto("/");
  const track = muteSwitch(page).locator("xpath=following-sibling::*[1]");
  await expect(track).toBeVisible();
  const bg = await track.evaluate((el) => getComputedStyle(el).backgroundColor);
  expect(bg).toBe(await resolve(page, "var(--accent)"));
});

test("primary button text uses on-accent, not white", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Advanced" }).click();
  await page.getByPlaceholder("Add a word").fill("example");
  const add = page.getByRole("button", { name: "Add", exact: true });
  const color = await add.evaluate((el) => getComputedStyle(el).color);
  expect(color).toBe(await resolve(page, "var(--on-accent)"));
});

test("settings rows follow the density setting", async ({ page }) => {
  await mockTauri(page, { settings: { ui_density: "compact" } });
  await page.goto("/");
  const row = page
    .getByText("Mute While Recording")
    .locator("xpath=ancestor::*[contains(@class,'row-setting')][1]");
  const compact = await row.evaluate((el) => el.getBoundingClientRect().height);
  await page.evaluate(
    () => (document.documentElement.dataset.density = "comfortable"),
  );
  const comfy = await row.evaluate((el) => el.getBoundingClientRect().height);
  expect(comfy).toBeGreaterThan(compact);
});

test("reduced motion stops the switch transition", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await mockTauri(page);
  await page.goto("/");
  const track = muteSwitch(page).locator("xpath=following-sibling::*[1]");
  const d = await track.evaluate(
    (el) => getComputedStyle(el, "::after").transitionDuration,
  );
  expect(d).toBe("0s");
});
