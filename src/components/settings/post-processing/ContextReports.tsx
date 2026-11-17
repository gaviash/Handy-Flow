import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  type AsrReport,
  type ContextReport,
  type LlmContextParts,
} from "@/bindings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { useSettings } from "../../../hooks/useSettings";

const k = (key: string) => `settings.postProcessing.context.${key}`;

const PARTS_DEFAULT: LlmContextParts = {
  screen: true,
  learned_words: true,
  style_rules: true,
  past_dictations: true,
};

/** A collapsible block, closed by default. */
const Fold: React.FC<{
  title: string;
  children: React.ReactNode;
  open?: boolean;
}> = ({ title, children, open = false }) => (
  <details
    open={open}
    className="border border-line-strong rounded-md bg-wash [&_summary]:cursor-pointer"
  >
    <summary className="px-2 py-1 text-sm font-medium select-none">
      {title}
    </summary>
    <div className="px-2 pb-2 space-y-1">{children}</div>
  </details>
);

const Pre: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <pre className="text-xs whitespace-pre-wrap break-words max-h-60 overflow-auto">
    {children}
  </pre>
);

/** A labelled value, or the "none" text when it's empty. */
const Field: React.FC<{ label: string; value: string | string[] | null }> = ({
  label,
  value,
}) => {
  const { t } = useTranslation();
  const text = Array.isArray(value) ? value.join(", ") : (value ?? "");
  return (
    <div className="text-xs">
      <span className="text-mid-gray mr-1">{label}</span>
      {text.trim() ? text : t(k("reports.empty"))}
    </div>
  );
};

const AsrPanel: React.FC<{ asr: AsrReport | null }> = ({ asr }) => {
  const { t } = useTranslation();
  if (!asr) return <Pre>{t(k("reports.asr.none"))}</Pre>;
  const note =
    !asr.accepts_vocabulary && !asr.accepts_prompt
      ? t(k("reports.asr.unsupported"))
      : !asr.enabled
        ? t(k("reports.asr.disabled"))
        : null;
  return (
    <>
      <Field label={t(k("reports.asr.model"))} value={asr.model} />
      {note ? (
        <Pre>{note}</Pre>
      ) : (
        <>
          <Field
            label={t(k("reports.asr.vocabulary"))}
            value={asr.vocabulary}
          />
          <Field
            label={t(k("reports.asr.expressions"))}
            value={asr.expressions}
          />
          <Field label={t(k("reports.asr.app"))} value={asr.app} />
          <Field label={t(k("reports.asr.before"))} value={asr.before} />
          {asr.screen_waited === false && (
            <div className="text-xs">{t(k("reports.asr.screenLate"))}</div>
          )}
          {asr.cloud_error && (
            <div className="text-xs">
              {t(k("reports.asr.cloudError"), { error: asr.cloud_error })}
            </div>
          )}
          {asr.echo_dropped && (
            <div className="text-xs">{t(k("reports.asr.echo"))}</div>
          )}
          <Fold title={t(k("reports.asr.prompt"))}>
            <Pre>{asr.prompt ?? t(k("reports.empty"))}</Pre>
          </Fold>
        </>
      )}
    </>
  );
};

const ReportPanel: React.FC<{ report: ContextReport; open: boolean }> = ({
  report,
  open,
}) => {
  const { t } = useTranslation();
  const capture = report.capture;
  const time = new Date(report.at * 1000).toLocaleTimeString();
  const target = capture
    ? [capture.process, capture.host].filter(Boolean).join(" · ")
    : "";
  const llm = !capture
    ? t(k("last.none"))
    : capture.sensitive
      ? t(k("last.sensitive"))
      : capture.excluded
        ? t(k("last.excluded"))
        : (report.sent ?? t(k("last.nothingSent")));
  const isDictation =
    report.kind === "dictation" || report.kind === "instruction";
  return (
    <Fold
      open={open}
      title={`${time} · ${t(k(`reports.kind.${report.kind}`))}${target ? ` · ${target}` : ""}`}
    >
      {isDictation && (
        <Fold title={t(k("reports.asr.title"))}>
          <AsrPanel asr={report.asr} />
        </Fold>
      )}
      <Fold title={t(k("reports.llm.title"))}>
        <Pre>{llm}</Pre>
      </Fold>
    </Fold>
  );
};

/** F114: what the last dictations and transformations sent, one panel each. */
export const ContextReports: React.FC = () => {
  const { t } = useTranslation();
  const [reports, setReports] = useState<ContextReport[]>([]);
  const refresh = () => commands.getContextReports().then(setReports);
  useEffect(() => {
    refresh();
  }, []);
  return (
    <SettingContainer
      title={t(k("last.label"))}
      description={t(k("last.description"))}
      layout="stacked"
      grouped={true}
    >
      <div className="w-full space-y-2">
        <div className="flex justify-end">
          <Button variant="secondary" size="sm" onClick={refresh}>
            {t(k("last.refresh"))}
          </Button>
        </div>
        {reports.length === 0 ? (
          <Pre>{t(k("last.none"))}</Pre>
        ) : (
          reports.map((r, i) => (
            <ReportPanel key={`${r.at}-${i}`} report={r} open={i === 0} />
          ))
        )}
      </div>
    </SettingContainer>
  );
};

/** Which parts of the context one LLM use gets (rewrite or transformations). */
export const ContextParts: React.FC<{
  setting: "rewrite_context" | "transform_context";
}> = ({ setting }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const parts = getSetting(setting) ?? PARTS_DEFAULT;
  const name = setting === "rewrite_context" ? "rewrite" : "transform";
  const toggle = (field: keyof LlmContextParts, label: string) => (
    <ToggleSwitch
      key={field}
      checked={parts[field]}
      onChange={(v) => updateSetting(setting, { ...parts, [field]: v })}
      isUpdating={isUpdating(setting)}
      label={t(k(`parts.${name}.${label}`))}
      description={t(k(`parts.${label}Description`))}
      descriptionMode="tooltip"
      grouped={true}
    />
  );
  return (
    <>
      {toggle("screen", "screen")}
      {toggle("learned_words", "learnedWords")}
      {toggle("style_rules", "styleRules")}
      {toggle("past_dictations", "pastDictations")}
    </>
  );
};
