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
