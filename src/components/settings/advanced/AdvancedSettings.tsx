import React from "react";
import { useTranslation } from "react-i18next";
import { ClipboardPaste } from "lucide-react";
import {
  CardGrid,
  ExpertSection,
  SettingCard,
  SettingsGroup,
  SubPage,
  useSubPage,
} from "@/components/ui";
import { PasteMethodSetting } from "../PasteMethod";
import { TypingToolSetting } from "../TypingTool";
import { ClipboardHandlingSetting } from "../ClipboardHandling";
import { AutoSubmit } from "../AutoSubmit";
import { AppendTrailingSpace } from "../AppendTrailingSpace";
import { useSettings } from "../../../hooks/useSettings";
import { useOsType } from "../../../hooks/useOsType";
import { VoiceActivityDetection } from "../VoiceActivityDetection";
import { LazyStreamClose } from "../LazyStreamClose";
import { AirPodsMode } from "../AirPodsMode";
import { VadBackendSelector } from "../VadBackendSelector";
import { AlwaysOnMicrophone } from "../AlwaysOnMicrophone";
import { PasteDelay } from "../debug/PasteDelay";
import { ReliablePasteToggle } from "../debug/ReliablePaste";
import { RecordingBuffer } from "../debug/RecordingBuffer";

const PASTE_LABEL: Record<string, string> = {
  ctrl_v: "clipboard",
  ctrl_shift_v: "clipboardCtrlShiftV",
  shift_insert: "clipboardShiftInsert",
  direct: "direct",
  none: "none",
  external_script: "externalScript",
};

/** How the text reaches the app, and how the microphone is captured. */
export const AdvancedSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const osType = useOsType();
  const sub = useSubPage<"insertion">();
  const title = t("sidebar.advanced");
  const a = (key: string) => `settings.advanced.${key}`;

  if (sub.open === "insertion")
    return (
      <SubPage parent={title} title={t(a("insertion"))} onBack={sub.back}>
        <SettingsGroup>
          <PasteMethodSetting descriptionMode="inline" grouped />
          <TypingToolSetting descriptionMode="inline" grouped />
          <ClipboardHandlingSetting descriptionMode="inline" grouped />
          <AppendTrailingSpace descriptionMode="inline" grouped />
          <AutoSubmit descriptionMode="inline" grouped />
        </SettingsGroup>
        <ExpertSection summary={t(a("insertionExpert"))}>
          <SettingsGroup>
            <PasteDelay descriptionMode="inline" grouped />
            <PasteDelay
              descriptionMode="inline"
              grouped
              settingKey="paste_delay_after_ms"
              labelKey="settings.debug.pasteDelayAfter.title"
              descriptionKey="settings.debug.pasteDelayAfter.description"
            />
            <ReliablePasteToggle descriptionMode="inline" grouped />
          </SettingsGroup>
        </ExpertSection>
      </SubPage>
    );

  const method = getSetting("paste_method") || "ctrl_v";
  return (
    <div className="max-w-3xl w-full mx-auto space-y-5 pb-4">
      <h1 className="page-title">{title}</h1>
      <CardGrid>
        <SettingCard
          ref={sub.cardRef("insertion")}
          icon={ClipboardPaste}
          title={t(a("insertion"))}
          summary={[
            t(a(`pasteMethod.options.${PASTE_LABEL[method] ?? "clipboard"}`), {
              modifier: osType === "macos" ? "Cmd" : "Ctrl",
            }),
            t(
              a(
                (getSetting("append_trailing_space") ?? false)
                  ? "trailingSpaceOn"
                  : "trailingSpaceOff",
              ),
            ),
          ].join(" · ")}
          onOpen={() => sub.show("insertion")}
        />
      </CardGrid>
      <ExpertSection summary={t(a("captureExpert"))}>
        <SettingsGroup>
          <VoiceActivityDetection descriptionMode="inline" grouped />
          <VadBackendSelector descriptionMode="inline" grouped />
          <LazyStreamClose descriptionMode="inline" grouped />
          <AirPodsMode descriptionMode="inline" grouped />
          <AlwaysOnMicrophone descriptionMode="inline" grouped />
          <RecordingBuffer descriptionMode="inline" grouped />
        </SettingsGroup>
      </ExpertSection>
    </div>
  );
};
