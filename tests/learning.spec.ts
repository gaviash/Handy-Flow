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

test("the judge shows which provider it uses", async ({ page }) => {
  await mockTauri(page, { results: { get_judge_provider: "OpenAI" } });
  await openLearning(page);
  await expect(page.getByText("Correction Judge")).toBeVisible();
  await expect(page.getByText("Uses OpenAI")).toBeVisible();
});

test("without a provider the judge says it is inactive", async ({ page }) => {
  await mockTauri(page, { results: { get_judge_provider: null } });
  await openLearning(page);
  await expect(page.getByText(/Inactive: no API key/)).toBeVisible();
});

test("the judge provider can be chosen", async ({ page }) => {
  const mock = await mockTauri(page, {
    results: {
      get_judge_provider: "Groq",
      get_judge_choices: [
        { id: "groq", label: "Groq" },
        { id: "openai", label: "OpenAI" },
      ],
    },
  });
  await openLearning(page);
  const auto = page.getByRole("button", { name: "Automatic", exact: true });
  await auto.scrollIntoViewIfNeeded();
  await auto.click();
  // The rewrite provider's dropdown also reads "OpenAI": pick the menu item.
  await page
    .locator("button:not([aria-expanded])", { hasText: /^OpenAI$/ })
    .click();
  await expect
    .poll(async () =>
      (await mock.calls()).find(
        (c) => c.cmd === "change_learning_judge_provider_setting",
      ),
    )
    .toMatchObject({ args: { providerId: "openai" } });
});

test("the status shows what is really used", async ({ page }) => {
  await mockTauri(page, {
    settings: { learning_judge_provider_id: "openai" },
    results: {
      get_judge_provider: "Groq",
      get_judge_choices: [{ id: "groq", label: "Groq" }],
    },
  });
  await openLearning(page);
  await expect(page.getByText("Judge Provider")).toBeVisible();
  await expect(page.getByText("Uses Groq")).toBeVisible();
});

test("an unusable choice shows Automatic", async ({ page }) => {
  await mockTauri(page, {
    settings: { learning_judge_provider_id: "openai" },
    results: {
      get_judge_provider: "Groq",
      get_judge_choices: [{ id: "groq", label: "Groq" }],
    },
  });
  await openLearning(page);
  await expect(
    page.getByRole("button", { name: "Automatic", exact: true }),
  ).toBeVisible();
});

test("the judge status is re-read after another setting is saved", async ({
  page,
}) => {
  const mock = await mockTauri(page, {
    results: { get_judge_provider: "Groq" },
  });
  await openLearning(page);
  await expect(page.getByText("Uses Groq")).toBeVisible();
  const reads = async () =>
    (await mock.calls()).filter((c) => c.cmd === "get_judge_provider").length;
  const before = await reads();
  // A key, model or provider change can switch the judge: any saved setting re-reads it.
  const sound = page.getByRole("checkbox", {
    name: "Sound When a Word Is Learned",
  });
  await sound.scrollIntoViewIfNeeded();
  await sound.click({ force: true });
  await expect.poll(reads).toBeGreaterThan(before);
});
