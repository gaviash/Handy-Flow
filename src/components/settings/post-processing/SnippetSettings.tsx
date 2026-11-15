import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Snippet } from "@/bindings";
import { Input } from "../../ui/Input";
import { Textarea } from "../../ui/Textarea";
import { Button } from "../../ui/Button";
import { SettingContainer } from "../../ui/SettingContainer";
import { useSettings } from "../../../hooks/useSettings";

/** Handy Adaptive (F120): a trigger said on its own inserts its text. */
export const SnippetSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const saved = getSetting("snippets") ?? [];
  const [rows, setRows] = useState<Snippet[]>(saved);
  useEffect(() => setRows(saved), [JSON.stringify(saved)]);

  const edit = (i: number, change: Partial<Snippet>) =>
    setRows(rows.map((r, j) => (j === i ? { ...r, ...change } : r)));
  const commit = (next: Snippet[] = rows) => updateSetting("snippets", next);

  return (
    <SettingContainer
      title={t("settings.postProcessing.snippets.label")}
      description={t("settings.postProcessing.snippets.description")}
      layout="stacked"
      grouped={true}
    >
      <div className="w-full space-y-3">
        {rows.map((r, i) => (
          <div key={i} className="flex gap-2 items-start">
            <Input
              className="w-40"
              variant="compact"
              value={r.trigger}
              placeholder={t("settings.postProcessing.snippets.trigger")}
              onChange={(e) => edit(i, { trigger: e.target.value })}
              onBlur={() => commit()}
            />
            <Textarea
              className="flex-1"
              variant="compact"
              value={r.text}
              placeholder={t("settings.postProcessing.snippets.text")}
              onChange={(e) => edit(i, { text: e.target.value })}
              onBlur={() => commit()}
            />
            <Button
              variant="secondary"
              size="sm"
              onClick={() => {
                const next = rows.filter((_, j) => j !== i);
                setRows(next);
                commit(next);
              }}
            >
              {t("settings.postProcessing.learning.words.delete")}
            </Button>
          </div>
        ))}
        <Button
          variant="secondary"
          size="sm"
          onClick={() => setRows([...rows, { trigger: "", text: "" }])}
        >
          {t("settings.postProcessing.snippets.add")}
        </Button>
      </div>
    </SettingContainer>
  );
};
