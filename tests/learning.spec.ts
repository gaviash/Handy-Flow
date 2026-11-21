import { test, expect } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

const openLearning = async (page: import("@playwright/test").Page) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Post Process", exact: true }).click();
};

test("undo removes only what was announced", async ({ page }) => {
  const mock = await mockTauri(page);
  await page.goto("/");
  await mock.emit("learned-words", [
    { kind: "form", term: "Qwen", heard: "Quinn" },
    { kind: "rule", rule: "Pas de point final." },
  ]);
  await expect(
    page.getByText(/Learned: Qwen, Pas de point final\./).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Undo" }).click();
  await expect
    .poll(async () => (await mock.calls()).map((c) => c.cmd))
    .toEqual(
      expect.arrayContaining(["remove_heard_form", "delete_style_rule"]),
    );
  const calls = await mock.calls();
  expect(calls.map((c) => c.cmd)).not.toContain("delete_learned_word");
  expect(calls.find((c) => c.cmd === "remove_heard_form")?.args).toEqual({
    term: "Qwen",
    heard: "Quinn",
  });
});

test("style rules have their own Clear All", async ({ page }) => {
  const mock = await mockTauri(page, {
    results: {
      get_style_rules: [
        {
          rule: "Pas de point final.",
          count: 2,
          active: true,
          last_seen: 0,
          apps: [],
        },
      ],
    },
  });
  // Tauri's ask() dialog: the user confirms.
  await page.addInitScript(() => {
    const ipc = (window as any).__TAURI_INTERNALS__;
    const invoke = ipc.invoke;
    ipc.invoke = (cmd: string, args: unknown) =>
      cmd === "plugin:dialog|ask" ? Promise.resolve(true) : invoke(cmd, args);
  });
  await openLearning(page);
  // The second Clear All belongs to the style rules.
  const clear = page.getByRole("button", { name: "Clear All" }).nth(1);
  await clear.scrollIntoViewIfNeeded();
  await clear.click();
  await expect
    .poll(async () => (await mock.calls()).map((c) => c.cmd))
    .toContain("clear_style_rules");
  expect((await mock.calls()).map((c) => c.cmd)).not.toContain(
    "clear_learned_words",
  );
});

test("a failed export says so", async ({ page }) => {
  await mockTauri(page);
  await page.addInitScript(() => {
    const ipc = (window as any).__TAURI_INTERNALS__;
    const invoke = ipc.invoke;
    ipc.invoke = (cmd: string, args: unknown) =>
      cmd === "plugin:dialog|save"
        ? Promise.resolve("C:/nope/words.json")
        : cmd === "export_learned_words"
          ? Promise.reject("access denied")
          : invoke(cmd, args);
  });
  await openLearning(page);
  const exportButton = page.getByRole("button", { name: "Export" });
  await exportButton.scrollIntoViewIfNeeded();
  await exportButton.click();
  await expect(
    page.getByText("Couldn't export what Handy Flow learned.").first(),
  ).toBeVisible();
});

function status(
  problem: string | null = null,
  automatic = false,
  provider_label = "Groq",
) {
  return {
    feature: "LearningValidator",
    provider_label,
    model: "gpt-oss",
    automatic,
    problem,
    reasoning: "Medium",
    model_reasons: true,
  };
}

test("the validator shows the effective provider and model", async ({
  page,
}) => {
  await mockTauri(page, {
    results: { get_feature_statuses: [status(null, false, "OpenAI")] },
  });
  await openLearning(page);
  await expect(page.getByText(/Uses OpenAI · gpt-oss/)).toBeVisible();
});

test("without a usable model the validator is inactive", async ({ page }) => {
  await mockTauri(page, {
    results: { get_feature_statuses: [status("NoModel")] },
  });
  await openLearning(page);
  await expect(page.getByText(/Inactive · no usable model/)).toBeVisible();
});

test("the validator summary links to the model picker", async ({ page }) => {
  await mockTauri(page, { results: { get_feature_statuses: [status()] } });
  await openLearning(page);
  await page.getByRole("button", { name: "Change in Models" }).last().click();
  await expect(
    page.getByRole("group", { name: "Learning validator" }),
  ).toBeVisible();
});

test("the validator shows why context sharing prevents it from running", async ({
  page,
}) => {
  await mockTauri(page, {
    results: { get_feature_statuses: [status("ContextOff")] },
  });
  await openLearning(page);
  await expect(
    page.getByText(/Inactive · context sharing is off/),
  ).toBeVisible();
});

test("the automatic validator shows the effective model and reasoning level", async ({
  page,
}) => {
  await mockTauri(page, {
    results: { get_feature_statuses: [status(null, true)] },
  });
  await openLearning(page);
  await expect(
    page.getByText(/Automatic · Groq · gpt-oss · reasoning Medium/),
  ).toBeVisible();
});

test("the validator status is re-read after a setting is saved", async ({
  page,
}) => {
  const mock = await mockTauri(page, {
    results: { get_feature_statuses: [status()] },
  });
  await openLearning(page);
  await expect(page.getByText(/Uses Groq/)).toBeVisible();
  const reads = async () =>
    (await mock.calls()).filter((c) => c.cmd === "get_feature_statuses").length;
  const before = await reads();
  const sound = page.getByRole("checkbox", {
    name: "Sound When a Word Is Learned",
  });
  await sound.scrollIntoViewIfNeeded();
  await sound.click({ force: true });
  await expect.poll(reads).toBeGreaterThan(before);
});

test("the last 30 days of learning are counted", async ({ page }) => {
  await mockTauri(page, {
    results: {
      get_learning_stats: {
        observed: 12,
        learned: 5,
        pending: 2,
        rejected: 4,
        rejected_by: [
          ["grammar", 3],
          ["refused", 1],
        ],
      },
    },
  });
  await openLearning(page);
  await expect(
    page.getByText(
      "Last 30 days: 12 corrections observed · 5 words learned · 2 pending · 4 rejected",
    ),
  ).toBeVisible();
});
