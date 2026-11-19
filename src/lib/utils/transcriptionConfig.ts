import type { AppSettings, ModelInfo } from "@/bindings";

export function getTranscriptionConfig(
  settings: Pick<
    AppSettings,
    | "features"
    | "provider_models"
    | "post_process_api_keys"
    | "post_process_providers"
  > | null,
  models: Pick<ModelInfo, "id" | "name" | "is_downloaded">[],
  currentModel: string,
) {
  const choice = settings?.features?.transcription_api;
  const id = choice?.provider_id ?? "";
  const model = choice?.model ?? "";
  const provider = settings?.post_process_providers?.find((p) => p.id === id);
  const api = !!id;
  const issue = !api
    ? null
    : !provider ||
        provider.api_format === "anthropic" ||
        id === "apple_intelligence"
      ? "missingProvider"
      : !settings?.provider_models?.[id]?.some(
            (entry) => entry.id === model && entry.speech,
          )
        ? "missingModel"
        : id !== "custom" &&
            !id.startsWith("custom-") &&
            !settings?.post_process_api_keys?.[id]?.trim()
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
