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

type Topic = "sounds";

const g = (key: string) => `settings.general.${key}`;

/** How you dictate: shortcuts, languages, microphone, sounds, startup. */
export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { audioFeedbackEnabled, getSetting } = useSettings();
  const osType = useOsType();
  const sub = useSubPage<Topic>();
  const isLinux = type() === "linux";
  // Only for Chinese speakers, or anyone who already picked a script.
  const speaksChinese =
    (getSetting("spoken_languages") ?? []).some(isChinese) ||
    (getSetting("chinese_script") ?? "as_transcribed") !== "as_transcribed";
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
  const open = page(
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
  );
  if (open) return open;

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
      <SettingsGroup title={t(g("shortcuts"))} icon={Keyboard}>
        <ShortcutInput shortcutId="transcribe" grouped />
        <ShortcutInput shortcutId="transcribe_with_post_process" grouped />
        <ShortcutActivationSetting descriptionMode="tooltip" grouped />
        {/* Cancel shortcut remains hidden on Linux because of dynamic shortcut instability. */}
        {!isLinux && <ShortcutInput shortcutId="cancel" grouped />}
      </SettingsGroup>
      <SettingsGroup title={t("settings.languages.title")} icon={Languages}>
        <PreferredLanguages descriptionMode="tooltip" grouped />
        {speaksChinese && (
          <ChineseScriptSetting descriptionMode="tooltip" grouped />
        )}
        <AppLanguageSelector descriptionMode="tooltip" grouped />
      </SettingsGroup>
      <SettingsGroup title={t("settings.sound.microphone.title")} icon={Mic}>
        <MicrophoneSelector descriptionMode="tooltip" grouped />
      </SettingsGroup>
      <CardGrid>
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
      </CardGrid>
      <SettingsGroup title={t(g("startup"))} icon={Power}>
        <AutostartToggle descriptionMode="tooltip" grouped />
        <StartHidden descriptionMode="tooltip" grouped />
        <ShowTrayIcon descriptionMode="tooltip" grouped />
      </SettingsGroup>
      <ExpertSection summary={t(g("generalExpert"))}>
        <SettingsGroup>
          <HoldThreshold descriptionMode="inline" grouped />
          <ChannelSelector descriptionMode="inline" grouped />
          <ClamshellMicrophoneSelector descriptionMode="inline" grouped />
        </SettingsGroup>
      </ExpertSection>
    </div>
  );
};
