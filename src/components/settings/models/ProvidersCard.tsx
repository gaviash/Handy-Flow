import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type ModelEntry,
  type PostProcessProvider,
} from "@/bindings";
import { SettingsGroup } from "@/components/ui";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { useSettings } from "@/hooks/useSettings";
import { notify } from "@/stores/noticeStore";
import { ApiKeyField } from "../PostProcessingSettingsApi/ApiKeyField";
import { BaseUrlField } from "../PostProcessingSettingsApi/BaseUrlField";
import { ModelSelect } from "../PostProcessingSettingsApi/ModelSelect";
import { featureSelectClass } from "./FeatureModelsCard";

export function ProvidersCard() {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const [selected, setSelected] = useState<string>();
  const [adding, setAdding] = useState(false);
  const [label, setLabel] = useState("");
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<
    Record<string, "ok" | "refused" | undefined>
  >({});

  const edit = async (action: () => Promise<void>) => {
    setBusy(true);
    try {
      await action();
    } catch {
      notify({ level: "error", title: t("settings.providers.saveError") });
    } finally {
      setBusy(false);
    }
  };

  return (
    <SettingsGroup title={t("settings.providers.title")}>
      {settings?.post_process_providers?.map((provider) => {
        const keyed = !!settings.post_process_api_keys?.[provider.id];
        const models = settings.provider_models?.[provider.id] ?? [];
        const open = selected === provider.id;
        const optional =
          provider.id === "custom" ||
          provider.id.startsWith("custom-") ||
          provider.id === "apple_intelligence";
        const state =
          results[provider.id] === "refused"
            ? "refused"
            : keyed
              ? "keyed"
              : optional
                ? "optional"
                : "missingKey";
        return (
          <div key={provider.id}>
            <button
              type="button"
              aria-expanded={open}
              disabled={busy}
              className="flex w-full items-center justify-between gap-2 px-4 py-3 text-left text-sm hover:bg-wash focus-visible:outline-2 focus-visible:outline-logo-primary"
              onClick={() => setSelected(open ? undefined : provider.id)}
            >
              <span className="font-semibold">{provider.label}</span>
              <span className="text-right text-xs text-ink-2">
                {t(`settings.providers.${state}`)} ·{" "}
                {t("settings.providers.modelCount", { count: models.length })}
              </span>
            </button>
            {open && (
              <ProviderEditor
                provider={provider}
                models={models}
                keyed={keyed}
                busy={busy}
                edit={edit}
                refreshSettings={refreshSettings}
                onTest={(result) =>
                  setResults((current) => ({
                    ...current,
                    [provider.id]: result,
                  }))
                }
              />
            )}
          </div>
        );
      })}
      <div className="space-y-3 p-4">
        <Button
          type="button"
          variant="outline"
          size="sm"
          aria-expanded={adding}
          disabled={busy}
          onClick={() => setAdding(!adding)}
        >
          {t("settings.providers.addProvider")}
        </Button>
        {adding && (
          <form
            className="grid gap-2 sm:grid-cols-2"
            onSubmit={(event) => {
              event.preventDefault();
              void edit(async () => {
                const result = await commands.upsertProvider(null, label, url);
                if (result.status === "error") throw new Error(result.error);
                await refreshSettings();
                setSelected(result.data);
                setAdding(false);
                setLabel("");
                setUrl("");
              });
            }}
          >
            <label className="space-y-1 text-sm">
              <span>{t("settings.providers.name")}</span>
              <Input
                required
                value={label}
                onChange={(event) => setLabel(event.target.value)}
                className="block w-full"
              />
            </label>
            <label className="space-y-1 text-sm">
              <span>{t("settings.providers.url")}</span>
              <Input
                required
                type="url"
                value={url}
                onChange={(event) => setUrl(event.target.value)}
                className="block w-full"
              />
            </label>
            <Button
              type="submit"
              size="sm"
              className="justify-self-start"
              disabled={busy}
            >
              {t("settings.providers.add")}
            </Button>
          </form>
        )}
      </div>
    </SettingsGroup>
  );
}

function ProviderEditor({
  provider,
  models,
  keyed,
  busy,
  edit,
  refreshSettings,
  onTest,
}: {
  provider: PostProcessProvider;
  models: ModelEntry[];
  keyed: boolean;
  busy: boolean;
  edit: (action: () => Promise<void>) => Promise<void>;
  refreshSettings: () => Promise<void>;
  onTest: (result: "ok" | "refused" | undefined) => void;
}) {
  const { t } = useTranslation();
  const [available, setAvailable] = useState<ModelEntry[]>([]);
  const [testMessage, setTestMessage] = useState("");
  const [keyVersion, setKeyVersion] = useState(0);
  const id = useId();
  const apple = provider.id === "apple_intelligence";
  const persist = async (entries: ModelEntry[]) => {
    const result = await commands.setProviderModels(provider.id, entries);
    if (result.status === "error") {
      if (result.error === "ModelInUse") {
        notify({ level: "warning", title: t("settings.providers.modelInUse") });
        return;
      }
      throw new Error(result.error);
    }
    await refreshSettings();
  };
  const add = (modelId: string) => {
    if (!modelId || models.some((model) => model.id === modelId)) return;
    const entry = available.find((model) => model.id === modelId);
    // The backend infers defaults for manually added model IDs.
    void edit(() =>
      persist([...models, entry ?? { id: modelId, speech: false }]),
    );
  };
  const saveKey = (value: string) => {
    void edit(async () => {
      try {
        const result = await commands.changePostProcessApiKeySetting(
          provider.id,
          value,
        );
        if (result.status === "error") throw new Error(result.error);
        await refreshSettings();
        onTest(undefined);
        setTestMessage("");
      } finally {
        setKeyVersion((version) => version + 1);
      }
    });
  };
  return (
    <fieldset
      aria-label={provider.label}
      className="space-y-4 border-t border-line bg-wash/40 px-4 py-4"
      disabled={busy}
    >
      {!apple && (
        <div className="grid gap-3 md:grid-cols-2">
          <div className="min-w-0 space-y-1 text-sm">
            <label htmlFor={`${id}-key`}>{t("settings.providers.key")}</label>
            <ApiKeyField
              id={`${id}-key`}
              key={keyVersion}
              value=""
              disabled={busy}
              placeholder={t(
                `settings.providers.${keyed ? "replaceKey" : "enterKey"}`,
              )}
              onBlur={(value) => {
                if (value.trim()) saveKey(value);
              }}
              className="!min-w-0 w-full"
            />
            {keyed && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => saveKey("")}
              >
                {t("settings.providers.clearKey")}
              </Button>
            )}
          </div>
          {provider.allow_base_url_edit && (
            <div className="min-w-0 space-y-1 text-sm">
              <label htmlFor={`${id}-url`}>{t("settings.providers.url")}</label>
              <BaseUrlField
                id={`${id}-url`}
                value={provider.base_url}
                disabled={busy}
                className="!min-w-0 w-full"
                onBlur={(value) => {
                  if (value === provider.base_url) return;
                  void edit(async () => {
                    const result = await commands.upsertProvider(
                      provider.id,
                      provider.label,
                      value,
                    );
                    if (result.status === "error")
                      throw new Error(result.error);
                    await refreshSettings();
                    setAvailable([]);
                    onTest(undefined);
                    setTestMessage("");
                  });
                }}
              />
            </div>
          )}
          {provider.id === "custom" && (
            <label className="min-w-0 space-y-1 text-sm">
              <span>{t("settings.postProcessing.api.apiFormat.title")}</span>
              <select
                className={featureSelectClass}
                value={provider.api_format ?? "openai"}
                onChange={(event) => {
                  const format =
                    event.target.value === "anthropic" ? "anthropic" : "openai";
                  void edit(async () => {
                    const result =
                      await commands.changePostProcessApiFormatSetting(
                        provider.id,
                        format,
                      );
                    if (result.status === "error") {
                      if (result.error !== "FormatInUse")
                        throw new Error(result.error);
                      notify({
                        level: "warning",
                        title: t("settings.providers.formatInUse"),
                      });
                      return;
                    }
                    await refreshSettings();
                    onTest(undefined);
                    setTestMessage("");
                  });
                }}
              >
                <option value="openai">
                  {t("settings.postProcessing.api.apiFormat.options.openai")}
                </option>
                <option value="anthropic">
                  {t("settings.postProcessing.api.apiFormat.options.anthropic")}
                </option>
              </select>
            </label>
          )}
        </div>
      )}
      <div className="flex flex-wrap gap-2">
        {!apple && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() =>
              void edit(async () => {
                try {
                  const result = await commands.testProvider(provider.id);
                  if (result.status === "error") {
                    onTest(
                      /^HTTP (401|403)$/.test(result.error)
                        ? "refused"
                        : undefined,
                    );
                    setTestMessage(
                      t(
                        `settings.providers.${result.error === "NoKey" ? "missingKey" : "testError"}`,
                      ),
                    );
                  } else {
                    onTest("ok");
                    setTestMessage(t("settings.providers.testOk"));
                  }
                } catch {
                  setTestMessage(t("settings.providers.testError"));
                }
              })
            }
          >
            {t("settings.providers.test")}
          </Button>
        )}
        {!apple && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() =>
              void edit(async () => {
                const result = await commands.refreshProviderModels(
                  provider.id,
                );
                if (result.status === "error") {
                  setTestMessage(
                    t(
                      `settings.providers.${result.error === "NoKey" ? "missingKey" : "refreshError"}`,
                    ),
                  );
                  return;
                }
                setAvailable(result.data);
                setTestMessage(
                  t("settings.providers.available", {
                    count: result.data.length,
                  }),
                );
              })
            }
          >
            {t("settings.providers.refresh")}
          </Button>
        )}
        {(provider.id === "custom" || provider.id.startsWith("custom-")) && (
          <Button
            type="button"
            variant="danger-ghost"
            size="sm"
            onClick={() =>
              void edit(async () => {
                const result = await commands.removeCustomProvider(provider.id);
                if (result.status === "error") {
                  if (!result.error.length) throw new Error();
                  notify({
                    level: "warning",
                    title: t("settings.providers.providerInUse", {
                      features: result.error
                        .map((feature) =>
                          t(`settings.features.name.${feature}`),
                        )
                        .join(", "),
                    }),
                  });
                } else await refreshSettings();
              })
            }
          >
            {t("settings.providers.removeProvider")}
          </Button>
        )}
      </div>
      {testMessage && (
        <p role="status" className="text-sm text-ink-2">
          {testMessage}
        </p>
      )}
      {!apple && (
        <ModelSelect
          value=""
          onSelect={add}
          onCreate={add}
          onBlur={() => {}}
          placeholder={t("settings.providers.addModel")}
          ariaLabel={t("settings.providers.addModel")}
          disabled={busy}
          options={available
            .filter((entry) => !models.some((model) => model.id === entry.id))
            .map((model) => ({ value: model.id, label: model.id }))}
          className="min-w-0"
        />
      )}
      <ul className="divide-y divide-line">
        {models.map((model) => (
          <li
            key={model.id}
            className="flex flex-wrap items-center gap-x-3 gap-y-1 py-2 text-sm"
          >
            <span className="min-w-0 flex-1 break-all">{model.id}</span>
            {!apple && (
              <>
                <label className="flex items-center gap-1">
                  <input
                    type="checkbox"
                    checked={model.speech}
                    className="accent-logo-primary"
                    onChange={(event) => {
                      const speech = event.target.checked;
                      void edit(() =>
                        persist(
                          models.map((entry) =>
                            entry.id === model.id
                              ? { ...entry, speech }
                              : entry,
                          ),
                        ),
                      );
                    }}
                  />
                  {t("settings.providers.speech")}
                </label>
                <label className="flex items-center gap-1">
                  <input
                    type="checkbox"
                    checked={!!model.reasoning}
                    disabled={model.speech}
                    className="accent-logo-primary"
                    onChange={(event) => {
                      const reasoning = event.target.checked;
                      void edit(() =>
                        persist(
                          models.map((entry) =>
                            entry.id === model.id
                              ? { ...entry, reasoning }
                              : entry,
                          ),
                        ),
                      );
                    }}
                  />
                  {t("settings.features.reasoning")}
                </label>
                <Button
                  type="button"
                  size="sm"
                  variant="danger-ghost"
                  aria-label={t("settings.providers.removeModelNamed", {
                    model: model.id,
                  })}
                  onClick={() =>
                    void edit(async () => {
                      const result = await commands.removeProviderModel(
                        provider.id,
                        model.id,
                      );
                      if (result.status === "error")
                        throw new Error(result.error);
                      await refreshSettings();
                      if (result.data.length)
                        notify({
                          level: "warning",
                          title: t("settings.providers.resetFeatures", {
                            features: result.data
                              .map((feature) =>
                                t(`settings.features.name.${feature}`),
                              )
                              .join(", "),
                          }),
                        });
                    })
                  }
                >
                  {t("settings.providers.removeModel")}
                </Button>
              </>
            )}
          </li>
        ))}
      </ul>
    </fieldset>
  );
}
