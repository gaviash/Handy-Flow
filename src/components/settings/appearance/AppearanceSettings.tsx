import React from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ThemeSelector } from "../ThemeSelector";
import { ShowOverlay } from "../ShowOverlay";
import { SettingContainer } from "../../ui/SettingContainer";
import { Segmented } from "../../ui/Segmented";
import { PalettePicker } from "./PalettePicker";
import { useSettings } from "@/hooks/useSettings";
import {
  withThemeTransition,
  normalizeAppearance,
} from "@/lib/utils/appearance";

export const AppearanceSettings: React.FC<{ focusPill?: boolean }> = ({
  focusPill = false,
}) => {
  React.useEffect(() => {
    if (focusPill) document.getElementById("dictation-pill")?.scrollIntoView();
  }, [focusPill]);
  const { t } = useTranslation();
  const { settings, updateSetting } = useSettings();
  if (!settings) return null;
  const appearance = normalizeAppearance(settings);
  const label = (key: string) => t(`settings.appearance.${key}`);
  const overlay =
    settings.overlay_style === "minimal"
      ? settings.overlay_show_label
        ? "pill"
        : "wave"
      : settings.overlay_style;
  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <header>
        <h1 className="page-title">{label("title")}</h1>
        <p className="mt-2 text-sm text-ink-2">{label("description")}</p>
      </header>
      <PalettePicker
        value={appearance.palette}
        onChange={(palette) =>
          withThemeTransition(() => {
            void updateSetting("palette", palette);
          })
        }
      />
      <SettingsGroup>
        <ThemeSelector descriptionMode="tooltip" grouped={true} />
        <SettingContainer
          title={label("headings.title")}
          description={label("headings.description")}
          grouped
        >
          <Segmented<"editorial" | "modern">
            ariaLabel={label("headings.title")}
            value={appearance.heading_style}
            options={(["editorial", "modern"] as const).map((value) => ({
              value,
              label: label(`headings.${value}`),
            }))}
            onChange={(value) =>
              withThemeTransition(() => {
                void updateSetting("heading_style", value);
              })
            }
          />
        </SettingContainer>
        <SettingContainer
          title={label("density.title")}
          description={label("density.description")}
          grouped
        >
          <Segmented<"comfortable" | "compact">
            ariaLabel={label("density.title")}
            value={appearance.ui_density}
            options={(["comfortable", "compact"] as const).map((value) => ({
              value,
              label: label(`density.${value}`),
            }))}
            onChange={(value) =>
              withThemeTransition(() => {
                void updateSetting("ui_density", value);
              })
            }
          />
        </SettingContainer>
      </SettingsGroup>
      <div id="dictation-pill">
        <SettingsGroup title={label("overlay.title")}>
          <SettingContainer
            title={label("pillTone.title")}
            description={label("pillTone.description")}
            grouped
          >
            <Segmented<"discreet" | "expressive">
              ariaLabel={label("pillTone.title")}
              value={appearance.pill_tone}
              options={(["discreet", "expressive"] as const).map((value) => ({
                value,
                label: label(`pillTone.${value}`),
              }))}
              onChange={(value) => void updateSetting("pill_tone", value)}
            />
          </SettingContainer>
          <SettingContainer
            title={label("overlay.title")}
          description={label("pillTone.description")}
            grouped
          >
            <Segmented
              ariaLabel={label("overlay.title")}
              value={overlay ?? "none"}
              options={["pill", "wave", "live", "none"].map((value) => ({
                value,
                label:
                  value === "none"
                    ? t("settings.advanced.overlay.style.options.none")
                    : label(`overlay.${value}`),
              }))}
              onChange={(value) =>
                withThemeTransition(() => {
                  void updateSetting(
                    "overlay_style",
                    value === "none"
                      ? "none"
                      : value === "live"
                        ? "live"
                        : "minimal",
                  );
                  if (value === "pill" || value === "wave")
                    void updateSetting("overlay_show_label", value === "pill");
                })
              }
            />
          </SettingContainer>
          <ShowOverlay grouped />
        </SettingsGroup>
      </div>
    </div>
  );
};
