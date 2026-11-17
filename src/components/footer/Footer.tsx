import React, { useState, useEffect } from "react";
import { getVersion } from "@tauri-apps/api/app";

import { useTranslation } from "react-i18next";
import { useSettings } from "@/hooks/useSettings";
import { useModelStore } from "@/stores/modelStore";
import { getTranscriptionConfig } from "@/lib/utils/transcriptionConfig";
import DownloadProgressDisplay from "../model-selector/DownloadProgressDisplay";
import UpdateChecker from "../update-checker";
import NoticeChip from "./NoticeChip";
import TranscriptionStatus from "./TranscriptionStatus";

interface FooterProps {
  onShowHistory: () => void;
  onShowModels: () => void;
}

const Footer: React.FC<FooterProps> = ({ onShowHistory, onShowModels }) => {
  const [version, setVersion] = useState("");
  const { t } = useTranslation();
  const { settings, isLoading, updateChecksLocked } = useSettings();
  const { models, currentModel, downloadProgress, loading } = useModelStore();
  const config = getTranscriptionConfig(settings, models, currentModel);
  const k = (key: string) => `settings.transcription.${key}`;
  const summary = isLoading
    ? t(k("loading"))
    : config.api
      ? config.issue
        ? t(k("footerIncomplete"))
        : t(k("footerApi"), {
            provider: config.providerName,
            model: config.model,
          })
      : t(k("footerLocal"), {
          model: loading
            ? t(k("loading"))
            : config.localModel?.name || t(k("localMissing")),
        });

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
      }
    };

    fetchVersion();
  }, []);

  return (
    <div className="w-full border-t border-line pt-3">
      <div className="flex justify-between items-center text-label px-4 pb-3 text-ink-2">
        <div className="flex min-w-0 items-center gap-4">
          <button
            type="button"
            onClick={onShowModels}
            title={summary}
            className="flex min-w-0 items-center gap-2 rounded text-start hover:text-text focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
          >
            <span
              aria-hidden="true"
              className={`size-2 shrink-0 rounded-full ${config.issue || (!loading && !config.localModel) ? "bg-warning" : "bg-ink-3"}`}
            />
            <span className="max-w-64 truncate">{summary}</span>
          </button>
          <TranscriptionStatus onShowHistory={onShowHistory} />
        </div>

        <div className="flex items-center gap-3">
          {/* Update Status */}
          <div className="flex items-center gap-1">
            {updateChecksLocked === false ? (
              <UpdateChecker />
            ) : (
              <span>{t("footer.appName")}</span>
            )}
            {version && (
              <>
                <span>•</span>
                {/* eslint-disable-next-line i18next/no-literal-string */}
                <span>v{version}</span>
              </>
            )}
          </div>
          <NoticeChip />
        </div>
      </div>
      <DownloadProgressDisplay downloadProgress={downloadProgress} />
    </div>
  );
};

export default Footer;
