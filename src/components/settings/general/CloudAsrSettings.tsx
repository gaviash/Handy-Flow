import React from "react";
import { useTranslation } from "react-i18next";
import type { CloudAsrSettings as Cloud } from "@/bindings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Input } from "../../ui/Input";
import { ProviderSelect } from "../PostProcessingSettingsApi/ProviderSelect";
import { ModelSelect } from "../PostProcessingSettingsApi/ModelSelect";
import { ApiKeyField } from "../PostProcessingSettingsApi/ApiKeyField";
import { useSettings } from "../../../hooks/useSettings";

/** Suggested models per provider; any other name can be typed. */
const MODELS: Record<string, string[]> = {
  groq: ["whisper-large-v3", "whisper-large-v3-turbo"],
  openai: ["whisper-1", "gpt-4o-transcribe", "gpt-4o-mini-transcribe"],
  custom: [],
};

const DEFAULTS: Cloud = {
  provider_id: "",
  models: { groq: "whisper-large-v3", openai: "whisper-1" },
  timeout_secs: 20,
};

/** Handy Adaptive: transcription by an API, the local model as fallback. */
export const CloudAsrSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating, updatePostProcessApiKey } =
    useSettings();
  const cloud = getSetting("cloud_asr") ?? DEFAULTS;
  const keys = getSetting("post_process_api_keys") ?? {};
  const busy = isUpdating("cloud_asr");
  const id = cloud.provider_id;
  const set = (patch: Partial<Cloud>) =>
    updateSetting("cloud_asr", { ...cloud, ...patch });
  const k = (key: string) => `settings.cloudAsr.${key}`;
  const model = cloud.models[id] ?? "";
  const options = (MODELS[id] ?? []).map((m) => ({ value: m, label: m }));
  if (model && !options.some((o) => o.value === model)) {
    options.push({ value: model, label: model });
  }
  const setModel = (m: string) =>
    set({ models: { ...cloud.models, [id]: m.trim() } });

  return (
    <SettingsGroup title={t(k("title"))}>
      <SettingContainer
        title={t(k("provider.label"))}
        description={t(k("provider.description"))}
        descriptionMode="tooltip"
        layout="horizontal"
        grouped={true}
      >
        <ProviderSelect
          value={id}
          disabled={busy}
          onChange={(v) => set({ provider_id: v })}
          options={[
            { value: "", label: t(k("provider.local")) },
            { value: "groq", label: "Groq" },
            { value: "openai", label: "OpenAI" },
            { value: "custom", label: t(k("provider.custom")) },
          ]}
        />
      </SettingContainer>
      {id && (
        <>
          <SettingContainer
            title={t(k("model.label"))}
            description={t(k("model.description"))}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <ModelSelect
              value={model}
              options={options}
              disabled={busy}
              onSelect={setModel}
              onCreate={setModel}
              onBlur={() => {}}
            />
          </SettingContainer>
          <SettingContainer
            title={t(k("apiKey.label"))}
            description={t(k("apiKey.description"))}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <ApiKeyField
              value={keys[id] ?? ""}
              disabled={isUpdating(`post_process_api_key:${id}`)}
              onBlur={(v) => updatePostProcessApiKey(id, v)}
            />
          </SettingContainer>
          <SettingContainer
            title={t(k("timeout.label"))}
            description={t(k("timeout.description"))}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <Input
              type="number"
              min={3}
              max={120}
              value={cloud.timeout_secs}
              disabled={busy}
              onChange={(e) => {
                const v = Number(e.target.value);
                if (Number.isInteger(v) && v >= 3 && v <= 120)
                  set({ timeout_secs: v });
              }}
              className="w-20"
            />
          </SettingContainer>
        </>
      )}
    </SettingsGroup>
  );
};
