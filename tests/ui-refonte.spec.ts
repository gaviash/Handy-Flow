import { expect, test } from "@playwright/test";
import { mockTauri } from "./fixtures/tauri-mock";

test("a failed save rolls back and says so", async ({ page }) => {
  await mockTauri(page, {
    settings: { audio_feedback: false },
    errors: { change_audio_feedback_setting: "disk full" },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^Sounds/ }).click();
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

test("the Enhancement tab stays visible with the rewrite off", async ({
  page,
}) => {
  await mockTauri(page, { settings: { post_process_enabled: false } });
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Enhancement", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Debug", exact: true }),
  ).toHaveCount(0);
});

test("Vocabulary opens personal words, adds one, and goes back to its card", async ({
  page,
}) => {
  const mock = await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Vocabulary", exact: true }).click();
  await page.getByRole("button", { name: /^Personal words/ }).click();
  await page.getByPlaceholder("Add a word").fill("Qwen");
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await expect
    .poll(async () => (await mock.calls()).map((c) => c.cmd))
    .toContain("update_custom_words");
  await page.getByRole("button", { name: "Vocabulary" }).last().click();
  await expect(
    page.getByRole("button", { name: /^Personal words/ }),
  ).toBeFocused();
});

test("only a pending learned word shows when it expires", async ({ page }) => {
  const old = Math.floor(Date.now() / 1000) - 40 * 86_400;
  const word = (term: string, active: boolean) => ({
    term,
    heard: ["x"],
    count: 1,
    active,
    last_seen: old,
    source: "Correction",
  });
  await mockTauri(page, {
    results: {
      get_learned_words: [word("Qwen", true), word("Kiro", false)],
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Vocabulary", exact: true }).click();
  await page.getByRole("button", { name: /^Learned words/i }).click();
  await expect(page.locator("li", { hasText: "Qwen" })).not.toHaveAttribute(
    "title",
    /.+/,
  );
  await expect(page.locator("li", { hasText: "Kiro" })).toHaveAttribute(
    "title",
    /.+/,
  );
});

test("turning sending off changes what the Context page says", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: { context_enabled: true, context_send_enabled: true },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Context", exact: true }).click();
  await expect(
    page.getByText(/The text around the cursor is sent to the model/),
  ).toBeVisible();
  await page
    .getByRole("checkbox", { name: "Send Context to the LLM" })
    .click({ force: true });
  await expect(
    page.getByText("Read on your PC, nothing is sent."),
  ).toBeVisible();
});

const rewriteStatus = (problem: string | null) => [
  {
    feature: "Rewrite",
    provider_label: "Groq",
    model: "llama",
    automatic: false,
    problem,
    reasoning: "Off",
    model_reasons: false,
  },
];
const openEnhancement = async (page: import("@playwright/test").Page) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Enhancement", exact: true }).click();
};

test("Enhancement says when the rewrite is off", async ({ page }) => {
  await mockTauri(page, { settings: { post_process_enabled: false } });
  await openEnhancement(page);
  await expect(
    page.getByText("Off: the dictation is pasted as is."),
  ).toBeVisible();
});

test("Enhancement asks for setup when the rewrite has no key", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: { post_process_enabled: true },
    results: { get_feature_statuses: rewriteStatus("NoKey") },
  });
  await openEnhancement(page);
  await expect(page.getByText("Needs setup")).toBeVisible();
  await page.getByRole("button", { name: "Open Models" }).click();
  await expect(
    page.getByRole("group", { name: "Learning validator" }),
  ).toBeVisible();
});

test("a tone is chosen with pills, and back returns to the Tone card", async ({
  page,
}) => {
  const mock = await mockTauri(page, {
    settings: { post_process_enabled: true },
    results: { get_feature_statuses: rewriteStatus(null) },
  });
  await openEnhancement(page);
  await expect(page.getByText(/rewrites with Groq/)).toBeVisible();
  await page.getByRole("button", { name: /^Styles/ }).click();
  await page
    .getByRole("radiogroup", { name: "Work Messaging" })
    .getByRole("radio", { name: "Formal" })
    .click();
  await expect
    .poll(async () =>
      (await mock.calls()).find((c) => c.cmd === "change_styles_setting"),
    )
    .toMatchObject({ args: { styles: { work: "formal" } } });
  await page.getByRole("button", { name: "Enhancement" }).last().click();
  await expect(page.getByRole("button", { name: /^Styles/ })).toBeFocused();
});

test("the provider sub-page only summarises and links to Models", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: { post_process_enabled: true },
    results: { get_feature_statuses: rewriteStatus(null) },
  });
  await openEnhancement(page);
  await page.getByRole("button", { name: /^Provider/ }).click();
  await expect(page.getByText("Rewrite: Groq · llama")).toBeVisible();
  await expect(
    page.locator("main input, main textarea, main select"),
  ).toHaveCount(0);
  await expect(
    page.locator("input:not([type=checkbox]), textarea, select"),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Change in Models" }).click();
  await expect(
    page.getByRole("group", { name: "Learning validator" }),
  ).toBeVisible();
});

test("General names the enhanced shortcut only when the rewrite is on", async ({
  page,
}) => {
  await mockTauri(page, { settings: { post_process_enabled: false } });
  await page.goto("/");
  await expect(page.getByText(/: dictation as is/)).toBeVisible();
  await expect(page.getByText(/enhanced dictation/)).toHaveCount(0);
});
