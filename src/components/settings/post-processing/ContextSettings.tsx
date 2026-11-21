import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../../ui/SettingContainer";
import { Textarea } from "../../ui/Textarea";
import { Input } from "../../ui/Input";
import { useSettings } from "../../../hooks/useSettings";

type ListKey = "context_excluded_apps" | "context_excluded_domains";

export const ExclusionList: React.FC<{
  settingKey: ListKey;
  label: string;
  description: string;
}> = ({ settingKey, label, description }) => {
  const { getSetting, updateSetting } = useSettings();
  const saved = (getSetting(settingKey) ?? []).join("\n");
  const [text, setText] = useState(saved);
  useEffect(() => setText(saved), [saved]);

  return (
    <SettingContainer
      title={label}
      description={description}
      layout="stacked"
      grouped={true}
    >
      <Textarea
        className="w-full"
        variant="compact"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onBlur={() =>
          updateSetting(
            settingKey,
            text
              .split(/[\n,]/)
              .map((s) => s.trim())
              .filter(Boolean),
          )
        }
      />
    </SettingContainer>
  );
};

/** Handy Adaptive (F63): the user's name, always spelled right. */
export const UserName: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const saved = getSetting("user_name") ?? "";
  const [name, setName] = useState(saved);
  useEffect(() => setName(saved), [saved]);
  return (
    <SettingContainer
      title={t("settings.postProcessing.context.userName.label")}
      description={t("settings.postProcessing.context.userName.description")}
      descriptionMode="tooltip"
      grouped={true}
    >
      <Input
        variant="compact"
        value={name}
        onChange={(e) => setName(e.target.value)}
        onBlur={() => updateSetting("user_name", name)}
      />
    </SettingContainer>
  );
};
