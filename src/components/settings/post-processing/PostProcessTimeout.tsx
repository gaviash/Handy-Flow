import React from "react";
import { useTranslation } from "react-i18next";
import { Slider } from "../../ui/Slider";
import { useSettings } from "../../../hooks/useSettings";

/** Handy Adaptive (F57): how long the rewrite may take before the cleaned transcript is pasted. */
export const PostProcessTimeout: React.FC = () => {
  const { t } = useTranslation();
  const { settings, updateSetting, resetSetting, isUpdating } = useSettings();

  return (
    <Slider
      value={settings?.post_process_timeout_secs ?? 8}
      onChange={(value) => updateSetting("post_process_timeout_secs", value)}
      onReset={() => resetSetting("post_process_timeout_secs")}
      isResetting={isUpdating("post_process_timeout_secs")}
      min={1}
      max={60}
      step={1}
      label={t("settings.postProcessing.timeout.title")}
      description={t("settings.postProcessing.timeout.description")}
      descriptionMode="tooltip"
      grouped={true}
      formatValue={(v) => `${v} s`}
    />
  );
};
