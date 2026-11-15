import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Transform } from "@/bindings";
import { ShortcutInput } from "../ShortcutInput";
import { Input } from "../../ui/Input";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";

/** Handy Adaptive (F121–F123): Polish, voice instruction, saved transformations. */
export const TransformSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const saved = getSetting("transforms") ?? [];
  const [rows, setRows] = useState<Transform[]>(saved);
  useEffect(() => setRows(saved), [JSON.stringify(saved)]);
  const edit = (i: number, change: Partial<Transform>) =>
    setRows(rows.map((r, j) => (j === i ? { ...r, ...change } : r)));

  return (
    <>
      <ShortcutInput
        shortcutId="polish"
        descriptionMode="tooltip"
        grouped={true}
      />
      <ShortcutInput
        shortcutId="instruct"
        descriptionMode="tooltip"
        grouped={true}
      />
      {[1, 2, 3].map((n) => (
        <ShortcutInput
          key={n}
          shortcutId={`transform_${n}`}
          descriptionMode="tooltip"
          grouped={true}
        />
      ))}
      <SettingContainer
        title={t("settings.postProcessing.transforms.saved")}
        description={t("settings.postProcessing.transforms.savedDescription")}
        layout="stacked"
        grouped={true}
      >
        <div className="w-full space-y-2">
          {rows.map((r, i) => (
            <div key={i} className="flex gap-2 items-center">
              <span className="text-sm text-mid-gray w-4">{i + 1}</span>
              <Input
                className="w-40"
                variant="compact"
                value={r.name}
                placeholder={t("settings.postProcessing.transforms.name")}
                onChange={(e) => edit(i, { name: e.target.value })}
                onBlur={() => updateSetting("transforms", rows)}
              />
              <Input
                className="flex-1"
                variant="compact"
                value={r.instruction}
                placeholder={t(
                  "settings.postProcessing.transforms.instruction",
                )}
                onChange={(e) => edit(i, { instruction: e.target.value })}
                onBlur={() => updateSetting("transforms", rows)}
              />
            </div>
          ))}
        </div>
      </SettingContainer>
    </>
  );
};
