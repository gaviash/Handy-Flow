import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

test("About links to the fork and has no donate button", async ({ page }) => {
  const mock = await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "About", exact: true }).click();
  await expect(page.getByRole("button", { name: "Donate" })).toHaveCount(0);
  await page.getByRole("button", { name: "View on GitHub" }).click();
  await expect
    .poll(async () =>
      (await mock.calls())
        .filter((c) => c.cmd === "plugin:opener|open_url")
        .map((c) => (c.args as { url: string }).url),
    )
    .toEqual(["https://github.com/gaviash/Handy-Flow"]);
});

test("a newer release is offered and installed from the footer", async ({
  page,
}) => {
  const mock = await mockTauri(page, {
    results: { is_update_checks_locked: false, check_for_update: "0.2.2" },
  });
  // Keep install_update pending, as the real one is until the app restarts.
  await page.addInitScript(() => {
    const ipc = (window as any).__TAURI_INTERNALS__;
    const invoke = ipc.invoke;
    ipc.invoke = (cmd: string, args: unknown) =>
      cmd === "install_update"
        ? (invoke(cmd, args), new Promise(() => {}))
        : invoke(cmd, args);
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Update available" }).click();
  await mock.emit("update-download-progress", 42);
  await expect(page.getByText(/Downloading\.\.\.\s+42%/)).toBeVisible();
  expect((await mock.calls()).map((c) => c.cmd)).toContain("install_update");
});

test("an up-to-date app offers a manual check", async ({ page }) => {
  await mockTauri(page, {
    results: { is_update_checks_locked: false, check_for_update: null },
  });
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Check for updates" }),
  ).toBeVisible();
  await expect(page.getByText("Update available")).toHaveCount(0);
});
