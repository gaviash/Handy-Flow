import { expect, test } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

test("a failed save rolls back and says so", async ({ page }) => {
  await mockTauri(page, {
    settings: { audio_feedback: false },
    errors: { change_audio_feedback_setting: "disk full" },
  });
  await page.goto("/");
  const toggle = page.getByRole("checkbox", { name: "Audio Feedback" });
  await toggle.click({ force: true });
  await expect(page.getByText("Setting not saved").first()).toBeVisible();
  await expect(toggle).not.toBeChecked();
});

test("a history that fails to load says so and can retry", async ({ page }) => {
  const mock = await mockTauri(page, {
    errors: { get_history_entries: "database locked" },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(page.getByText("Couldn't load the history.")).toBeVisible();
  const loads = async () =>
    (await mock.calls()).filter((c) => c.cmd === "get_history_entries").length;
  const before = await loads();
  await page.getByRole("button", { name: "Retry" }).click();
  await expect.poll(loads).toBeGreaterThan(before);
});

const historyWith = (text: string) => ({
  get_history_entries: {
    entries: [
      {
        id: 7,
        file_name: "handy-7.wav",
        timestamp: Math.floor(Date.now() / 1000),
        saved: false,
        title: "",
        transcription_text: text,
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
});

test("a deleted history entry can be undone", async ({ page }) => {
  await page.clock.install();
  const mock = await mockTauri(page, { results: historyWith("bonjour") });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByRole("button", { name: "Delete entry" }).click();
  const row = page.getByRole("button", { name: "Delete entry" });
  await expect(row).toHaveCount(0);
  await page.getByRole("button", { name: "Undo" }).click();
  await expect(row).toHaveCount(1);
  await page.clock.runFor(10_000);
  const deletes = (await mock.calls()).filter(
    (c) => c.cmd === "delete_history_entry",
  );
  expect(deletes).toHaveLength(0);
});

test("a deleted history entry is deleted once the undo time is over", async ({
  page,
}) => {
  await page.clock.install();
  const mock = await mockTauri(page, { results: historyWith("bonjour") });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByRole("button", { name: "Delete entry" }).click();
  const deletes = async () =>
    (await mock.calls()).filter((c) => c.cmd === "delete_history_entry").length;
  expect(await deletes()).toBe(0);
  await page.clock.runFor(10_000);
  await expect.poll(deletes).toBe(1);
});
