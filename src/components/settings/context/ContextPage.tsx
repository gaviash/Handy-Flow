import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Inbox, ShieldOff } from "lucide-react";
import { commands, type ContextReport } from "@/bindings";
import {
  CardGrid,
  ExpertSection,
  SettingCard,
  SettingsGroup,
  StatusHeader,
  SubPage,
  ToggleSwitch,
  useSubPage,
} from "@/components/ui";
import { useSettings } from "@/hooks/useSettings";
import { useOsType } from "@/hooks/useOsType";
import { AsrContextSettings } from "../post-processing/AsrContextSettings";
import {
  ContextParts,
  ContextReports,
} from "../post-processing/ContextReports";
import { ExclusionList } from "../post-processing/ContextSettings";

const c = (key: string) => `settings.postProcessing.context.${key}`;
const p = (key: string) => `settings.context.${key}`;

/** What Handy Flow reads around the cursor, what it sends, and where never. */
export const ContextPage: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const osType = useOsType();
  const sub = useSubPage<"exclusions" | "sent">();
  const [reports, setReports] = useState<ContextReport[]>([]);
  useEffect(() => {
    commands.getContextReports().then(setReports);
  }, []);

  const read = getSetting("context_enabled") ?? true;
  const send = getSetting("context_send_enabled") ?? true;
  const apps = (getSetting("context_excluded_apps") ?? []).length;
  const sites = (getSetting("context_excluded_domains") ?? []).length;
  const title = t("sidebar.context");

  if (sub.open === "exclusions")
    return (
      <SubPage
        parent={title}
        title={t(p("exclusions"))}
        description={t(p("exclusionsHelp"))}
        onBack={sub.back}
      >
        <SettingsGroup>
          <ExclusionList
            settingKey="context_excluded_apps"
            label={t(c("excludedApps.label"))}
            description={t(c("excludedApps.description"))}
          />
          <ExclusionList
            settingKey="context_excluded_domains"
            label={t(c("excludedDomains.label"))}
            description={t(c("excludedDomains.description"))}
          />
        </SettingsGroup>
      </SubPage>
    );
  if (sub.open === "sent")
    return (
      <SubPage parent={title} title={t(c("last.label"))} onBack={sub.back}>
        <SettingsGroup>
          <ContextReports />
        </SettingsGroup>
      </SubPage>
    );

  const toggle = (
    key: "context_enabled" | "context_send_enabled",
    label: string,
  ) => (
    <ToggleSwitch
      checked={getSetting(key) ?? true}
      onChange={(on) => updateSetting(key, on)}
      isUpdating={isUpdating(key)}
      label={t(c(`${label}.label`))}
      description={t(c(`${label}.description`))}
      descriptionMode="inline"
      grouped
    />
  );
  const last = reports[0];

  return (
    <div className="mx-auto w-full max-w-3xl space-y-5 pb-4">
      <div className="space-y-2">
        <h1 className="page-title">{title}</h1>
        <p className="text-sm text-ink-2">{t(p("subtitle"))}</p>
      </div>
      <StatusHeader
        tone={read ? "ok" : "off"}
        title={t(
          p(!read ? "statusOff" : !send ? "statusLocal" : "statusSent"),
          { apps, sites },
        )}
      />
      <SettingsGroup>
        {toggle("context_enabled", "read")}
        {toggle("context_send_enabled", "send")}
        <AsrContextSettings part="screen" />
      </SettingsGroup>
      <CardGrid>
        <SettingCard
          ref={sub.cardRef("exclusions")}
          icon={ShieldOff}
          title={t(p("exclusions"))}
          summary={t(p("exclusionsSummary"), { apps, sites })}
          onOpen={() => sub.show("exclusions")}
        />
        <SettingCard
          ref={sub.cardRef("sent")}
          icon={Inbox}
          title={t(c("last.label"))}
          summary={
            last
              ? t(p("lastAt"), {
                  time: new Date(last.at * 1000).toLocaleTimeString(),
                })
              : t(c("last.none"))
          }
          onOpen={() => sub.show("sent")}
        />
      </CardGrid>
      <ExpertSection summary={t(p("expert"))}>
        <SettingsGroup title={t("settings.features.name.Rewrite")}>
          <ContextParts setting="rewrite_context" />
        </SettingsGroup>
        <SettingsGroup title={t("settings.postProcessing.transforms.title")}>
          <ContextParts setting="transform_context" />
        </SettingsGroup>
        <SettingsGroup>
          <AsrContextSettings part="limits" />
          {/* vscode.rs reads %APPDATA%: Windows only. */}
          {osType === "windows" && (
            <ToggleSwitch
              checked={getSetting("vscode_screen_reader") ?? true}
              onChange={(on) => updateSetting("vscode_screen_reader", on)}
              isUpdating={isUpdating("vscode_screen_reader")}
              label={t(c("vscode.label"))}
              description={t(c("vscode.description"))}
              descriptionMode="tooltip"
              grouped
            />
          )}
        </SettingsGroup>
      </ExpertSection>
    </div>
  );
};
