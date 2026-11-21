import { expect, test } from "@playwright/test";
import { DEFAULT_SETTINGS, mockTauri } from "./fixtures/tauri-mock";

const empty = { provider_id: "", model: "", reasoning: "Off" };
const choice = (provider_id: string, model: string, reasoning = "Off") => ({
  provider_id,
  model,
  reasoning,
});
const providerSettings = {
  features: {
    rewrite: choice("openai", "gpt-4o-mini"),
    rewrite_fallback: empty,
    learning_validator: choice("groq", "openai/gpt-oss-120b", "Medium"),
    transcription_api: choice("groq", "whisper-large-v3"),
  },
  provider_models: {
    openai: [{ id: "gpt-4o-mini", speech: false }],
    groq: [
      { id: "llama-3.3-70b-versatile", speech: false },
      { id: "openai/gpt-oss-120b", speech: false, reasoning: true },
      { id: "whisper-large-v3", speech: true },
    ],
    custom: [{ id: "local-chat", speech: false }],
  },
  post_process_api_keys: { openai: "configured", groq: "configured" },
};

test("models differ per feature and selections persist on reload", async ({
  page,
}) => {
  const mock = await mockTauri(page, { settings: providerSettings });
  const open = async () => {
    await page.getByRole("button", { name: "Models", exact: true }).click();
    await page.getByRole("button", { name: /^Usage by feature/ }).click();
  };
  await page.goto("/");
  await open();
  const rewrite = page.getByRole("group", { name: "Rewrite", exact: true });
  const validator = page.getByRole("group", { name: "Learning validator" });
  const speech = page.getByRole("group", { name: "Speech API", exact: true });
  await expect(rewrite.getByLabel("Provider")).toHaveValue("openai");
  await expect(validator.getByLabel("Provider")).toHaveValue("groq");
  await expect(
    speech
      .getByRole("combobox", { name: "Model", exact: true })
      .locator("option"),
  ).toHaveText(["Choose a model", "whisper-large-v3"]);
  const validatorModel = validator.getByRole("combobox", {
    name: "Model",
    exact: true,
  });
  await expect(validatorModel.locator("option")).toHaveText([
    "Choose a model",
    "openai/gpt-oss-120b · reasoning",
    "llama-3.3-70b-versatile",
  ]);
  await validator
    .getByRole("combobox", { name: "Reasoning", exact: true })
    .selectOption("High");
  await expect
    .poll(async () =>
      (await mock.calls()).find((call) => call.cmd === "set_feature_model"),
    )
    .toMatchObject({
      args: {
        feature: "LearningValidator",
        choice: choice("groq", "openai/gpt-oss-120b", "High"),
      },
    });
  await expect(
    validator.getByRole("combobox", { name: "Reasoning", exact: true }),
  ).toHaveValue("High");
  await page.reload();
  await open();
  await expect(
    rewrite.getByRole("combobox", { name: "Model", exact: true }),
  ).toHaveValue("gpt-4o-mini");
  await expect(
    validator.getByRole("combobox", { name: "Reasoning", exact: true }),
  ).toHaveValue("High");
  await validatorModel.selectOption("llama-3.3-70b-versatile");
  await expect(
    validator.getByRole("combobox", { name: "Reasoning", exact: true }),
  ).toHaveCount(0);
});

test("a provider in use cannot be removed", async ({ page }) => {
  await mockTauri(page, {
    settings: {
      ...providerSettings,
      features: {
        ...providerSettings.features,
        rewrite: choice("custom", "local-chat"),
      },
      post_process_providers: DEFAULT_SETTINGS.post_process_providers,
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Models", exact: true }).click();
  await page.getByRole("button", { name: /^Providers/ }).click();
  await page.getByRole("button", { name: /^Custom/ }).click();
  await page.getByRole("button", { name: "Remove provider" }).click();
  await expect(
    page.getByText("Provider in use by Rewrite").first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Transcription models" }).click();
  await page.getByRole("button", { name: /^Usage by feature/ }).click();
  await expect(
    page
      .getByRole("group", { name: "Rewrite", exact: true })
      .getByLabel("Provider"),
  ).toHaveValue("custom");
});
