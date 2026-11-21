import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { RefreshCw, Stethoscope } from "lucide-react";
import {
  CardGrid,
  ExpertSection,
  SettingCard,
  SettingsGroup,
  SubPage,
  useSubPage,
} from "@/components/ui";
import { useSettings } from "@/hooks/useSettings";
import { UpdateChecksToggle } from "../UpdateChecksToggle";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { AppDataDirectory } from "../AppDataDirectory";
import { ShowWhatsNewOnUpdate } from "../ShowWhatsNewOnUpdate";
import { LogDirectory } from "../debug";
import HandyFlowWordmark from "../../icons/HandyFlowWordmark";
import roseIcon from "../../../../src-tauri/resources/icons/palette-rose.png";

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");
  const { getSetting } = useSettings();
  const sub = useSubPage<"updates" | "diagnostic">();

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("");
      }
    };

    fetchVersion();
  }, []);

  const title = t("sidebar.about");
  if (sub.open === "updates")
    return (
      <SubPage
        parent={title}
        title={t("settings.about.updates")}
        onBack={sub.back}
      >
        <SettingsGroup>
          <ShowWhatsNewOnUpdate descriptionMode="inline" grouped />
          <UpdateChecksToggle descriptionMode="inline" grouped />
        </SettingsGroup>
      </SubPage>
    );
  if (sub.open === "diagnostic")
    return (
      <SubPage
        parent={title}
        title={t("settings.about.diagnostic")}
        onBack={sub.back}
      >
        <SettingsGroup>
          <AppDataDirectory descriptionMode="inline" grouped />
        </SettingsGroup>
        <ExpertSection summary={t("settings.about.logsExpert")}>
          <SettingsGroup>
            <LogDirectory grouped />
          </SettingsGroup>
        </ExpertSection>
      </SubPage>
    );

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <header className="flex items-center gap-4">
        <img src={roseIcon} alt="" width={56} height={56} />
        <div>
          <HandyFlowWordmark size={30} />
          <p className="mt-2 text-sm text-ink-2">{t("about.forkOf")}</p>
        </div>
      </header>
      <SettingsGroup title={t("settings.about.title")}>
        <SettingContainer
          title={t("settings.about.version.title")}
          description={t("settings.about.version.description")}
          grouped={true}
        >
          <span className="text-sm font-mono">
            {version ? `v${version}` : t("settings.about.version.unavailable")}
          </span>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.sourceCode.title")}
          description={t("settings.about.sourceCode.description")}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() => openUrl("https://github.com/gaviash/Handy-Flow")}
          >
            {t("settings.about.sourceCode.button")}
          </Button>
        </SettingContainer>
      </SettingsGroup>
      <CardGrid>
        <SettingCard
          ref={sub.cardRef("updates")}
          icon={RefreshCw}
          title={t("settings.about.updates")}
          summary={t(
            (getSetting("update_checks_enabled") ?? true)
              ? "settings.about.updatesOn"
              : "settings.about.updatesOff",
          )}
          onOpen={() => sub.show("updates")}
        />
        <SettingCard
          ref={sub.cardRef("diagnostic")}
          icon={Stethoscope}
          title={t("settings.about.diagnostic")}
          summary={t("settings.about.diagnosticSummary")}
          onOpen={() => sub.show("diagnostic")}
        />
      </CardGrid>

      <SettingsGroup title={t("settings.about.acknowledgments.title")}>
        <SettingContainer
          title={t("settings.about.acknowledgments.ggml.title")}
          description={t("settings.about.acknowledgments.ggml.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-ink-3">
            {t("settings.about.acknowledgments.ggml.details")}
          </div>
        </SettingContainer>
      </SettingsGroup>
    </div>
  );
};
