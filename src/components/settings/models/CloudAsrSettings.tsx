import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cloud, Laptop } from "lucide-react";
import type { CloudAsrSettings as CloudSettings } from "@/bindings";
import { useSettings } from "@/hooks/useSettings";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { ProviderSelect } from "../PostProcessingSettingsApi/ProviderSelect";
import { ModelSelect } from "../PostProcessingSettingsApi/ModelSelect";
import { ApiKeyField } from "../PostProcessingSettingsApi/ApiKeyField";
import { BaseUrlField } from "../PostProcessingSettingsApi/BaseUrlField";
import { TranslateToEnglish } from "../TranslateToEnglish";
import { LANGUAGES, languageName } from "@/lib/constants/languages";
import { useSettingsStore } from "@/stores/settingsStore";

const MODELS: Record<string, string[]> = {
  groq: ["whisper-large-v3", "whisper-large-v3-turbo"],
  openai: ["whisper-1", "gpt-4o-transcribe", "gpt-4o-mini-transcribe"],
  custom: [],
};

const DEFAULTS: CloudSettings = {
  provider_id: "",
  models: { groq: "whisper-large-v3", openai: "whisper-1" },
  timeout_secs: 20,
};

export const CloudAsrSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const {
    getSetting,
    updateSetting,
    isUpdating,
    updatePostProcessApiKey,
    isLoading,
  } = useSettings();
  const cloud = getSetting("cloud_asr") ?? DEFAULTS;
  const keys = getSetting("post_process_api_keys") ?? {};
  const busy = isLoading || isUpdating("cloud_asr");
  const id = cloud.provider_id;
  const lastProvider = useRef(id || "groq");
  const [saveError, setSaveError] = useState(false);
  if (id) lastProvider.current = id;
  const provider = getSetting("post_process_providers")?.find(
    (p) => p.id === id,
  );
  const save = async (action: () => Promise<void>, verify: () => boolean) => {
    setSaveError(false);
    try {
      await action();
      setSaveError(!verify());
    } catch {
      setSaveError(true);
    }
  };
  const set = (patch: Partial<CloudSettings>) => {
    const next = { ...cloud, ...patch };
    return save(
      () => updateSetting("cloud_asr", next),
      () =>
        JSON.stringify(useSettingsStore.getState().settings?.cloud_asr) ===
        JSON.stringify(next),
    );
  };
  const k = (key: string) => `settings.transcription.${key}`;
  const model = cloud.models[id] ?? "";
  const options = (MODELS[id] ?? []).map((m) => ({ value: m, label: m }));
  if (model && !options.some((o) => o.value === model))
    options.push({ value: model, label: model });
  const setModel = (value: string) =>
    set({ models: { ...cloud.models, [id]: value.trim() } });

  return (
    <section className="space-y-5" aria-label={t(k("modeTitle"))}>
      <fieldset className="space-y-3" disabled={busy}>
        <legend className="text-base font-semibold">{t(k("modeTitle"))}</legend>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {(
            [
              {
                api: false,
                Icon: Laptop,
                title: "localMode",
                description: "localDescription",
              },
              {
                api: true,
                Icon: Cloud,
                title: "apiMode",
                description: "apiDescription",
              },
            ] as const
          ).map(({ api, Icon, title, description }) => (
            <label
              key={title}
              className={`flex cursor-pointer items-start gap-3 rounded-lg border p-4 transition-colors has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-logo-primary ${!!id === api ? "border-logo-primary bg-logo-primary/5" : "border-line hover:bg-wash"} ${busy ? "opacity-60" : ""}`}
            >
              <input
                type="radio"
                name="transcription-mode"
                checked={!!id === api}
                onChange={() =>
                  set({ provider_id: api ? lastProvider.current : "" })
                }
                className="mt-1 accent-logo-primary"
              />
              <span className="min-w-0">
                <span className="flex items-center gap-2 text-sm font-semibold">
                  <Icon className="size-4" aria-hidden="true" />
                  {t(k(title))}
                </span>
                <span className="mt-1 block text-sm text-ink-2">
                  {t(k(description))}
                </span>
              </span>
            </label>
          ))}
        </div>
      </fieldset>
      {id && (
        <div className="space-y-4 rounded-lg border border-line bg-background p-4">
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div className="min-w-0 space-y-2">
              <p className="text-sm font-medium">
                {t("settings.cloudAsr.provider.label")}
              </p>
              <ProviderSelect
                value={id}
                disabled={busy}
                onChange={(value) => set({ provider_id: value })}
                options={[
                  { value: "groq", label: "Groq" },
                  { value: "openai", label: "OpenAI" },
                  {
                    value: "custom",
                    label: t("settings.cloudAsr.provider.custom"),
                  },
                ]}
              />
            </div>
            <label className="block min-w-0 space-y-2">
              <span className="block text-sm font-medium">
                {t("settings.cloudAsr.model.label")}
              </span>
              <ModelSelect
                key={id}
                value={model}
                options={options}
                disabled={busy}
                onSelect={setModel}
                onCreate={setModel}
                onBlur={() => {}}
                className="w-full min-w-0"
              />
            </label>
          </div>
          <label className="block space-y-2">
            <span className="block text-sm font-medium">
              {t("settings.cloudAsr.apiKey.label")}
            </span>
            <ApiKeyField
              key={id}
              value={keys[id] ?? ""}
              disabled={isLoading || isUpdating(`post_process_api_key:${id}`)}
              onBlur={(value) =>
                save(
                  () => updatePostProcessApiKey(id, value),
                  () =>
                    useSettingsStore.getState().settings
                      ?.post_process_api_keys?.[id] === value,
                )
              }
              className="!min-w-0 w-full"
            />
          </label>
          <p className="text-xs text-ink-2">
            {t(k(id === "custom" ? "customKeyHelp" : "sharedKey"))}
          </p>
          <details className="border-t border-line pt-3">
            <summary className="cursor-pointer rounded text-sm font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary">
              {t(k("advanced"))}
            </summary>
            <div className="mt-4 space-y-4">
              <label className="block space-y-2">
                <span className="block text-sm font-medium">
                  {t(k("timeoutTitle"))}
                </span>
                <Input
                  type="number"
                  min={3}
                  max={120}
                  value={cloud.timeout_secs}
                  disabled={busy}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    if (Number.isInteger(value) && value >= 3 && value <= 120)
                      set({ timeout_secs: value });
                  }}
                  className="w-24"
                />
                <span className="block text-xs text-ink-2">
                  {t("settings.cloudAsr.timeout.description")}
                </span>
              </label>
              {id === "custom" && (
                <label className="block space-y-2">
                  <span className="block text-sm font-medium">
                    {t(k("serverUrl"))}
                  </span>
                  <BaseUrlField
                    key={id}
                    value={provider?.base_url ?? ""}
                    disabled={
                      isLoading || isUpdating(`post_process_base_url:${id}`)
                    }
                    onBlur={(value) =>
                      save(
                        () =>
                          useSettingsStore
                            .getState()
                            .updatePostProcessSetting("base_url", id, value),
                        () =>
                          useSettingsStore
                            .getState()
                            .settings?.post_process_providers?.find(
                              (p) => p.id === id,
                            )?.base_url === value,
                      )
                    }
                    className="!min-w-0 w-full"
                  />
                  <span className="block text-xs text-ink-2">
                    {t(k("sharedUrl"))}
                  </span>
                </label>
              )}
              <label className="block space-y-2">
                <span className="block text-sm font-medium">
                  {t(k("apiLanguage"))}
                </span>
                <Select
                  value={getSetting("selected_language") || "auto"}
                  options={[
                    { value: "auto", label: t(k("automaticLanguage")) },
                    ...LANGUAGES.map((language) => ({
                      value: language.code,
                      label: languageName(language.code, i18n.language),
                    })),
                  ]}
                  disabled={isUpdating("selected_language")}
                  onChange={(value) => {
                    if (value) void updateSetting("selected_language", value);
                  }}
                />
              </label>
              <TranslateToEnglish grouped />
              <p className="text-xs text-ink-2">{t(k("sharedLanguage"))}</p>
            </div>
          </details>
        </div>
      )}
      {saveError && (
        <p role="alert" className="text-sm text-error">
          {t(k("saveError"))}
        </p>
      )}
    </section>
  );
};
