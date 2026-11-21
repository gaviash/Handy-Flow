import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cpu, RefreshCw, Server, Workflow } from "lucide-react";
import { Tooltip } from "@/components/ui/Tooltip";
import { useModelStore } from "@/stores/modelStore";
import { notify } from "@/stores/noticeStore";
import {
  ModelTable,
  Note,
  useModelFilters,
  type TableSection,
} from "@/components/shared/modelTable/ModelTable";
import RetiredModelsNotice from "./RetiredModelsNotice";
import { ModelRow } from "./ModelRow";
import { Legend } from "./Legend";
import { useModelEntries } from "./modelEntries";
import { CloudAsrSettings } from "./CloudAsrSettings";
import {
  FeatureModelsCard,
  featureStatusText,
  useFeatureStatuses,
} from "./FeatureModelsCard";
import {
  CardGrid,
  ExpertSection,
  SettingCard,
  SettingsGroup,
  StatusHeader,
  SubPage,
  useSubPage,
} from "@/components/ui";
import { ModelUnloadTimeoutSetting } from "../ModelUnloadTimeout";
import { LongFormChunking } from "../LongFormChunking";
import { AccelerationSelector } from "../AccelerationSelector";
import { LanguageDetection } from "../LanguageDetection";
import { LanguageDetectionWindow } from "../debug/LanguageDetectionWindow";
import { ProvidersCard } from "./ProvidersCard";
import { ModelSettingsCard } from "./ModelSettingsCard";
import { useSettings } from "@/hooks/useSettings";
import { getTranscriptionConfig } from "@/lib/utils/transcriptionConfig";

/** Past this many models on disk, the rest fold under "Show N more". */
const YOURS_LIMIT = 5;

/** Look for model files added outside Handy: quiet, beside its section's name. */
const Rescan: React.FC = () => {
  const { t } = useTranslation();
  const rescanning = useModelStore((s) => s.isRescanning);
  return (
    <Tooltip text={t("settings.models.rescan.tooltip")}>
      <button
        type="button"
        onClick={() => useModelStore.getState().rescanLocalModels()}
        disabled={rescanning}
        aria-label={t("settings.models.rescan.label")}
        className="ms-[6px] grid size-[22px] cursor-pointer place-items-center rounded-full text-ink-3 hover:bg-wash hover:text-text disabled:cursor-default focus:outline-none focus-visible:ring-1 focus-visible:ring-logo-primary"
      >
        <RefreshCw
          className={`size-[12px] ${rescanning ? "animate-spin" : ""}`}
          aria-hidden="true"
        />
      </button>
    </Tooltip>
  );
};

interface ModelsSettingsProps {
  /** Where the languages are changed (General). */
  onChangeLanguages?: () => void;
  onShowReformulation?: () => void;
  /** Open straight on a topic, e.g. "features" from Enhancement. */
  initialTopic?: ModelsTopic;
}

export type ModelsTopic = "providers" | "features" | "local";

/** The Models page: onboarding's model table, filling the window. */
export const ModelsSettings: React.FC<ModelsSettingsProps> = ({
  onChangeLanguages,
  onShowReformulation,
  initialTopic,
}) => {
  const { t } = useTranslation();
  const sub = useSubPage<ModelsTopic>(initialTopic ?? null);
  const statuses = useFeatureStatuses();
  const loading = useModelStore((s) => s.loading);
  const { facts, lists } = useModelEntries();
  const { filters, setFilters, searching, matches } = useModelFilters();
  const [more, setMore] = useState(false);
  const { settings, isLoading } = useSettings();
  const { models, currentModel } = useModelStore();
  const modelError = useModelStore((s) => s.error);
  useEffect(() => {
    if (modelError)
      notify({ title: t("settings.models.actionError"), detail: modelError });
  }, [modelError, t]);
  const config = getTranscriptionConfig(settings, models, currentModel);
  const k = (key: string) => `settings.transcription.${key}`;

  const keep = (xs: typeof lists.yours) => xs.filter(matches);
  const moreButton = !searching && lists.yours.length > YOURS_LIMIT && (
    <button
      type="button"
      onClick={() => setMore((m) => !m)}
      className="ms-[40px] mt-[2px] flex h-[28px] cursor-pointer items-center gap-[8px] rounded-[7px] px-[2px] text-small font-medium text-ink-2 hover:text-text focus:outline-none focus-visible:ring-1 focus-visible:ring-logo-primary"
    >
      {more
        ? t("settings.models.showFewer")
        : t("settings.models.showMore", {
            count: lists.yours.length - YOURS_LIMIT,
          })}
      <span
        aria-hidden="true"
        className={`inline-block size-[6px] shrink-0 border-e-[1.5px] border-b-[1.5px] border-current ${more ? "mt-[3px] -rotate-[135deg]" : "-mt-[3px] rotate-45"}`}
      />
    </button>
  );

  const sections: TableSection[] = searching
    ? [
        {
          key: "yours",
          label: t("settings.models.sections.yours"),
          rows: keep(lists.yours),
        },
        {
          key: "fits",
          label: t("settings.models.sections.fits"),
          rows: keep(lists.fits),
        },
        {
          key: "partial",
          label: t("modelTable.missingLanguage"),
          rows: keep(lists.partial),
        },
        {
          key: "untested",
          label: t("settings.models.sections.untested"),
          rows: filters.query.trim() ? keep(lists.untested) : [],
          note: t("settings.models.untestedNote"),
        },
      ]
    : [
        {
          key: "yours",
          label: (
            <>
              {t("settings.models.sections.yours")}
              <Rescan />
            </>
          ),
          rows: more ? lists.yours : lists.yours.slice(0, YOURS_LIMIT),
          after: lists.yours.length ? (
            moreButton
          ) : (
            <Note>{t("settings.models.emptyYours")}</Note>
          ),
        },
        {
          key: "fits",
          label: t("settings.models.sections.fits"),
          rows: lists.fits,
        },
        {
          key: "partial",
          label: t("modelTable.missingLanguage"),
          rows: lists.partial,
        },
      ];

  const title = t(k("title"));
  // Local transcription chosen, and no model to run it.
  const noModel = !config.api && !loading && !config.localModel;
  // Features the user turned on that cannot run as set up.
  const OFF = [
    "RewriteOff",
    "ValidatorOff",
    "LearningOff",
    "ContextOff",
    "NotChosen",
  ];
  const broken = statuses.find((s) => s.problem && !OFF.includes(s.problem));
  const status = (f: string) => statuses.find((s) => s.feature === f);
  const rewrite = status("Rewrite");
  const validator = status("LearningValidator");
  const speech = status("TranscriptionApi");
  const providers = settings?.post_process_providers ?? [];
  const keys = Object.values(settings?.post_process_api_keys ?? {}).filter(
    Boolean,
  ).length;

  if (sub.open === "providers")
    return (
      <SubPage
        parent={title}
        title={t("settings.providers.title")}
        onBack={sub.back}
      >
        <ProvidersCard />
      </SubPage>
    );
  if (sub.open === "features")
    return (
      <SubPage
        parent={title}
        title={t("settings.features.title")}
        onBack={sub.back}
      >
        <FeatureModelsCard />
        <CloudAsrSettings />
      </SubPage>
    );
  if (sub.open === "local")
    return (
      <SubPage
        parent={title}
        title={t(k(config.api ? "fallbackTitle" : "localTitle"))}
        onBack={sub.back}
      >
        <section className="space-y-3" aria-labelledby="local-model-title">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <h2 id="local-model-title" className="text-base font-semibold">
              {t(k(config.api ? "fallbackTitle" : "localTitle"))}
            </h2>
            {onChangeLanguages && (
              <button
                type="button"
                onClick={onChangeLanguages}
                className="shrink-0 cursor-pointer text-sm text-text underline decoration-line-strong underline-offset-[3px] hover:decoration-text focus:outline-none focus-visible:ring-1 focus-visible:ring-logo-primary"
              >
                {t("settings.models.changeLanguages")}
              </button>
            )}
          </div>
          <p className="text-sm text-ink-2">
            {t(k(config.api ? "fallbackHelp" : "localHelp"))}
          </p>
          <p className="text-sm font-medium">
            {loading
              ? t(k("loading"))
              : config.localModel?.name || t(k("localMissing"))}
          </p>
          {!loading && !config.localModel && (
            <p role="status" className="text-sm text-text">
              {t(k(config.api ? "noFallback" : "downloadRequired"))}
            </p>
          )}
          {!config.api && <ModelSettingsCard />}
          <details className="group" open={!config.localModel || undefined}>
            <summary className="cursor-pointer rounded py-2 text-sm font-medium text-text underline decoration-line-strong underline-offset-4 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary">
              {t(k(config.api ? "chooseFallback" : "chooseLocal"))}
            </summary>
            <p className="mb-3 text-xs text-ink-2">
              {t(k(config.api ? "catalogFallbackHelp" : "catalogLocalHelp"))}
            </p>
            <div className="mx-[12px] mb-[16px] flex-none empty:hidden">
              <RetiredModelsNotice />
            </div>
            {loading ? (
              <p className="py-8 text-center text-sm text-ink-2">
                {t(k("loading"))}
              </p>
            ) : (
              <div className="flex h-[480px] min-h-0 flex-col">
                <ModelTable
                  grading={facts}
                  filters={filters}
                  setFilters={setFilters}
                  // "Yours" always shows at rest; a search hides empty sections.
                  sections={sections.filter(
                    (s) => s.rows.length || (s.key === "yours" && !searching),
                  )}
                  renderRow={(e) => (
                    <ModelRow
                      key={e.id}
                      entry={e}
                      role={config.api ? "fallback" : "primary"}
                    />
                  )}
                  actionsHead={<Legend tier={facts.tier} />}
                  footer={
                    !searching &&
                    lists.untested.length > 0 && (
                      <Note>
                        {t("settings.models.untestedHidden", {
                          count: lists.untested.length,
                        })}
                      </Note>
                    )
                  }
                  thinEdge
                />
              </div>
            )}
          </details>
        </section>
      </SubPage>
    );

  return (
    <div className="mx-auto w-full max-w-3xl space-y-5 pb-4">
      <div className="space-y-2">
        <h1 className="page-title">{title}</h1>
        <p className="text-sm text-ink-2">{t(k("intro"))}</p>
        {onShowReformulation && (
          <button
            type="button"
            onClick={onShowReformulation}
            className="cursor-pointer rounded text-sm text-text underline decoration-line-strong underline-offset-4 hover:decoration-text focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
          >
            {t(k("reformulationLink"))}
          </button>
        )}
      </div>
      <StatusHeader
        tone={config.issue || noModel || broken ? "warn" : "ok"}
        title={
          isLoading
            ? t(k("loading"))
            : config.api
              ? t(k("apiSummary"), {
                  provider: config.providerName,
                  model: config.model || t(k("modelMissing")),
                })
              : t(k("localSummary"), {
                  model: config.localModel?.name || t(k("localMissing")),
                })
        }
        detail={
          isLoading
            ? undefined
            : noModel
              ? t(k("downloadRequired"))
              : config.issue
                ? `${t(k(config.issue))} ${t(k(config.localModel ? "usingLocal" : "noFallback"))}`
                : broken
                  ? t("settings.models.featureProblem", {
                      name: t(`settings.features.name.${broken.feature}`),
                      status: featureStatusText(t, broken),
                    })
                  : t("settings.models.featuresReady")
        }
      />
      <CardGrid>
        <SettingCard
          ref={sub.cardRef("providers")}
          icon={Server}
          title={t("settings.providers.title")}
          summary={t("settings.models.providersSummary", {
            count: providers.length,
            keys,
          })}
          onOpen={() => sub.show("providers")}
        />
        <SettingCard
          ref={sub.cardRef("features")}
          icon={Workflow}
          title={t("settings.features.title")}
          summary={[
            rewrite &&
              `${t("settings.features.name.Rewrite")} ${rewrite.problem ? "—" : rewrite.provider_label}`,
            validator &&
              `${t("settings.features.name.LearningValidator")} ${
                validator.problem
                  ? "—"
                  : validator.automatic
                    ? t("settings.features.empty.LearningValidator")
                    : validator.provider_label
              }`,
            `${t("settings.features.name.TranscriptionApi")} ${
              speech && !speech.problem
                ? speech.provider_label
                : t("settings.models.local")
            }`,
          ]
            .filter(Boolean)
            .join(" · ")}
          tone={broken ? "warn" : undefined}
          onOpen={() => sub.show("features")}
        />
        <SettingCard
          ref={sub.cardRef("local")}
          icon={Cpu}
          title={t(k(config.api ? "fallbackTitle" : "localTitle"))}
          summary={
            loading
              ? t(k("loading"))
              : config.localModel?.name || t(k("localMissing"))
          }
          onOpen={() => sub.show("local")}
        />
      </CardGrid>
      <ExpertSection summary={t("settings.models.engineExpert")}>
        <SettingsGroup>
          <ModelUnloadTimeoutSetting descriptionMode="inline" grouped />
          <LongFormChunking descriptionMode="inline" grouped />
          <AccelerationSelector descriptionMode="inline" grouped />
          <LanguageDetection descriptionMode="inline" grouped />
          <LanguageDetectionWindow descriptionMode="inline" grouped />
        </SettingsGroup>
      </ExpertSection>
    </div>
  );
};
