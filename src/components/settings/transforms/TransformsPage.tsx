import React from "react";
import { useTranslation } from "react-i18next";
import { Plus } from "lucide-react";
import { StatusHeader, SubPage, useSubPage } from "@/components/ui";
import { Button } from "../../ui/Button";
import { useFeatureStatuses } from "../models/FeatureModelsCard";
import {
  TransformCards,
  TransformEditor,
  useFreeSlot,
  useTransformTitle,
  type TransformId,
} from "../post-processing/TransformCards";

const k = (key: string) => `settings.transformsPage.${key}`;

/** Shortcuts that rewrite the selected text: Polish, the voice instruction
 * and the saved transformations, one card each. */
export const TransformsPage: React.FC<{ onShowModels?: () => void }> = ({
  onShowModels,
}) => {
  const { t } = useTranslation();
  const sub = useSubPage<TransformId>();
  const editing = useTransformTitle(sub.open ?? "polish");
  // They use the rewrite's model, whether or not dictation is enhanced.
  const rewrite = useFeatureStatuses().find((s) => s.feature === "Rewrite");
  const ready =
    !!rewrite?.provider_label &&
    (!rewrite.problem || rewrite.problem === "RewriteOff");
  const title = t("sidebar.transforms");
  const free = useFreeSlot();

  if (sub.open)
    return (
      <SubPage parent={title} title={editing} onBack={sub.back}>
        <TransformEditor id={sub.open} onDeleted={sub.back} />
      </SubPage>
    );

  return (
    <div className="mx-auto w-full max-w-3xl space-y-5 pb-4">
      <div className="flex items-start justify-between gap-4">
        <div className="space-y-2">
          <h1 className="page-title">{title}</h1>
          <p className="text-sm text-ink-2">{t(k("subtitle"))}</p>
        </div>
        <Button
          className="flex shrink-0 items-center gap-1.5"
          disabled={!free}
          title={free ? undefined : t(k("full"))}
          onClick={() => free && sub.show(free)}
        >
          <Plus aria-hidden="true" size={14} />
          {t(k("create"))}
        </Button>
      </div>
      <StatusHeader
        tone={ready ? "ok" : "warn"}
        title={
          ready
            ? t(k("ready"), {
                provider: rewrite?.provider_label,
                model: rewrite?.model,
              })
            : t(k("setup"))
        }
      >
        {!ready && (
          <Button size="sm" variant="outline" onClick={onShowModels}>
            {t("settings.enhancement.openModels")}
          </Button>
        )}
      </StatusHeader>
      <TransformCards open={sub.show} cardRef={sub.cardRef} />
    </div>
  );
};
