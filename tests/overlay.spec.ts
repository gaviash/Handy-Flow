import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";
import { DEFAULT_APPEARANCE } from "../src/lib/utils/appearance";

test("recording Pill has a label and Wave keeps Cancel inside the pill", async ({
  page,
}) => {
  const backend = await mockTauri(page, { label: "recording_overlay" });
  await page.goto("/src/overlay/index.html");
  await expect
    .poll(async () =>
      (await backend.calls()).some((c) => c.cmd === "get_app_settings"),
    )
    .toBe(true);
  await backend.emit("show-overlay", "recording");
  await expect(page.locator(".ov-stage")).toHaveClass(/show/);
  await backend.emit("appearance-changed", {
    ...DEFAULT_APPEARANCE,
    overlay_show_label: false,
  });
  const card = await page.locator(".scard").boundingBox();
  const cancel = await page
    .getByRole("button", { name: "Cancel" })
    .boundingBox();
  const dot = await page.locator(".sdot").boundingBox();
  expect(
    Math.abs(cancel!.y + cancel!.height / 2 - (dot!.y + dot!.height / 2)),
  ).toBeLessThan(1);
  expect(cancel!.y + cancel!.height).toBeLessThanOrEqual(
    card!.y + card!.height,
  );
  await backend.emit("appearance-changed", DEFAULT_APPEARANCE);
  await expect(page.locator(".slisten-label")).toBeVisible();
  await expect(page.locator(".swave i")).toHaveCount(9);
});

test("recolors live and hides only the label in Wave", async ({ page }) => {
  const backend = await mockTauri(page, { label: "recording_overlay" });
  await page.goto("/src/overlay/index.html");
  await expect
    .poll(async () =>
      (await backend.calls()).some((c) => c.cmd === "get_app_settings"),
    )
    .toBe(true);
  await backend.emit("show-overlay", "transcribing");
  await expect(page.locator(".swork-label")).toBeVisible();
  await backend.emit("appearance-changed", {
    ...DEFAULT_APPEARANCE,
    palette: "lime",
    overlay_show_label: false,
  });
  await expect(page.locator("html")).toHaveAttribute("data-palette", "lime");
  await expect(page.locator(".scard")).toHaveCSS(
    "background-color",
    "rgb(20, 22, 17)",
  );
  await expect(page.locator(".swork-label")).toBeHidden();
  await expect(page.locator(".scard")).toHaveAttribute(
    "aria-label",
    "Transcribing...",
  );
  await expect(page.getByRole("button", { name: "Cancel" })).toBeVisible();
});

for (const palette of ["rose", "sauge", "contraste"])
  for (const state of ["recording", "transcribing", "processing"]) {
    test(`${state} in ${palette}`, async ({ page }) => {
      const backend = await mockTauri(page, {
        label: "recording_overlay",
        settings: { palette, theme: "light" },
      });
      await page.setViewportSize({ width: 420, height: 140 });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.goto("/src/overlay/index.html");
      await expect
        .poll(async () =>
          (await backend.calls()).some((c) => c.cmd === "get_app_settings"),
        )
        .toBe(true);
      await backend.emit("show-overlay", state);
      await expect(page.locator(".ov-stage")).toHaveClass(/show/);
      if (state === "recording") {
        await backend.emit("recording-ready", null);
        await expect(page.locator(".sdot")).toHaveClass(/ready/);
      }
      await expect(page).toHaveScreenshot(`overlay-${state}-${palette}.png`);
    });
  }
