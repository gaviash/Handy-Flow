import React from "react";
import { useTranslation } from "react-i18next";
import {
  AudioLines,
  BookA,
  Gavel,
  PenLine,
  Sparkles,
  TextQuote,
} from "lucide-react";
import {
  CardGrid,
  ExpertSection,
  SettingCard,
  SettingsGroup,
  StatusHeader,
  SubPage,
  ToggleSwitch,
  useSubPage,
} from "@/components/ui";
import { useSettings } from "@/hooks/useSettings";
import { CustomWords } from "../CustomWords";
import { WordCorrectionThreshold } from "../debug/WordCorrectionThreshold";
import {
  FeatureSummary,
  featureStatusText,
  useFeatureStatuses,
} from "../models/FeatureModelsCard";
import { AsrContextSettings } from "../post-processing/AsrContextSettings";
import { UserName } from "../post-processing/ContextSettings";
import {
  LearnedWords,
  StyleRules,
  useLearning,
  ValidatorToggle,
} from "../post-processing/LearningSettings";
import { SnippetSettings } from "../post-processing/SnippetSettings";

type Topic = "custom" | "learned" | "rules" | "validator" | "snippets" | "asr";

const v = (key: string) => `settings.vocabulary.${key}`;
const l = (key: string) => `settings.postProcessing.learning.${key}`;

/** The words Handy Flow writes your way: yours, learned, and how it learns. */
export const VocabularySettings: React.FC<{ onShowModels?: () => void }> = ({
  onShowModels,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const sub = useSubPage<Topic>();
  const learning = useLearning();
  const validator = useFeatureStatuses().find(
    (s) => s.feature === "LearningValidator",
  );
  const learningOn = getSetting("learning_enabled") ?? true;
  const custom = getSetting("custom_words") ?? [];
  const asr = getSetting("asr_context");
  const pending = learning.words.filter((w) => !w.active).length;
  // A model that does not reason is usable, but makes more mistakes.
  const noReasoning =
    !!validator && !validator.problem && !validator.model_reasons;

  const title = t("sidebar.vocabulary");
  const page = (topic: Topic, heading: string, body: React.ReactNode) =>
    sub.open === topic && (
      <SubPage parent={title} title={heading} onBack={sub.back}>
        {body}
      </SubPage>
    );
  const open =
    page(
      "custom",
      t(v("custom")),
      <SettingsGroup>
        <CustomWords descriptionMode="inline" grouped />
      </SettingsGroup>,
    ) ||
    page(
      "learned",
      t(l("words.label")),
      <SettingsGroup>
        <LearnedWords {...learning} />
      </SettingsGroup>,
    ) ||
    page(
      "rules",
      t(l("rules.label")),
      <SettingsGroup>
        <StyleRules {...learning} />
      </SettingsGroup>,
    ) ||
    page(
      "validator",
      t(l("judge.label")),
      <SettingsGroup>
        <ValidatorToggle />
        <FeatureSummary
          feature="LearningValidator"
          onShowModels={onShowModels}
        />
        {noReasoning && (
          <p className="px-4 py-3 text-sm text-warning">
            {t(v("reasoningHint"))}
          </p>
        )}
      </SettingsGroup>,
    ) ||
    page(
      "snippets",
      t("settings.postProcessing.snippets.label"),
      <SettingsGroup>
        <SnippetSettings />
      </SettingsGroup>,
    ) ||
    page(
      "asr",
      t(l("asr.label")),
      <SettingsGroup>
        <AsrContextSettings part="vocabulary" />
      </SettingsGroup>,
    );
  if (open) return open;

  const asrParts = [
    asr?.learned_words && t(l("asr.learnedWords")),
    asr?.expressions && t(l("asr.expressions")),
  ].filter(Boolean);

  return (
    <div className="mx-auto w-full max-w-3xl space-y-5 pb-4">
      <div className="space-y-2">
        <h1 className="page-title">{title}</h1>
        <p className="text-sm text-ink-2">{t(v("subtitle"))}</p>
      </div>
      <StatusHeader
        tone={learningOn ? "ok" : "off"}
        title={t(v(learningOn ? "statusOn" : "statusPaused"), {
          custom: custom.length,
          learned: learning.words.length,
        })}
      />
      <SettingsGroup>
        <ToggleSwitch
          checked={learningOn}
          onChange={(on) => updateSetting("learning_enabled", on)}
          isUpdating={isUpdating("learning_enabled")}
          label={t(l("toggle.label"))}
          description={t(l("toggle.description"))}
          descriptionMode="inline"
          grouped
        />
        <UserName />
      </SettingsGroup>
      <CardGrid>
        <SettingCard
          ref={sub.cardRef("custom")}
          icon={BookA}
          title={t(v("custom"))}
          summary={t(v("count"), { count: custom.length })}
          onOpen={() => sub.show("custom")}
        />
        <SettingCard
          ref={sub.cardRef("learned")}
          icon={Sparkles}
          title={t(l("words.label"))}
          summary={t(v("learnedSummary"), {
            count: learning.words.length,
            pending,
          })}
          onOpen={() => sub.show("learned")}
        />
        <SettingCard
          ref={sub.cardRef("rules")}
          icon={PenLine}
          title={t(l("rules.label"))}
          summary={t(v("count"), { count: learning.rules.length })}
          onOpen={() => sub.show("rules")}
        />
        <SettingCard
          ref={sub.cardRef("validator")}
          icon={Gavel}
          title={t(l("judge.label"))}
          summary={validator ? featureStatusText(t, validator) : ""}
          tone={noReasoning ? "warn" : undefined}
          onOpen={() => sub.show("validator")}
        />
        <SettingCard
          ref={sub.cardRef("snippets")}
          icon={TextQuote}
          title={t("settings.postProcessing.snippets.label")}
          summary={t(v("count"), {
            count: (getSetting("snippets") ?? []).length,
          })}
          onOpen={() => sub.show("snippets")}
        />
        <SettingCard
          ref={sub.cardRef("asr")}
          icon={AudioLines}
          title={t(l("asr.label"))}
          summary={
            asr?.enabled
              ? [t("status.on"), ...asrParts].join(" · ")
              : t("status.off")
          }
          onOpen={() => sub.show("asr")}
        />
      </CardGrid>
      <ExpertSection summary={t(v("expert"))}>
        <SettingsGroup>
          <ToggleSwitch
            checked={getSetting("learned_sound_enabled") ?? true}
            onChange={(on) => updateSetting("learned_sound_enabled", on)}
            isUpdating={isUpdating("learned_sound_enabled")}
            label={t(l("sound.label"))}
            description={t(l("sound.description"))}
            descriptionMode="inline"
            grouped
          />
          <WordCorrectionThreshold descriptionMode="inline" grouped />
        </SettingsGroup>
      </ExpertSection>
    </div>
  );
};
