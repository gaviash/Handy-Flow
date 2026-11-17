import type {
  CloudAsrSettings,
  ModelInfo,
  PostProcessProvider,
} from "@/bindings";

export function getTranscriptionConfig(
  settings: {
    cloud_asr?: CloudAsrSettings;
    post_process_api_keys?: Partial<Record<string, string>>;
    post_process_providers?: Pick<PostProcessProvider, "id" | "label">[];
  } | null,
  models: Pick<ModelInfo, "id" | "name" | "is_downloaded">[],
  currentModel: string,
) {
  const id = settings?.cloud_asr?.provider_id ?? "";
  const provider = settings?.post_process_providers?.find((p) => p.id === id);
  const model = settings?.cloud_asr?.models[id]?.trim() ?? "";
  const api = !!id;
  const issue = !api
    ? null
    : !provider || !["groq", "openai", "custom"].includes(id)
      ? "missingProvider"
      : !model
        ? "missingModel"
        : id !== "custom" && !settings?.post_process_api_keys?.[id]?.trim()
          ? "missingKey"
          : null;
  return {
    api,
    providerName: provider?.label ?? id,
    model,
    issue,
    localModel: models.find((m) => m.id === currentModel && m.is_downloaded),
  };
}
