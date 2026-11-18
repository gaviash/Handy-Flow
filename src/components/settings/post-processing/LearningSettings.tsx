import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type JudgeChoice,
  type Learned,
  type StyleRule,
} from "@/bindings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { Dropdown } from "../../ui/Dropdown";
import { useSettings } from "../../../hooks/useSettings";
import { AsrContextSettings } from "./AsrContextSettings";
import { notify } from "@/stores/noticeStore";
import { EmptyMascot } from "../../shared/Mascot";

// Mirrors learning.rs: a word unused for 30 days is forgotten.
const FORGET_AFTER_S = 30 * 86_400;
const now = () => Date.now() / 1000;
/** Learned or used in the last day: it glows once on arrival. */
const isFresh = (lastSeen: number) => now() - lastSeen < 86_400;
/** Full ink until the last week, then fading like drying ink, down to 45%. */
const inkLeft = (lastSeen: number) => {
  const daysLeft = (lastSeen + FORGET_AFTER_S - now()) / 86_400;
  return daysLeft >= 7 ? 1 : 0.45 + (0.55 * Math.max(daysLeft, 0)) / 7;
};

/** Handy Adaptive (F96, F97, F113): learned spellings, delete, clear, export. */
export const LearningSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [words, setWords] = useState<Learned[]>([]);
  const [rules, setRules] = useState<StyleRule[]>([]);
  const refresh = () =>
    Promise.all([
      commands.getLearnedWords().then(setWords),
      commands.getStyleRules().then(setRules),
    ]);
  const pending = (
    <span className="text-xs text-mid-gray">
      {" "}
      ({t("settings.postProcessing.learning.pending")})
    </span>
  );
  useEffect(() => {
    refresh();
    // A word learned while the page is open shows up at once, glowing.
    const unlisten = listen("learned-words", () => refresh());
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const exportWords = async () => {
    const path = await save({ defaultPath: "handy-learned-words.json" });
    if (!path) return;
    const result = await commands
      .exportLearnedWords(path)
      .catch(() => ({ status: "error" as const }));
    if (result.status === "error")
      notify({
        title: t("settings.postProcessing.learning.words.exportError"),
      });
  };
  // Which provider the correction judge calls; null = it can't run.
  const [judge, setJudge] = useState<string | null>(null);
  const judgeOn = getSetting("learning_judge_enabled") ?? true;
  const judgeChoice = getSetting("learning_judge_provider_id") ?? "";
  const [choices, setChoices] = useState<JudgeChoice[]>([]);
  useEffect(() => {
    commands.getJudgeProvider().then(setJudge);
    commands.getJudgeChoices().then(setChoices);
  }, [judgeOn, judgeChoice]);

  return (
    <>
      <ToggleSwitch
        checked={getSetting("learning_enabled") ?? true}
        onChange={(v) => updateSetting("learning_enabled", v)}
        isUpdating={isUpdating("learning_enabled")}
        label={t("settings.postProcessing.learning.toggle.label")}
        description={t("settings.postProcessing.learning.toggle.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      <ToggleSwitch
        checked={getSetting("learned_sound_enabled") ?? true}
        onChange={(v) => updateSetting("learned_sound_enabled", v)}
        isUpdating={isUpdating("learned_sound_enabled")}
        label={t("settings.postProcessing.learning.sound.label")}
        description={t("settings.postProcessing.learning.sound.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      <ToggleSwitch
        checked={judgeOn}
        onChange={(v) => updateSetting("learning_judge_enabled", v)}
        isUpdating={isUpdating("learning_judge_enabled")}
        label={t("settings.postProcessing.learning.judge.label")}
        description={t("settings.postProcessing.learning.judge.description")}
        descriptionMode="tooltip"
        grouped={true}
      />
      {judgeOn && (
        <SettingContainer
          title={t("settings.postProcessing.learning.judge.provider.label")}
          description={t(
            "settings.postProcessing.learning.judge.provider.description",
          )}
          descriptionMode="tooltip"
          grouped={true}
        >
          <Dropdown
            options={[
              {
                value: "",
                label: t(
                  "settings.postProcessing.learning.judge.provider.auto",
                ),
              },
              ...choices.map((c) => ({ value: c.id, label: c.label })),
            ]}
            selectedValue={judgeChoice}
            onSelect={(v) => updateSetting("learning_judge_provider_id", v)}
            disabled={isUpdating("learning_judge_provider_id")}
          />
        </SettingContainer>
      )}
      {judgeOn && (
        <p className="row-setting px-4 pb-3 -mt-2 text-xs text-ink-3">
          {judge
            ? t("settings.postProcessing.learning.judge.status", {
                provider: judge,
              })
            : t("settings.postProcessing.learning.judge.none")}
        </p>
      )}
      <SettingContainer
        title={t("settings.postProcessing.learning.words.label")}
        description={t("settings.postProcessing.learning.words.description")}
        layout="stacked"
        grouped={true}
      >
        <div className="w-full space-y-2">
          {words.length === 0 ? (
            <div className="py-2 text-center">
              <EmptyMascot page="learned-words" size={36} />
              <p className="text-sm text-mid-gray">
                {t("settings.postProcessing.learning.words.empty")}
              </p>
            </div>
          ) : (
            <ul className="space-y-1">
              {words.map((w) => (
                <li
                  key={w.term}
                  className="flex items-center justify-between gap-2 text-sm"
                  title={t("settings.postProcessing.learning.words.expires", {
                    date: new Date(
                      (w.last_seen + FORGET_AFTER_S) * 1000,
                    ).toLocaleDateString(i18n.language, {
                      day: "numeric",
                      month: "long",
                    }),
                  })}
                >
                  <span style={{ opacity: inkLeft(w.last_seen) }}>
                    <span
                      className={`font-semibold inline-block ${
                        isFresh(w.last_seen) ? "animate-learned" : ""
                      }`}
                    >
                      {w.term}
                    </span>
                    {!w.active && pending}{" "}
                    <span className="text-mid-gray">
                      {t("settings.postProcessing.learning.words.heard", {
                        heard: w.heard.join(", "),
                      })}
                    </span>
                  </span>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      commands.deleteLearnedWord(w.term).then(refresh)
                    }
                  >
                    {t("settings.postProcessing.learning.words.delete")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
          <div className="flex gap-2">
            <Button variant="secondary" size="sm" onClick={refresh}>
              {t("settings.postProcessing.context.last.refresh")}
            </Button>
            <Button variant="secondary" size="sm" onClick={exportWords}>
              {t("settings.postProcessing.learning.words.export")}
            </Button>
            <Button
              variant="secondary"
              size="sm"
              disabled={words.length === 0}
              onClick={async () => {
                if (
                  await ask(
                    t("settings.postProcessing.learning.words.confirmClear"),
                  )
                )
                  commands.clearLearnedWords().then(refresh);
              }}
            >
              {t("settings.postProcessing.learning.words.clear")}
            </Button>
          </div>
        </div>
      </SettingContainer>
      <SettingContainer
        title={t("settings.postProcessing.learning.rules.label")}
        description={t("settings.postProcessing.learning.rules.description")}
        layout="stacked"
        grouped={true}
      >
        {rules.length === 0 ? (
          <p className="text-sm text-mid-gray">
            {t("settings.postProcessing.learning.rules.empty")}
          </p>
        ) : (
          <div className="w-full space-y-2">
            <ul className="w-full space-y-1">
              {rules.map((r) => (
                <li
                  key={r.rule}
                  className="flex items-center justify-between gap-2 text-sm"
                >
                  <span>
                    {r.rule}
                    {!r.active && pending}
                  </span>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() =>
                      commands.deleteStyleRule(r.rule).then(refresh)
                    }
                  >
                    {t("settings.postProcessing.learning.words.delete")}
                  </Button>
                </li>
              ))}
            </ul>
            <Button
              variant="secondary"
              size="sm"
              onClick={async () => {
                if (
                  await ask(
                    t("settings.postProcessing.learning.rules.confirmClear"),
                  )
                )
                  commands.clearStyleRules().then(refresh);
              }}
            >
              {t("settings.postProcessing.learning.rules.clear")}
            </Button>
          </div>
        )}
      </SettingContainer>
      <AsrContextSettings />
    </>
  );
};
