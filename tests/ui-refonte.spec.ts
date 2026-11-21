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

test("Enhancement shows its model in a line that links to Models", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: { post_process_enabled: true },
    results: { get_feature_statuses: rewriteStatus(null) },
  });
  await openEnhancement(page);
  await expect(page.getByText("Uses Groq · llama")).toBeVisible();
  await page.getByRole("button", { name: "Change in Models" }).first().click();
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

test("an undo that comes after the deletion is no longer offered", async ({
  page,
}) => {
  await page.clock.install();
  const mock = await mockTauri(page, { results: historyWith("bonjour") });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByRole("button", { name: "Delete entry" }).click();
  // Hovering keeps the toast up past the 8 s.
  await page.getByRole("button", { name: "Undo" }).hover();
  await page.clock.runFor(10_000);
  await expect
    .poll(
      async () =>
        (await mock.calls()).filter((c) => c.cmd === "delete_history_entry")
          .length,
    )
    .toBe(1);
  await expect(page.getByRole("button", { name: "Undo" })).toHaveCount(0);
});

test("two deleted entries with the same text each keep their undo", async ({
  page,
}) => {
  const two = historyWith("bonjour");
  const [a] = two.get_history_entries.entries;
  two.get_history_entries.entries = [a, { ...a, id: 8, file_name: "h-8.wav" }];
  await page.clock.install();
  await mockTauri(page, { results: two });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByRole("button", { name: "Delete entry" }).first().click();
  await page.getByRole("button", { name: "Delete entry" }).first().click();
  await expect(page.getByRole("button", { name: "Undo" })).toHaveCount(2);
});

test("a failed save only rolls back its own setting", async ({ page }) => {
  await mockTauri(page, {
    settings: { audio_feedback: false, mute_while_recording: false },
    errors: { change_audio_feedback_setting: "disk full" },
    delays: { change_audio_feedback_setting: 600 },
  });
  await page.goto("/");
  await page.getByRole("button", { name: /^Sounds/ }).click();
  await page
    .getByRole("checkbox", { name: "Audio Feedback" })
    .click({ force: true });
  const mute = page.getByRole("checkbox", { name: "Mute While Recording" });
  await mute.click({ force: true });
  await expect(page.getByText("Setting not saved").first()).toBeVisible();
  await expect(mute).toBeChecked();
});

test("exclusion lists are named for screen readers", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Context", exact: true }).click();
  await page.getByRole("button", { name: /^Exclusions/ }).click();
  await expect(
    page.getByRole("textbox", { name: "Excluded Apps" }),
  ).toBeVisible();
});

test("a click on the active tab returns to its overview", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "Models", exact: true }).click();
  await page.getByRole("button", { name: /^Providers/ }).click();
  await page.getByRole("button", { name: "Models", exact: true }).click();
  await expect(page.getByRole("button", { name: /^Providers/ })).toBeVisible();
});

test("Advanced shows text insertion as a card and folds audio capture", async ({
  page,
}) => {
  await mockTauri(page, { settings: { experimental_enabled: false } });
  await page.goto("/");
  await page.getByRole("button", { name: "Advanced", exact: true }).click();
  await page.getByRole("button", { name: /^Text insertion/ }).click();
  await expect(
    page.getByRole("heading", { level: 1, name: "Text insertion" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Advanced" }).last().click();
  // Formerly behind Experimental: now reachable without it.
  await page.getByText("Expert settings").click();
  await expect(page.getByText("Silence detector").first()).toBeVisible();
});

test("History shows its retention, and the count only for 'keep the last'", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: { recording_retention_period: "days30" },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(page.getByText("30 Days (Audio of the Last 20)")).toBeVisible();
  await expect(page.getByText("History Limit")).toHaveCount(0);
});

test("About shows updates and diagnostics directly", async ({ page }) => {
  await mockTauri(page);
  await page.goto("/");
  await page.getByRole("button", { name: "About", exact: true }).click();
  await expect(
    page.getByRole("checkbox", { name: "Check for Updates" }),
  ).toBeAttached();
});

test("a setting's title names its field, and French says Amélioration", async ({
  page,
}) => {
  await mockTauri(page, { settings: { app_language: "fr" } });
  await page.goto("/");
  await expect(
    page.getByRole("button", { name: "Amélioration", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Vocabulaire", exact: true }).click();
  await expect(page.getByRole("group", { name: "Votre nom" })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Plus d'informations" }).first(),
  ).toBeVisible();
});

test("a pending deletion stays hidden when another one fails", async ({
  page,
}) => {
  const two = historyWith("bonjour");
  const [a] = two.get_history_entries.entries;
  two.get_history_entries.entries = [
    a,
    { ...a, id: 8, file_name: "h-8.wav", transcription_text: "salut" },
  ];
  await page.clock.install();
  await mockTauri(page, {
    results: two,
    errors: { delete_history_entry: "locked" },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByRole("button", { name: "Delete entry" }).first().click();
  await page.clock.runFor(4_000);
  await page.getByRole("button", { name: "Delete entry" }).first().click();
  // The first delete fails and reloads the list; the second still waits.
  await page.clock.runFor(5_000);
  await expect(page.getByRole("button", { name: "Delete entry" })).toHaveCount(
    1,
  );
});

test("Models warns when nothing can transcribe", async ({ page }) => {
  await mockTauri(page, { results: { get_current_model: "" } });
  await page.goto("/");
  await page.getByRole("button", { name: "Models", exact: true }).click();
  await expect(page.getByText("Needs setup")).toBeVisible();
  await expect(page.getByText("AI features ready.")).toHaveCount(0);
});

test("the name and history-limit fields carry their own names", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: { recording_retention_period: "preserve_limit" },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Vocabulary", exact: true }).click();
  await expect(page.getByRole("textbox", { name: "Your Name" })).toBeVisible();
  await page.getByRole("button", { name: "History", exact: true }).click();
  await expect(
    page.getByRole("spinbutton", { name: "History Limit" }),
  ).toBeVisible();
});

test("Transformations is its own tab of cards that open their editor", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: {
      transforms: [
        { name: "Plus court", instruction: "Make it shorter." },
        { name: "", instruction: "" },
        { name: "En anglais", instruction: "Translate it into English." },
      ],
    },
  });
  await page.goto("/");
  await page
    .getByRole("button", { name: "Transformations", exact: true })
    .click();
  await expect(page.getByRole("button", { name: /Polish/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Plus court/ })).toBeVisible();
  await page.getByRole("button", { name: /Create your own/ }).click();
  await expect(page.getByRole("textbox", { name: "Name" })).toHaveValue("");
});

test("Enhancement names its fallback line", async ({ page }) => {
  await mockTauri(page, {
    settings: { post_process_enabled: true },
    results: {
      get_feature_statuses: [
        ...rewriteStatus(null),
        {
          feature: "RewriteFallback",
          provider_label: "",
          model: "",
          automatic: false,
          problem: "NotChosen",
          reasoning: "Off",
          model_reasons: false,
        },
      ],
    },
  });
  await openEnhancement(page);
  await expect(page.getByText("Rewrite fallback: None")).toBeVisible();
});

test("'New transformation' creates a fourth one, and it can be deleted", async ({
  page,
}) => {
  const mock = await mockTauri(page);
  await page.goto("/");
  await page
    .getByRole("button", { name: "Transformations", exact: true })
    .click();
  await page.getByRole("button", { name: "New transformation" }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Résumer");
  await page
    .getByRole("textbox", { name: "Instruction" })
    .fill("Summarize it.");
  await page.getByRole("textbox", { name: "Name" }).focus();
  const saved = async () =>
    (await mock.calls())
      .filter((c) => c.cmd === "change_transforms_setting")
      .map((c) => (c.args as { transforms: { name: string }[] }).transforms)
      .pop();
  await expect.poll(saved).toHaveLength(4);
  expect((await saved())?.[3]).toMatchObject({ name: "Résumer" });
  await page
    .getByRole("button", { name: "Delete this transformation" })
    .click();
  await expect.poll(async () => (await saved())?.[3]?.name).toBe("");
  // Its shortcut is released too, so its keys stop being caught.
  await expect
    .poll(async () =>
      (await mock.calls()).find((c) => c.cmd === "reset_binding"),
    )
    .toMatchObject({ args: { id: "transform_4" } });
});

test("a transformation with only an instruction keeps its card", async ({
  page,
}) => {
  await mockTauri(page, {
    settings: {
      transforms: [
        { name: "", instruction: "Make it shorter." },
        { name: "Plus formel", instruction: "Make it more formal." },
        { name: "En anglais", instruction: "Translate it into English." },
      ],
    },
  });
  await page.goto("/");
  await page
    .getByRole("button", { name: "Transformations", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: /Make it shorter/ }),
  ).toBeVisible();
});
