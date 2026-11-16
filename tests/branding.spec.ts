import { test, expect, type Page } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

const BARE_HANDY = /\bHandy\b(?!\s*Flow)/;

async function visibleText(page: Page): Promise<string[]> {
  return page.evaluate(() => {
    const out: string[] = [];
    const walker = document.createTreeWalker(
      document.body,
      NodeFilter.SHOW_TEXT,
    );
    while (walker.nextNode()) {
      const node = walker.currentNode as Text;
      const el = node.parentElement;
      if (!el || !node.textContent?.trim()) continue;
      const style = getComputedStyle(el);
      if (style.display === "none" || style.visibility === "hidden") continue;
      // The wordmark splits "Handy" and "Flow" into two elements.
      out.push(
        (el.closest("[aria-label]")?.getAttribute("aria-label") ?? "") +
          "|" +
          node.textContent,
      );
    }
    return out;
  });
}

test.beforeEach(async ({ page }) => {
  await mockTauri(page);
});

test("window title says Handy Flow", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveTitle("Handy Flow");
});

test("sidebar shows the Handy Flow wordmark", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByLabel("Handy Flow").first()).toBeVisible();
});

test("About credits the original project beside the wordmark", async ({
  page,
}) => {
  await page.goto("/");
  await page.getByRole("button", { name: "About", exact: true }).click();
  await expect(
    page.getByText("Based on Handy, MIT license", { exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel("Handy Flow")).toHaveCount(2);
});

test("no visible text calls the app Handy alone", async ({ page }) => {
  await page.goto("/");
  const nav = page
    .getByRole("navigation", { name: "Sections" })
    .getByRole("button");
  await expect(nav.first()).toBeVisible();
  const names = await nav.allInnerTexts();
  for (const [i, name] of names.entries()) {
    await nav.getByText(name, { exact: true }).click();
    await page.waitForTimeout(150);
    const offenders = (await visibleText(page)).filter((t) => {
      const [label, text] = t.split("|");
      // Credit and donations name the original project on purpose.
      const credit = /Based on Handy|continue building Handy/.test(text);
      return BARE_HANDY.test(text) && label !== "Handy Flow" && !credit;
    });
    expect(offenders, `section ${i}`).toEqual([]);
  }
});
