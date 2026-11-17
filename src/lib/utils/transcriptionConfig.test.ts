import assert from "node:assert/strict";
import { getTranscriptionConfig } from "./transcriptionConfig";

const settings = {
  cloud_asr: {
    provider_id: "groq",
    models: { groq: "whisper-large-v3", custom: "local-whisper" },
    timeout_secs: 20,
  },
  post_process_api_keys: { groq: "test-key" },
  post_process_providers: [
    { id: "groq", label: "Groq" },
    { id: "custom", label: "Custom" },
  ],
};
const models = [{ id: "local", name: "Qwen", is_downloaded: true }];
const configured = getTranscriptionConfig(settings, models, "local");
assert.equal(configured.issue, null);
assert.equal(configured.localModel?.name, "Qwen");
assert.equal(configured.api, true);
assert.equal(
  getTranscriptionConfig(
    { ...settings, cloud_asr: { ...settings.cloud_asr, provider_id: "" } },
    models,
    "local",
  ).api,
  false,
);
assert.equal(
  getTranscriptionConfig(
    { ...settings, post_process_api_keys: {} },
    models,
    "local",
  ).issue,
  "missingKey",
);
assert.equal(
  getTranscriptionConfig(
    {
      ...settings,
      cloud_asr: { ...settings.cloud_asr, models: { groq: "  " } },
    },
    models,
    "local",
  ).issue,
  "missingModel",
);
assert.equal(
  getTranscriptionConfig(
    {
      ...settings,
      cloud_asr: { ...settings.cloud_asr, provider_id: "custom" },
      post_process_api_keys: {},
    },
    models,
    "local",
  ).issue,
  null,
);
assert.equal(
  getTranscriptionConfig(
    { ...settings, post_process_providers: [] },
    models,
    "local",
  ).issue,
  "missingProvider",
);
assert.equal(
  getTranscriptionConfig(
    settings,
    [{ ...models[0], is_downloaded: false }],
    "local",
  ).localModel,
  undefined,
);
assert.equal(
  getTranscriptionConfig(settings, [], "local").localModel,
  undefined,
);
assert.equal(
  getTranscriptionConfig(settings, models, "other").localModel,
  undefined,
);
assert.equal(getTranscriptionConfig(null, [], "").api, false);
assert.deepEqual(settings.cloud_asr, {
  provider_id: "groq",
  models: { groq: "whisper-large-v3", custom: "local-whisper" },
  timeout_secs: 20,
});
console.log("transcriptionConfig: all assertions passed");
