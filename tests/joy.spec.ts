import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

const DAY = 86_400;
const now = () => Math.floor(Date.now() / 1000);

test("the sidebar highlight glides to the chosen section", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  const pill = page.getByTestId("sidebar-pill");
  const top = () => pill.evaluate((el) => el.getBoundingClientRect().top);
  const general = await top();
  await page.getByRole("button", { name: "History", exact: true }).click();
  const history = await page
    .getByRole("button", { name: "History", exact: true })
    .evaluate((el) => el.getBoundingClientRect().top);
  await expect.poll(top).toBeCloseTo(history, 0);
  expect(history).toBeGreaterThan(general);
});

test("a fresh learned word glows, an old one fades", async ({ page }) => {
  await mockTauri(page, {
    results: {
      get_learned_words: [
        { term: "Kubernetes", heard: ["cube"], count: 1, last_seen: now() },
        {
          term: "Qwen",
          heard: ["queen"],
          count: 1,
          last_seen: now() - 27 * DAY,
        },
      ],
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Post Process", exact: true }).click();
  await expect(page.getByText("Kubernetes", { exact: true })).toHaveClass(
    /animate-learned/,
  );
  const qwen = page.getByText("Qwen", { exact: true });
  await expect(qwen).not.toHaveClass(/animate-learned/);
  const opacity = await qwen.evaluate((el) =>
    Number(getComputedStyle(el.parentElement!).opacity),
  );
  expect(opacity).toBeLessThan(1);
  await expect(page.locator("li", { has: qwen })).toHaveAttribute(
    "title",
    /Forgotten on/,
  );
});

test("the empty history greets with the mascot", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(page.locator(".animate-wave")).toBeVisible();
});

test("a snippet previews what saying it types", async ({ page }) => {
  await mockTauri(page, {
    settings: { snippets: [{ trigger: "signature", text: "Marie Dupont" }] },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Post Process", exact: true }).click();
  await page.getByRole("button", { name: "Preview", exact: true }).click();
  await expect(page.getByText("Say “signature” on its own")).toBeVisible();
  await expect(page.locator(".snippet-reveal")).toHaveText("Marie Dupont");
});

test("the style list shows an example of the tone", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Post Process", exact: true }).click();
  await expect(page.getByText("Example for Email · Formal")).toBeVisible();
  await expect(
    page.getByText(/I confirm our meeting tomorrow at 10 a\.m\./),
  ).toBeVisible();
});
