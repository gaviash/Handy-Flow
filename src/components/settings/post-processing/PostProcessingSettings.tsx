import React from "react";
import { useTranslation } from "react-i18next";
import {
  Eraser,
  FileText,
  Keyboard,
  MessageCircle,
  Server,
  Wand2,
} from "lucide-react";
import type { FeatureStatus } from "@/bindings";
import {
  CardGrid,
  ExpertSection,
  SettingCard,
  SettingsGroup,
  StatusHeader,
  SubPage,
  useSubPage,
} from "@/components/ui";
import { Button } from "../../ui/Button";
import { Switch } from "../../ui/Switch";
import { useSettings } from "../../../hooks/useSettings";
import { useOsType } from "../../../hooks/useOsType";
import { formatKeyCombination } from "../../../lib/utils/keyboard";
import { FillerWordRemoval } from "../FillerWordRemoval";
import { useFeatureStatuses } from "../models/FeatureModelsCard";
import { ShortcutInput } from "../ShortcutInput";
import { PostProcessTimeout } from "./PostProcessTimeout";
import { PromptEditor } from "./PromptEditor";
import { StyleSettings, CATEGORIES } from "./StyleSettings";
import { TransformSettings } from "./TransformSettings";

type Topic =
  | "provider"
  | "tone"
  | "transforms"
  | "cleanup"
  | "shortcut"
  | "prompt";

const e = (key: string) => `settings.enhancement.${key}`;
const pp = (key: string) => `settings.postProcessing.${key}`;

/** "Groq · llama", or null when the feature has nothing it can use. */
const uses = (status?: FeatureStatus) =>
  status && !status.problem
    ? `${status.provider_label} · ${status.model}`
    : null;

/** The rewrite: whether it runs, with what, in which tone. */
export const PostProcessingSettings: React.FC<{
  onShowModels?: () => void;
  onShowGeneral?: () => void;
}> = ({ onShowModels, onShowGeneral }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const osType = useOsType();
  const sub = useSubPage<Topic>();
  const statuses = useFeatureStatuses();
  const rewrite = statuses.find((s) => s.feature === "Rewrite");
  const fallback = statuses.find((s) => s.feature === "RewriteFallback");

  const enabled = getSetting("post_process_enabled") ?? false;
  const shortcut =
    formatKeyCombination(
      getSetting("bindings")?.transcribe_with_post_process?.current_binding ??
        "",
      osType,
    ) || t("settings.general.shortcut.none");
  const provider = uses(rewrite);
  const backup = uses(fallback) ?? t(e("none"));
  const styles = getSetting("styles");
  const transforms = (getSetting("transforms") ?? []).filter((x) =>
    x.name.trim(),
  ).length;
  const title = t("sidebar.postProcessing");

  const page = (topic: Topic, heading: string, body: React.ReactNode) =>
    sub.open === topic && (
      <SubPage parent={title} title={heading} onBack={sub.back}>
        {body}
      </SubPage>
    );
  const open =
    page(
      "provider",
      t(e("provider")),
      <SettingsGroup>
        <div className="px-4 py-3 text-sm space-y-1">
          <p>
            {t(e("rewriteUses"), {
              value: provider ?? t(e("notConfigured")),
            })}
          </p>
          <p>{t(e("fallbackUses"), { value: backup })}</p>
          <p className="pt-2 text-ink-2">{t(e("providerHelp"))}</p>
        </div>
        <div className="px-4 pb-3">
          <Button size="sm" variant="outline" onClick={onShowModels}>
            {t("settings.features.changeInModels")}
          </Button>
        </div>
      </SettingsGroup>,
    ) ||
    page(
      "tone",
      t(pp("styles.title")),
      <SettingsGroup>
        <StyleSettings />
      </SettingsGroup>,
    ) ||
    page(
      "transforms",
      t(pp("transforms.title")),
      <SettingsGroup>
        <TransformSettings />
      </SettingsGroup>,
    ) ||
    page(
      "cleanup",
      t(e("cleanup")),
      <SettingsGroup>
        <FillerWordRemoval descriptionMode="inline" grouped />
      </SettingsGroup>,
    ) ||
    page(
      "shortcut",
      t(pp("hotkey.title")),
      <>
        <SettingsGroup>
          <ShortcutInput
            shortcutId="transcribe_with_post_process"
            descriptionMode="inline"
            grouped
          />
        </SettingsGroup>
        <Button size="sm" variant="outline" onClick={onShowGeneral}>
          {t(e("allShortcuts"))}
        </Button>
      </>,
    ) ||
    page(
      "prompt",
      t(pp("prompts.title")),
      <SettingsGroup>
        <PromptEditor />
      </SettingsGroup>,
    );
  if (open) return open;

  const ready = enabled && !!provider;
  return (
    <div className="mx-auto w-full max-w-3xl space-y-5 pb-4">
      <div className="flex items-start justify-between gap-4">
        <div className="space-y-2">
          <h1 className="page-title">{title}</h1>
          <p className="text-sm text-ink-2">{t(e("subtitle"))}</p>
        </div>
        <Switch
          checked={enabled}
          onChange={(on) => updateSetting("post_process_enabled", on)}
          disabled={isUpdating("post_process_enabled")}
          ariaLabel={t("settings.debug.postProcessingToggle.label")}
        />
      </div>
      <StatusHeader
        tone={!enabled ? "off" : ready ? "ok" : "warn"}
        title={
          !enabled
            ? t(e("statusOff"))
            : ready
              ? t(e("statusReady"), {
                  shortcut,
                  provider: rewrite?.provider_label,
                })
              : t(e("statusSetup"))
        }
        detail={
          ready &&
          t(e("statusDetail"), { model: rewrite?.model, fallback: backup })
        }
      >
        {enabled && !ready && (
          <Button size="sm" variant="outline" onClick={onShowModels}>
            {t(e("openModels"))}
          </Button>
        )}
      </StatusHeader>
      <CardGrid>
        <SettingCard
          ref={sub.cardRef("provider")}
          icon={Server}
          title={t(e("provider"))}
          summary={
            provider
              ? uses(fallback)
                ? `${provider} · ${t(e("fallbackShort"), { value: backup })}`
                : provider
              : t(e("notConfigured"))
          }
          tone={provider ? undefined : "warn"}
          onOpen={() => sub.show("provider")}
        />
        <SettingCard
          ref={sub.cardRef("tone")}
          icon={MessageCircle}
          title={t(pp("styles.title"))}
          summary={CATEGORIES.map(
            (c) =>
              `${t(pp(`styles.categories.${c}`))} ${t(
                pp(`styles.tones.${styles?.[c] ?? "default"}`),
              ).toLowerCase()}`,
          ).join(" · ")}
          onOpen={() => sub.show("tone")}
        />
        <SettingCard
          ref={sub.cardRef("transforms")}
          icon={Wand2}
          title={t(pp("transforms.title"))}
          summary={t(e("transformsSummary"), { count: transforms })}
          onOpen={() => sub.show("transforms")}
        />
        <SettingCard
          ref={sub.cardRef("cleanup")}
          icon={Eraser}
          title={t(e("cleanup"))}
          summary={t(
            e(
              (getSetting("filler_word_removal_enabled") ?? true)
                ? "fillersRemoved"
                : "fillersKept",
            ),
          )}
          onOpen={() => sub.show("cleanup")}
        />
        <SettingCard
          ref={sub.cardRef("shortcut")}
          icon={Keyboard}
          title={t(pp("hotkey.title"))}
          summary={shortcut}
          onOpen={() => sub.show("shortcut")}
        />
      </CardGrid>
      <ExpertSection summary={t(e("expert"))}>
        <SettingsGroup>
          <PostProcessTimeout />
        </SettingsGroup>
        <SettingCard
          ref={sub.cardRef("prompt")}
          icon={FileText}
          title={t(pp("prompts.title"))}
          summary={
            (getSetting("post_process_prompts") ?? []).find(
              (p) => p.id === getSetting("post_process_selected_prompt_id"),
            )?.name ?? ""
          }
          onOpen={() => sub.show("prompt")}
        />
      </ExpertSection>
    </div>
  );
};
