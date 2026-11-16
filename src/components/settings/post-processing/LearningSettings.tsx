import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask, save } from "@tauri-apps/plugin-dialog";
import { commands, type Learned } from "@/bindings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { useSettings } from "../../../hooks/useSettings";

/** Handy Adaptive (F96, F97, F113): learned spellings, delete, clear, export. */
export const LearningSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [words, setWords] = useState<Learned[]>([]);
  const refresh = () => commands.getLearnedWords().then(setWords);
  useEffect(() => {
    refresh();
  }, []);

  const exportWords = async () => {
    const path = await save({ defaultPath: "handy-learned-words.json" });
    if (path) await commands.exportLearnedWords(path);
  };

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
      <SettingContainer
        title={t("settings.postProcessing.learning.words.label")}
        description={t("settings.postProcessing.learning.words.description")}
        layout="stacked"
        grouped={true}
      >
        <div className="w-full space-y-2">
          {words.length === 0 ? (
            <p className="text-sm text-mid-gray">
              {t("settings.postProcessing.learning.words.empty")}
            </p>
          ) : (
            <ul className="space-y-1">
              {words.map((w) => (
                <li
                  key={w.term}
                  className="flex items-center justify-between gap-2 text-sm"
                >
                  <span>
                    <span className="font-semibold">{w.term}</span>{" "}
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
