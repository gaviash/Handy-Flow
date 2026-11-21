import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { Keyboard, Languages, Mic, Power, Volume2 } from "lucide-react";
import {
  CardGrid,
  ExpertSection,
  SettingCard,
  SettingsGroup,
  StatusHeader,
  SubPage,
  useSubPage,
} from "@/components/ui";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { ChannelSelector } from "../ChannelSelector";
import { ClamshellMicrophoneSelector } from "../ClamshellMicrophoneSelector";
import { ShortcutInput } from "../ShortcutInput";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { ShortcutActivationSetting } from "../ShortcutActivation";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";
import { useOsType } from "../../../hooks/useOsType";
import { formatKeyCombination } from "../../../lib/utils/keyboard";
import { VolumeSlider } from "../VolumeSlider";
import { SoundPicker } from "../SoundPicker";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { PreferredLanguages } from "../PreferredLanguages";
import { ChineseScriptSetting } from "../ChineseScript";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { AutostartToggle } from "../AutostartToggle";
import { StartHidden } from "../StartHidden";
import { ShowTrayIcon } from "../ShowTrayIcon";
import { HoldThreshold } from "../debug/HoldThreshold";
import { isChinese } from "@/lib/utils/languageSuggestions";
import { TodayCounter } from "./TodayCounter";

type Topic = "shortcuts" | "languages" | "microphone" | "sounds" | "startup";

const g = (key: string) => `settings.general.${key}`;

/** How you dictate: shortcuts, languages, microphone, sounds, startup. */
export const GeneralSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { audioFeedbackEnabled, getSetting } = useSettings();
  const osType = useOsType();
  const sub = useSubPage<Topic>();
  const isLinux = type() === "linux";
  const spoken = getSetting("spoken_languages") ?? [];
  // Only for Chinese speakers, or anyone who already picked a script.
  const speaksChinese =
    spoken.some(isChinese) ||
    (getSetting("chinese_script") ?? "as_transcribed") !== "as_transcribed";
  const languageName = (code: string) => {
    try {
      return (
        new Intl.DisplayNames([i18n.language], { type: "language" }).of(code) ??
        code
      );
    } catch {
      return code;
    }
  };
  const keys = (id: string) =>
    formatKeyCombination(
      getSetting("bindings")?.[id]?.current_binding ?? "",
      osType,
    ) || t(g("shortcut.none"));
  const raw = keys("transcribe");
  const enhanced = keys("transcribe_with_post_process");
  const rewriteOn = getSetting("post_process_enabled") ?? false;
  const mic = getSetting("selected_microphone") || t(g("defaultMic"));
  const title = t(g("title"));

  const page = (topic: Topic, heading: string, body: React.ReactNode) =>
    sub.open === topic && (
      <SubPage parent={title} title={heading} onBack={sub.back}>
        {body}
      </SubPage>
    );
  const open =
    page(
      "shortcuts",
      t(g("shortcuts")),
      <>
        <SettingsGroup>
          <ShortcutInput shortcutId="transcribe" grouped />
          <ShortcutInput shortcutId="transcribe_with_post_process" grouped />
          <ShortcutActivationSetting descriptionMode="inline" grouped />
          {/* Cancel shortcut remains hidden on Linux because of dynamic shortcut instability. */}
          {!isLinux && <ShortcutInput shortcutId="cancel" grouped />}
        </SettingsGroup>
        <ExpertSection summary={t(g("shortcutsExpert"))}>
          <SettingsGroup>
            <HoldThreshold descriptionMode="inline" grouped />
          </SettingsGroup>
        </ExpertSection>
      </>,
    ) ||
    page(
      "languages",
      t("settings.languages.title"),
      <SettingsGroup>
        <PreferredLanguages descriptionMode="inline" grouped />
        {speaksChinese && (
          <ChineseScriptSetting descriptionMode="inline" grouped />
        )}
        <AppLanguageSelector descriptionMode="inline" grouped />
      </SettingsGroup>,
    ) ||
    page(
      "microphone",
      t("settings.sound.microphone.title"),
      <>
        <SettingsGroup>
          <MicrophoneSelector descriptionMode="inline" grouped />
        </SettingsGroup>
        <ExpertSection summary={t(g("micExpert"))}>
          <SettingsGroup>
            <ChannelSelector descriptionMode="inline" grouped />
            <ClamshellMicrophoneSelector descriptionMode="inline" grouped />
          </SettingsGroup>
        </ExpertSection>
      </>,
    ) ||
    page(
      "sounds",
      t(g("sounds")),
      <>
        <SettingsGroup>
          <AudioFeedback descriptionMode="inline" grouped />
          <VolumeSlider disabled={!audioFeedbackEnabled} />
          <SoundPicker
            label={t("settings.debug.soundTheme.label")}
            description={t("settings.debug.soundTheme.description")}
          />
          <MuteWhileRecording descriptionMode="inline" grouped />
        </SettingsGroup>
        <ExpertSection summary={t("settings.sound.outputDevice.title")}>
          <SettingsGroup>
            <OutputDeviceSelector
              descriptionMode="inline"
              grouped
              disabled={!audioFeedbackEnabled}
            />
          </SettingsGroup>
        </ExpertSection>
      </>,
    ) ||
    page(
      "startup",
      t(g("startup")),
      <SettingsGroup>
        <AutostartToggle descriptionMode="inline" grouped />
        <StartHidden descriptionMode="inline" grouped />
        <ShowTrayIcon descriptionMode="inline" grouped />
      </SettingsGroup>,
    );
  if (open) return open;

  const autostart = getSetting("autostart_enabled") ?? false;
  return (
    <div className="max-w-3xl w-full mx-auto space-y-5 pb-4">
      <h1 className="page-title">{title}</h1>
      <TodayCounter />
      <StatusHeader
        tone="ok"
        title={[
          t(g("statusRaw"), { shortcut: raw }),
          rewriteOn && t(g("statusEnhanced"), { shortcut: enhanced }),
        ]
          .filter(Boolean)
          .join(" · ")}
        detail={t(g("statusMic"), { mic })}
      />
      <CardGrid>
        <SettingCard
          ref={sub.cardRef("shortcuts")}
          icon={Keyboard}
          title={t(g("shortcuts"))}
          summary={rewriteOn ? `${raw} · ${enhanced}` : raw}
          onOpen={() => sub.show("shortcuts")}
        />
        <SettingCard
          ref={sub.cardRef("languages")}
          icon={Languages}
          title={t("settings.languages.title")}
          summary={t(g("languagesSummary"), {
            spoken: spoken.map(languageName).join(", ") || "—",
            app: languageName(getSetting("app_language") || i18n.language),
          })}
          onOpen={() => sub.show("languages")}
        />
        <SettingCard
          ref={sub.cardRef("microphone")}
          icon={Mic}
          title={t("settings.sound.microphone.title")}
          summary={mic}
          onOpen={() => sub.show("microphone")}
        />
        <SettingCard
          ref={sub.cardRef("sounds")}
          icon={Volume2}
          title={t(g("sounds"))}
          summary={
            audioFeedbackEnabled
              ? t(g("soundsOn"), {
                  volume: Math.round(
                    (getSetting("audio_feedback_volume") ?? 0.5) * 100,
                  ),
                })
              : t(g("soundsOff"))
          }
          onOpen={() => sub.show("sounds")}
        />
        <SettingCard
          ref={sub.cardRef("startup")}
          icon={Power}
          title={t(g("startup"))}
          summary={[
            t(g(autostart ? "startsWithWindows" : "startsByHand")),
            (getSetting("start_hidden") ?? false) && t(g("hidden")),
          ]
            .filter(Boolean)
            .join(" · ")}
          onOpen={() => sub.show("startup")}
        />
      </CardGrid>
    </div>
  );
};
