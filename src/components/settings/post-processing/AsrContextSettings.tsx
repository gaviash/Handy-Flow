import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type AsrContextSettings as Asr,
  type AsrSupport,
} from "@/bindings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { Input } from "../../ui/Input";
import { useSettings } from "../../../hooks/useSettings";

const DEFAULTS: Asr = {
  enabled: true,
  learned_words: true,
  expressions: true,
  screen: false,
  max_terms: 50,
  capture_wait_ms: 150,
};

/** Handy Adaptive (spec §7): what the speech model is told before it listens. */
export const AsrContextSettings: React.FC<{
  /** Vocabulary shows what is told; Context shows the screen, then its limits. */
  part: "vocabulary" | "screen" | "limits";
}> = ({ part }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [support, setSupport] = useState<AsrSupport | null>(null);
  const [expressions, setExpressions] = useState<string[]>([]);
  useEffect(() => {
    commands.getAsrSupport().then(setSupport);
    commands.getFrequentExpressions().then(setExpressions);
  }, []);
  const asr = getSetting("asr_context") ?? DEFAULTS;
  const busy = isUpdating("asr_context");
  const set = (patch: Partial<Asr>) =>
    updateSetting("asr_context", { ...asr, ...patch });
  const key = (k: string) => `settings.postProcessing.learning.asr.${k}`;
  const status = !support
    ? t(key("noModel"))
    : support.vocabulary || support.prompt
      ? t(key("supported"))
      : t(key("unsupported"));
  const toggle = (field: keyof Asr, label: string, disabled = false) => (
    <ToggleSwitch
      checked={asr[field] as boolean}
      onChange={(v) => set({ [field]: v })}
      isUpdating={busy}
      disabled={disabled}
      label={t(key(label))}
      description={t(key("description"))}
      descriptionMode={part === "screen" ? "inline" : "tooltip"}
      grouped={true}
    />
  );
  const number = (
    field: keyof Asr,
    label: string,
    min: number,
    max: number,
  ) => (
    <SettingContainer
      title={t(key(label))}
      description={t(key("description"))}
      descriptionMode="tooltip"
      layout="horizontal"
      grouped={true}
    >
      <Input
        type="number"
        min={min}
        max={max}
        value={asr[field] as number}
        disabled={busy || !asr.enabled}
        onChange={(e) => {
          const v = Number(e.target.value);
          if (Number.isInteger(v) && v >= min && v <= max) set({ [field]: v });
        }}
        className="w-20"
      />
    </SettingContainer>
  );

  if (part === "screen") return toggle("screen", "screen", !asr.enabled);
  if (part === "limits")
    return (
      <>
        {number("max_terms", "maxTerms", 1, 200)}
        {number("capture_wait_ms", "captureWait", 0, 1000)}
      </>
    );

  return (
    <>
      <SettingContainer
        title={t(key("label"))}
        description={t(key("description"))}
        layout="stacked"
        grouped={true}
      >
        <p className="text-sm text-mid-gray">{status}</p>
      </SettingContainer>
      {toggle("enabled", "enabled")}
      {toggle("learned_words", "learnedWords", !asr.enabled)}
      {toggle("expressions", "expressions", !asr.enabled)}
      <SettingContainer
        title={t(key("expressionsList.label"))}
        description={t(key("expressionsList.description"))}
        layout="stacked"
        grouped={true}
      >
        <details className="w-full border border-line-strong rounded-md bg-wash">
          <summary className="px-2 py-1 text-sm cursor-pointer select-none">
            {t(key("expressionsList.count"), { count: expressions.length })}
          </summary>
          <ul className="px-4 pb-2 text-xs list-disc">
            {expressions.map((e) => (
              <li key={e}>{e}</li>
            ))}
          </ul>
        </details>
      </SettingContainer>
    </>
  );
};
