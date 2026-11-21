import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type Feature,
  type FeatureModel,
  type FeatureModels,
  type FeatureStatus,
  type Reasoning,
} from "@/bindings";
import { SettingsGroup } from "@/components/ui";
import { Button } from "@/components/ui/Button";
import { useSettings } from "@/hooks/useSettings";
import { notify } from "@/stores/noticeStore";
import { useSettingsStore } from "@/stores/settingsStore";

const keys = {
  Rewrite: "rewrite",
  RewriteFallback: "rewrite_fallback",
  LearningValidator: "learning_validator",
  TranscriptionApi: "transcription_api",
} as const satisfies Record<Feature, keyof FeatureModels>;

const emptyChoice: FeatureModel = {
  provider_id: "",
  model: "",
  reasoning: "Off",
};
const reasoningLevels: Reasoning[] = ["Off", "Low", "Medium", "High"];
export const featureSelectClass =
  "w-full min-w-0 rounded-md border border-line-strong bg-wash px-2 py-1.5 text-sm text-text focus-visible:outline-2 focus-visible:outline-logo-primary disabled:opacity-60";

function useFeatureStatuses() {
  const settings = useSettingsStore((state) => state.settings);
  const saving = useSettingsStore((state) =>
    Object.values(state.isUpdating).some(Boolean),
  );
  const [statuses, setStatuses] = useState<FeatureStatus[]>([]);
  useEffect(() => {
    if (!settings || saving) return;
    let current = true;
    void commands.getFeatureStatuses().then(
      (result) => {
        if (current) setStatuses(result);
      },
      () => {
        if (current) setStatuses([]);
      },
    );
    return () => {
      current = false;
    };
  }, [settings, saving]);
  return statuses;
}

function FeatureStatusLine({ status }: { status?: FeatureStatus }) {
  const { t } = useTranslation();
  if (!status) return null;
  const problem = status.problem;
  const description =
    problem === "NotChosen"
      ? t(`settings.features.empty.${status.feature}`)
      : problem
        ? t(`settings.features.problem.${problem}`)
        : t(`settings.features.${status.automatic ? "automaticUse" : "uses"}`, {
            provider: status.provider_label,
            model: status.model,
          });
  return (
    <p role="status" className="text-sm text-ink-2">
      {description}
      {!problem && status.model_reasons && (
        <>
          {" "}
          ·{" "}
          {t("settings.features.reasoningStatus", {
            level: t(`settings.features.level.${status.reasoning}`),
          })}
        </>
      )}
    </p>
  );
}

export function FeatureSummary({
  feature,
  onShowModels,
}: {
  feature: Feature;
  onShowModels?: () => void;
}) {
  const { t } = useTranslation();
  const status = useFeatureStatuses().find((item) => item.feature === feature);
  return (
    <div className="row-setting flex flex-wrap items-center justify-between gap-2 px-4 py-3">
      <FeatureStatusLine status={status} />
      {onShowModels && (
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={onShowModels}
        >
          {t("settings.features.changeInModels")}
        </Button>
      )}
    </div>
  );
}

export function FeatureModelsCard() {
  const { t } = useTranslation();
  const { settings, refreshSettings } = useSettings();
  const statuses = useFeatureStatuses();
  const [pending, setPending] = useState(false);
  const [appleUnavailable, setAppleUnavailable] = useState(false);
  const appleSelected =
    settings?.features?.rewrite.provider_id === "apple_intelligence";
  useEffect(() => {
    if (!appleSelected) return;
    let current = true;
    void commands.checkAppleIntelligenceAvailable().then(
      (available) => {
        if (current) setAppleUnavailable(!available);
      },
      () => {
        if (current) setAppleUnavailable(true);
      },
    );
    return () => {
      current = false;
    };
  }, [appleSelected]);
  const [drafts, setDrafts] = useState<Partial<Record<Feature, string>>>({});
  const save = async (feature: Feature, choice: FeatureModel) => {
    setPending(true);
    try {
      const result = await commands.setFeatureModel(feature, choice);
      if (result.status === "error") throw new Error(result.error);
      await refreshSettings();
      setDrafts((current) => ({ ...current, [feature]: undefined }));
    } catch {
      notify({ level: "error", title: t("settings.features.saveError") });
    } finally {
      setPending(false);
    }
  };

  return (
    <SettingsGroup title={t("settings.features.title")}>
      {(Object.keys(keys) as Feature[]).map((feature) => {
        const name = t(`settings.features.name.${feature}`);
        const choice = settings?.features?.[keys[feature]] ?? emptyChoice;
        const providerId = drafts[feature] ?? choice.provider_id;
        const validator = feature === "LearningValidator";
        const speech = feature === "TranscriptionApi";
        const providers = (settings?.post_process_providers ?? []).filter(
          (provider) =>
            feature === "Rewrite" ||
            (provider.id !== "apple_intelligence" &&
              (!(speech || validator) || provider.api_format !== "anthropic")),
        );
        const models = (settings?.provider_models?.[providerId] ?? [])
          .filter((model) => model.speech === speech)
          .slice()
          .sort((a, b) => a.id.localeCompare(b.id));
        const selectedModel =
          providerId === choice.provider_id ? choice.model : "";
        const selected = models.find((model) => model.id === selectedModel);
        const options = (entries: typeof models) =>
          entries.map((model) => (
            <option key={model.id} value={model.id}>
              {model.reasoning
                ? t("settings.features.reasoningModel", { model: model.id })
                : model.id}
            </option>
          ));
        const recommended = validator ? models.filter((m) => m.reasoning) : [];
        return (
          <fieldset
            key={feature}
            disabled={pending || !settings}
            aria-label={name}
            className="space-y-2 p-4"
          >
            <h3 className="text-sm font-semibold">{name}</h3>
            <div className="grid gap-2 sm:grid-cols-2">
              <label className="min-w-0 space-y-1 text-sm">
                <span>{t("settings.features.provider")}</span>
                <select
                  className={featureSelectClass}
                  value={providerId}
                  onChange={(event) => {
                    const id = event.target.value;
                    setDrafts((current) => ({ ...current, [feature]: id }));
                    if (!id) void save(feature, emptyChoice);
                  }}
                >
                  <option value="">
                    {t(`settings.features.empty.${feature}`)}
                  </option>
                  {providers.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.label}
                    </option>
                  ))}
                </select>
              </label>
              {providerId && (
                <label className="min-w-0 space-y-1 text-sm">
                  <span>{t("settings.features.model")}</span>
                  <select
                    className={featureSelectClass}
                    value={selected ? selectedModel : ""}
                    onChange={(event) => {
                      const model = models.find(
                        (entry) => entry.id === event.target.value,
                      );
                      if (model)
                        void save(feature, {
                          provider_id: providerId,
                          model: model.id,
                          reasoning:
                            validator && model.reasoning ? "Medium" : "Off",
                        });
                    }}
                  >
                    <option value="" disabled>
                      {t(
                        `settings.features.${models.length ? "chooseModel" : "noModels"}`,
                      )}
                    </option>
                    {recommended.length > 0 && (
                      <optgroup label={t("settings.features.recommended")}>
                        {options(recommended)}
                      </optgroup>
                    )}
                    {options(
                      validator
                        ? models.filter((model) => !model.reasoning)
                        : models,
                    )}
                  </select>
                </label>
              )}
              {selected?.reasoning && (
                <label className="min-w-0 space-y-1 text-sm">
                  <span>{t("settings.features.reasoning")}</span>
                  <select
                    className={featureSelectClass}
                    value={choice.reasoning ?? "Off"}
                    onChange={(e) => {
                      const reasoning = reasoningLevels.find(
                        (level) => level === e.target.value,
                      );
                      if (reasoning)
                        void save(feature, { ...choice, reasoning });
                    }}
                  >
                    {reasoningLevels.map((level) => (
                      <option key={level} value={level}>
                        {t(`settings.features.level.${level}`)}
                      </option>
                    ))}
                  </select>
                </label>
              )}
            </div>
            {(validator || feature === "Rewrite") && (
              <p className="text-xs text-ink-2">
                {t(`settings.features.help.${feature}`)}
              </p>
            )}
            {feature === "Rewrite" && appleSelected && appleUnavailable ? (
              <p role="status" className="text-sm text-warning">
                {t("settings.postProcessing.api.appleIntelligence.unavailable")}
              </p>
            ) : (
              <FeatureStatusLine
                status={statuses.find((status) => status.feature === feature)}
              />
            )}
          </fieldset>
        );
      })}
    </SettingsGroup>
  );
}
