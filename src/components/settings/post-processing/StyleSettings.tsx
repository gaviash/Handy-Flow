import React from "react";
import { useTranslation } from "react-i18next";
import type { Styles, Tone } from "@/bindings";
import { Dropdown } from "../../ui/Dropdown";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";

const CATEGORIES: (keyof Styles)[] = ["email", "work", "personal", "other"];
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
          grouped={true}
        >
          <Dropdown
            options={options}
            selectedValue={styles[category]}
            onSelect={(value) =>
              updateSetting("styles", { ...styles, [category]: value as Tone })
            }
          />
        </SettingContainer>
      ))}
    </>
  );
};
