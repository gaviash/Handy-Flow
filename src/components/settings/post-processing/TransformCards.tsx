import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus } from "lucide-react";
import { SettingContainer, SettingsGroup, Textarea } from "@/components/ui";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { ShortcutInput } from "../ShortcutInput";
import { useSettings } from "../../../hooks/useSettings";
import { useOsType } from "../../../hooks/useOsType";
import { formatKeyCombination } from "../../../lib/utils/keyboard";

/** Mirrors adaptive.rs MAX_TRANSFORMS: one shortcut per saved slot. */
const MAX_TRANSFORMS = 9;
/** Polish, the voice instruction, then the saved transformations. */
export const TRANSFORM_IDS = [
  "polish",
  "instruct",
  ...Array.from({ length: MAX_TRANSFORMS }, (_, i) => `transform_${i + 1}`),
];
export type TransformId = string;

const slot = (id: TransformId) =>
  id.startsWith("transform_") ? Number(id.slice("transform_".length)) - 1 : -1;
const b = (id: string, key: string) =>
  `settings.general.shortcut.bindings.${id}.${key}`;

/** A transformation's title and one-line description, as its card shows them. */
function useTransform(id: TransformId) {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const saved = (getSetting("transforms") ?? [])[slot(id)];
  return {
    saved,
    empty: slot(id) >= 0 && !saved?.name.trim(),
    title: saved?.name.trim() || t(b(id, "name")),
    description: saved?.instruction.trim() || t(b(id, "description")),
  };
}

const Keys: React.FC<{ id: TransformId }> = ({ id }) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const osType = useOsType();
  const combo = formatKeyCombination(
    getSetting("bindings")?.[id]?.current_binding ?? "",
    osType,
  );
  if (!combo)
    return (
      <span className="text-xs text-ink-3">
        {t("settings.enhancement.noShortcut")}
      </span>
    );
  return (
    <span className="flex flex-wrap gap-1">
      {combo.split(" + ").map((key) => (
        <kbd
          key={key}
          className="rounded-md border border-line bg-wash px-1.5 py-0.5 font-sans text-xs text-text"
        >
          {key}
        </kbd>
      ))}
    </span>
  );
};

const cardClass =
  "flex min-h-[124px] w-full cursor-pointer flex-col items-start gap-2 rounded-[var(--radius-panel)] border bg-raised p-4 text-start transition-colors duration-150 hover:bg-wash focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary";

const TransformCard = React.forwardRef<
  HTMLButtonElement,
  { id: TransformId; onOpen: () => void }
>(({ id, onOpen }, ref) => {
  const { title, description } = useTransform(id);
  return (
    <button
      ref={ref}
      type="button"
      onClick={onOpen}
      className={`${cardClass} border-line`}
    >
      <Keys id={id} />
      <span className="mt-auto block text-sm font-semibold text-text">
        {title}
      </span>
      <span className="line-clamp-2 text-xs text-ink-2">{description}</span>
    </button>
  );
});
TransformCard.displayName = "TransformCard";

/** The first slot with no saved transformation, or none when all are used. */
export function useFreeSlot(): TransformId | undefined {
  const { getSetting } = useSettings();
  const transforms = getSetting("transforms") ?? [];
  return TRANSFORM_IDS.find(
    (id) => slot(id) >= 0 && !transforms[slot(id)]?.name.trim(),
  );
}

/** One card per transformation, its shortcut on top; "+" fills an empty slot. */
export const TransformCards: React.FC<{
  open: (id: TransformId) => void;
  cardRef: (id: TransformId) => (el: HTMLButtonElement | null) => void;
}> = ({ open, cardRef }) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const transforms = getSetting("transforms") ?? [];
  const shown = TRANSFORM_IDS.filter(
    (id) => slot(id) < 0 || transforms[slot(id)]?.name.trim(),
  );
  const free = useFreeSlot();
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
      {shown.map((id) => (
        <TransformCard
          key={id}
          ref={cardRef(id)}
          id={id}
          onOpen={() => open(id)}
        />
      ))}
      {free && (
        <button
          ref={cardRef(free)}
          type="button"
          onClick={() => open(free)}
          className={`${cardClass} border-dashed border-line-strong`}
        >
          <span className="flex size-7 items-center justify-center rounded-full bg-wash text-ink-2">
            <Plus aria-hidden="true" size={14} />
          </span>
          <span className="mt-auto block text-sm font-semibold text-text">
            {t("settings.enhancement.createOwn")}
          </span>
          <span className="text-xs text-ink-2">
            {t("settings.enhancement.createOwnHelp")}
          </span>
        </button>
      )}
    </div>
  );
};

/** A transformation's sub-page: its shortcut, and for a saved one its name
 * and instruction (saved when the field is left). */
export const TransformEditor: React.FC<{
  id: TransformId;
  onDeleted?: () => void;
}> = ({ id, onDeleted }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const transforms = getSetting("transforms") ?? [];
  const index = slot(id);
  const saved = transforms[index];
  const [name, setName] = useState(saved?.name ?? "");
  const [instruction, setInstruction] = useState(saved?.instruction ?? "");
  useEffect(() => {
    setName(saved?.name ?? "");
    setInstruction(saved?.instruction ?? "");
  }, [saved?.name, saved?.instruction]);
  /** Writes this slot, padding the list when the slot is past its end. */
  const write = (value: { name: string; instruction: string }) => {
    const next = Array.from(
      { length: Math.max(transforms.length, index + 1) },
      (_, i) => transforms[i] ?? { name: "", instruction: "" },
    );
    next[index] = value;
    return updateSetting("transforms", next);
  };
  const save = () =>
    write({ name: name.trim(), instruction: instruction.trim() });
  const p = (key: string) => `settings.postProcessing.transforms.${key}`;
  return (
    <SettingsGroup>
      <ShortcutInput shortcutId={id} descriptionMode="inline" grouped />
      {index >= 0 && (
        <>
          <SettingContainer
            title={t(p("name"))}
            description={t(p("savedDescription"))}
            descriptionMode="tooltip"
            grouped
          >
            <Input
              aria-label={t(p("name"))}
              variant="compact"
              value={name}
              onChange={(e) => setName(e.target.value)}
              onBlur={save}
            />
          </SettingContainer>
          <SettingContainer
            title={t(p("instruction"))}
            description={t(p("savedDescription"))}
            descriptionMode="tooltip"
            layout="stacked"
            grouped
          >
            <Textarea
              aria-label={t(p("instruction"))}
              value={instruction}
              onChange={(e) => setInstruction(e.target.value)}
              onBlur={save}
            />
          </SettingContainer>
          {saved?.name.trim() && (
            <div className="px-4 py-3">
              <Button
                variant="danger-ghost"
                size="sm"
                onClick={() => {
                  write({ name: "", instruction: "" });
                  onDeleted?.();
                }}
              >
                {t("settings.transformsPage.delete")}
              </Button>
            </div>
          )}
        </>
      )}
    </SettingsGroup>
  );
};

export const useTransformTitle = (id: TransformId) => useTransform(id).title;
