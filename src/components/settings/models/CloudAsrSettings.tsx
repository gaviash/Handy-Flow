import { useTranslation } from "react-i18next";
import { useSettings } from "@/hooks/useSettings";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { TranslateToEnglish } from "../TranslateToEnglish";
import { LANGUAGES, languageName } from "@/lib/constants/languages";

export function CloudAsrSettings() {
  const { t, i18n } = useTranslation();
  const { getSetting, isUpdating, updateSetting } = useSettings();
  const cloud = getSetting("cloud_asr");
  if (!cloud || !getSetting("features")?.transcription_api.provider_id)
    return null;
  return (
    <details className="rounded-lg border border-line p-4">
      <summary className="cursor-pointer rounded text-sm font-medium focus-visible:outline-2 focus-visible:outline-logo-primary">
        {t("settings.features.speechOptions")}
      </summary>
      <div className="mt-4 space-y-4">
        <label className="block space-y-2 text-sm">
          <span className="block font-medium">
            {t("settings.transcription.timeoutTitle")}
          </span>
          <Input
            type="number"
            min={3}
            max={120}
            value={cloud.timeout_secs}
            disabled={isUpdating("cloud_asr")}
            className="w-24"
            onChange={(event) => {
              const seconds = Number(event.target.value);
              if (Number.isInteger(seconds) && seconds >= 3 && seconds <= 120) {
                void updateSetting("cloud_asr", {
                  ...cloud,
                  timeout_secs: seconds,
                });
              }
            }}
          />
          <span className="block text-xs text-ink-2">
            {t("settings.cloudAsr.timeout.description")}
          </span>
        </label>
        <label className="block space-y-2 text-sm">
          <span className="block font-medium">
            {t("settings.transcription.apiLanguage")}
          </span>
          <Select
            value={getSetting("selected_language") || "auto"}
            options={[
              {
                value: "auto",
                label: t("settings.transcription.automaticLanguage"),
              },
              ...LANGUAGES.map((language) => ({
                value: language.code,
                label: languageName(language.code, i18n.language),
              })),
            ]}
            disabled={isUpdating("selected_language")}
            onChange={(value) => {
              if (value) void updateSetting("selected_language", value);
            }}
          />
        </label>
        <TranslateToEnglish grouped />
        <p className="text-xs text-ink-2">
          {t("settings.transcription.sharedLanguage")}
        </p>
      </div>
    </details>
  );
}
