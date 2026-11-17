import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

const stats = (words_today: number) => ({
  dictations: 3,
  words: 900,
  top_apps: [],
  learned_words: 0,
  words_today,
});

test("General counts today's words and the time they saved", async ({
  page,
}) => {
  await mockTauri(page, { results: { get_dictation_stats: stats(1200) } });
  await page.goto("/");
  const counter = page.getByTestId("today-counter");
  // 1200 words: 30 min typing at 40 wpm, 8 min speaking at 150 wpm.
  await expect(counter).toContainText("1,200");
  await expect(counter).toContainText("22 min");
  await expect(counter).toContainText("Words dictated today");
});

test("the counter rolls up live when a dictation lands", async ({ page }) => {
  const mock = await mockTauri(page, {
    results: { get_dictation_stats: stats(10) },
  });
  await page.goto("/");
  const counter = page.getByTestId("today-counter");
  await expect(counter).toContainText("10");
  await page.evaluate((next) => {
    // Next stats call returns more words, as after a real dictation.
    const ipc = (window as any).__TAURI_INTERNALS__;
    const invoke = ipc.invoke;
    ipc.invoke = (cmd: string, args: unknown) =>
      cmd === "get_dictation_stats" ? Promise.resolve(next) : invoke(cmd, args);
  }, stats(60));
  await mock.emit("history-update-payload", { action: "added", entry: {} });
  await expect(counter).toContainText("60");
});

test("no dictation today shows a quiet line", async ({ page }) => {
  await mockTauri(page, { results: { get_dictation_stats: stats(0) } });
  await page.goto("/");
  await expect(page.getByText("Nothing dictated yet today.")).toBeVisible();
});
