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
