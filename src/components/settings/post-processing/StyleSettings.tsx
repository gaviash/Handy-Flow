import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import type { Styles, Tone } from "@/bindings";
import { Segmented } from "../../ui/Segmented";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";

export const CATEGORIES: (keyof Styles)[] = [
  "email",
  "work",
  "personal",
  "other",
];
const TONES: Tone[] = ["formal", "casual", "very_casual", "excited", "default"];
const DEFAULTS: Styles = {
  email: "formal",
  work: "casual",
  personal: "casual",
  other: "default",
};

/** Handy Adaptive (F58): one tone per style category. */
export const StyleSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const styles = getSetting("styles") ?? DEFAULTS;
  // The category last changed: its example sentence shows below the list.
  const [shown, setShown] = useState<keyof Styles>("email");
  const options = TONES.map((tone) => ({
    value: tone,
    label: t(`settings.postProcessing.styles.tones.${tone}`),
  }));

  return (
    <>
      {CATEGORIES.map((category) => (
        <SettingContainer
          key={category}
          title={t(`settings.postProcessing.styles.categories.${category}`)}
          description={t("settings.postProcessing.styles.description")}
          descriptionMode="tooltip"
          layout="stacked"
          grouped={true}
        >
          <Segmented
            options={options}
            value={styles[category]}
            ariaLabel={t(
              `settings.postProcessing.styles.categories.${category}`,
            )}
            onChange={(value) => {
              setShown(category);
              updateSetting("styles", { ...styles, [category]: value });
            }}
          />
        </SettingContainer>
      ))}
      <div className="row-setting px-4 py-3" aria-live="polite">
        <p className="text-xs text-ink-3">
          {t("settings.postProcessing.styles.exampleLabel", {
            category: t(`settings.postProcessing.styles.categories.${shown}`),
            tone: t(`settings.postProcessing.styles.tones.${styles[shown]}`),
          })}
        </p>
        <p
          key={`${shown}-${styles[shown]}`}
          className="mt-1 text-sm animate-rise"
        >
          {t(`settings.postProcessing.styles.examples.${styles[shown]}`)}
        </p>
      </div>
    </>
  );
};
