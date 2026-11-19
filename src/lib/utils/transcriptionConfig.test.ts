import assert from "node:assert/strict";
import type { AppSettings, FeatureModel } from "@/bindings";
import { getTranscriptionConfig } from "./transcriptionConfig";

const empty = { provider_id: "", model: "" };
const settings: AppSettings = {
  features: {
    rewrite: empty,
    rewrite_fallback: empty,
    learning_validator: empty,
    transcription_api: { provider_id: "groq", model: "whisper-large-v3" },
  },
  provider_models: {
    groq: [
      { id: "whisper-large-v3", speech: true },
      { id: "llama", speech: false },
    ],
    custom: [{ id: "local-whisper", speech: true }],
  },
  post_process_api_keys: { groq: "configured" },
  post_process_providers: [
    { id: "groq", label: "Groq", base_url: "https://api.groq.com" },
    { id: "custom", label: "Custom", base_url: "http://localhost" },
  ],
};
const models = [{ id: "local", name: "Qwen", is_downloaded: true }];
const config = (patch: Partial<AppSettings> = {}) =>
  getTranscriptionConfig({ ...settings, ...patch }, models, "local");
const forChoice = (choice: FeatureModel, patch: Partial<AppSettings> = {}) =>
  config({
    ...patch,
    features: { ...settings.features!, transcription_api: choice },
  });

assert.equal(config().issue, null);
assert.equal(config().localModel?.name, "Qwen");
assert.equal(config().api, true);
assert.equal(forChoice(empty).api, false);
assert.equal(config({ post_process_api_keys: {} }).issue, "missingKey");
assert.equal(
  forChoice({ provider_id: "groq", model: "llama" }).issue,
  "missingModel",
);
assert.equal(
  forChoice(
    { provider_id: "custom", model: "local-whisper" },
    { post_process_api_keys: {} },
  ).issue,
  null,
);
assert.equal(config({ post_process_providers: [] }).issue, "missingProvider");
assert.equal(
  getTranscriptionConfig(settings, [], "local").localModel,
  undefined,
);
assert.equal(
  getTranscriptionConfig(settings, models, "other").localModel,
  undefined,
);
assert.equal(getTranscriptionConfig(null, [], "").api, false);
