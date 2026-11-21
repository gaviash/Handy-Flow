import React, { useEffect, useId, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { commands } from "@/bindings";
import { notify } from "@/stores/noticeStore";
import { Dropdown, SettingContainer, Textarea } from "@/components/ui";
import { Button } from "../../ui/Button";
import { Input } from "../../ui/Input";
import { useSettings } from "../../../hooks/useSettings";

const k = (key: string) => `settings.postProcessing.prompts.${key}`;

/** The prompt the rewrite follows: choose, edit, create or delete one. */
export const PromptEditor: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const id = useId();
  const [isCreating, setIsCreating] = useState(false);
  const [draftName, setDraftName] = useState("");
  const [draftText, setDraftText] = useState("");

  const prompts = getSetting("post_process_prompts") || [];
  const selectedPromptId = getSetting("post_process_selected_prompt_id") || "";
  const selectedPrompt =
    prompts.find((prompt) => prompt.id === selectedPromptId) || null;

  const reset = () => {
    setDraftName(selectedPrompt?.name ?? "");
    setDraftText(selectedPrompt?.prompt ?? "");
  };
  useEffect(() => {
    if (!isCreating) reset();
  }, [
    isCreating,
    selectedPromptId,
    selectedPrompt?.name,
    selectedPrompt?.prompt,
  ]);

  /** Runs a prompt command; a failure is said, never swallowed. */
  const run = async (
    command: () => Promise<
      { status: "ok"; data: unknown } | { status: "error"; error: string }
    >,
    then?: (data: unknown) => void,
  ) => {
    try {
      const result = await command();
      if (result.status === "error") throw result.error;
      await refreshSettings();
      then?.(result.data);
    } catch (error) {
      console.error("Prompt command failed:", error);
      notify({ title: t(k("saveError")), detail: String(error) });
    }
  };

  const name = draftName.trim();
  const text = draftText.trim();
  // Handy Adaptive: the house prompt is code-owned and re-synced on launch.
  const isHouse = !isCreating && selectedPromptId === "adaptive_house";
  const isDirty =
    !!selectedPrompt &&
    (name !== selectedPrompt.name || text !== selectedPrompt.prompt.trim());
  const editing = isCreating || !!selectedPrompt;

  return (
    <SettingContainer
      title={t(k("selectedPrompt.title"))}
      description={t(k("selectedPrompt.description"))}
      descriptionMode="inline"
      layout="stacked"
      grouped={true}
    >
      <div className="space-y-3">
        <div className="flex gap-2 min-w-0">
          <Dropdown
            selectedValue={selectedPromptId || null}
            options={prompts.map((p) => ({ value: p.id, label: p.name }))}
            onSelect={(value) => {
              if (!value) return;
              updateSetting("post_process_selected_prompt_id", value);
              setIsCreating(false);
            }}
            placeholder={t(k(prompts.length ? "selectPrompt" : "noPrompts"))}
            disabled={
              isUpdating("post_process_selected_prompt_id") || isCreating
            }
            className="flex-1 min-w-0"
          />
          <Button
            onClick={() => {
              setIsCreating(true);
              setDraftName("");
              setDraftText("");
            }}
            variant="primary"
            size="md"
            disabled={isCreating}
            className="shrink-0"
          >
            {t(k("createNew"))}
          </Button>
        </div>

        {!editing && (
          <div className="p-3 bg-wash rounded-md border border-line">
            <p className="text-sm text-ink-3">
              {t(k(prompts.length ? "selectToEdit" : "createFirst"))}
            </p>
          </div>
        )}

        {editing && (
          <div className="space-y-3">
            <div className="space-y-2 flex flex-col">
              <label htmlFor={`${id}-name`} className="text-sm font-semibold">
                {t(k("promptLabel"))}
              </label>
              <Input
                id={`${id}-name`}
                type="text"
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                readOnly={isHouse}
                placeholder={t(k("promptLabelPlaceholder"))}
                variant="compact"
              />
            </div>
            <div className="space-y-2 flex flex-col">
              <label htmlFor={`${id}-text`} className="text-sm font-semibold">
                {t(k("promptInstructions"))}
              </label>
              <Textarea
                id={`${id}-text`}
                value={draftText}
                onChange={(e) => setDraftText(e.target.value)}
                readOnly={isHouse}
                placeholder={t(k("promptInstructionsPlaceholder"))}
              />
              <p className="text-xs text-ink-3">
                <Trans
                  i18nKey={k("promptTip")}
                  components={{ code: <code /> }}
                />
              </p>
            </div>
            <div className="flex gap-2 pt-2">
              {isCreating ? (
                <>
                  <Button
                    onClick={() =>
                      run(
                        () => commands.addPostProcessPrompt(name, text),
                        (data) => {
                          updateSetting(
                            "post_process_selected_prompt_id",
                            (data as { id: string }).id,
                          );
                          setIsCreating(false);
                        },
                      )
                    }
                    variant="primary"
                    size="md"
                    disabled={!name || !text}
                  >
                    {t(k("createPrompt"))}
                  </Button>
                  <Button
                    onClick={() => setIsCreating(false)}
                    variant="secondary"
                    size="md"
                  >
                    {t(k("cancel"))}
                  </Button>
                </>
              ) : (
                <>
                  <Button
                    onClick={() =>
                      run(() =>
                        commands.updatePostProcessPrompt(
                          selectedPromptId,
                          name,
                          text,
                        ),
                      )
                    }
                    variant="primary"
                    size="md"
                    disabled={isHouse || !name || !text || !isDirty}
                  >
                    {t(k("updatePrompt"))}
                  </Button>
                  <Button
                    onClick={() =>
                      run(() =>
                        commands.deletePostProcessPrompt(selectedPromptId),
                      )
                    }
                    variant="secondary"
                    size="md"
                    disabled={isHouse || prompts.length <= 1}
                  >
                    {t(k("deletePrompt"))}
                  </Button>
                </>
              )}
            </div>
          </div>
        )}
      </div>
    </SettingContainer>
  );
};
