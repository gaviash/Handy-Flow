import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import {
  commands,
  type Learned,
  type LearningStats,
  type StyleRule,
} from "@/bindings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { useSettings } from "../../../hooks/useSettings";
import { notify } from "@/stores/noticeStore";
import { EmptyMascot } from "../../shared/Mascot";

// Mirrors learning.rs: a pending word not confirmed within 30 days is
// forgotten. An active word is never forgotten (Wispr keeps its dictionary).
const FORGET_AFTER_S = 30 * 86_400;
const now = () => Date.now() / 1000;
/** Learned or used in the last day: it glows once on arrival. */
const isFresh = (lastSeen: number) => now() - lastSeen < 86_400;
/** A pending word: full ink until its last week, then fading down to 45%. */
const inkLeft = (w: Learned) => {
  if (w.active) return 1;
  const daysLeft = (w.last_seen + FORGET_AFTER_S - now()) / 86_400;
  return daysLeft >= 7 ? 1 : 0.45 + (0.55 * Math.max(daysLeft, 0)) / 7;
};

/** What Handy Flow learned: words, style rules and the last 30 days' counts. */
export function useLearning() {
  const [words, setWords] = useState<Learned[]>([]);
  const [rules, setRules] = useState<StyleRule[]>([]);
  const [stats, setStats] = useState<LearningStats | null>(null);
  const refresh = () =>
    Promise.all([
      commands.getLearnedWords().then(setWords),
      commands.getStyleRules().then(setRules),
      commands.getLearningStats().then(setStats),
    ]);
  useEffect(() => {
    refresh();
    // A word learned while the page is open shows up at once, glowing.
    const unlisten = listen("learned-words", () => refresh());
    return () => {
      unlisten.then((f) => f());
    };
  }, []);
  return { words, rules, stats, refresh };
}

type Learning = ReturnType<typeof useLearning>;

const Pending: React.FC = () => {
  const { t } = useTranslation();
  return (
    <span className="text-xs text-mid-gray">
      {" "}
      ({t("settings.postProcessing.learning.pending")})
    </span>
  );
};

/** Handy Adaptive (F121): whether corrections go to the learning validator. */
export const ValidatorToggle: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  return (
    <ToggleSwitch
      checked={getSetting("learning_judge_enabled") ?? true}
      onChange={(v) => updateSetting("learning_judge_enabled", v)}
      isUpdating={isUpdating("learning_judge_enabled")}
      label={t("settings.postProcessing.learning.judge.label")}
      description={`${t("settings.postProcessing.learning.judge.description")} ${t("settings.postProcessing.learning.judge.offNote")}`}
      descriptionMode="inline"
      grouped={true}
    />
  );
};

/** Handy Adaptive (F96, F97, F113): learned spellings, delete, clear, export. */
export const LearnedWords: React.FC<Learning> = ({ words, stats, refresh }) => {
  const { t, i18n } = useTranslation();
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
  return (
    <>
      <SettingContainer
        title={t("settings.postProcessing.learning.words.label")}
        description={t("settings.postProcessing.learning.words.description")}
        layout="stacked"
        grouped={true}
      >
        <div className="w-full space-y-2">
          {stats && (
            <p className="text-xs text-mid-gray">
              {t("settings.postProcessing.learning.stats", { ...stats })}
            </p>
          )}
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
                  title={
                    w.active
                      ? undefined
                      : t("settings.postProcessing.learning.words.expires", {
                          date: new Date(
                            (w.last_seen + FORGET_AFTER_S) * 1000,
                          ).toLocaleDateString(i18n.language, {
                            day: "numeric",
                            month: "long",
                          }),
                        })
                  }
                >
                  <span style={{ opacity: inkLeft(w) }}>
                    <span
                      className={`font-semibold inline-block ${
                        isFresh(w.last_seen) ? "animate-learned" : ""
                      }`}
                    >
                      {w.term}
                    </span>
                    {!w.active && <Pending />}{" "}
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
    </>
  );
};

/** Style preferences drawn from corrections. */
export const StyleRules: React.FC<Learning> = ({ rules, refresh }) => {
  const { t } = useTranslation();
  return (
    <>
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
                    {!r.active && <Pending />}
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
    </>
  );
};
