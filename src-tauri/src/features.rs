use crate::settings::{
    AppSettings, PostProcessApiFormat, PostProcessProvider, APPLE_INTELLIGENCE_PROVIDER_ID,
};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq, Type)]
pub enum Reasoning {
    #[default]
    Off,
    Low,
    Medium,
    High,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, Type)]
pub struct FeatureModel {
    pub provider_id: String,
    pub model: String,
    #[serde(default)]
    pub reasoning: Reasoning,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq, Type)]
pub struct FeatureModels {
    pub rewrite: FeatureModel,
    pub rewrite_fallback: FeatureModel,
    pub learning_validator: FeatureModel,
    pub transcription_api: FeatureModel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Type)]
pub struct ModelEntry {
    pub id: String,
    pub speech: bool,
    #[serde(default)]
    pub reasoning: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Type)]
pub enum Feature {
    Rewrite,
    RewriteFallback,
    LearningValidator,
    TranscriptionApi,
}

impl Feature {
    const ALL: [Self; 4] = [
        Self::Rewrite,
        Self::RewriteFallback,
        Self::LearningValidator,
        Self::TranscriptionApi,
    ];

    fn choose(self, models: &mut FeatureModels) -> &mut FeatureModel {
        match self {
            Self::Rewrite => &mut models.rewrite,
            Self::RewriteFallback => &mut models.rewrite_fallback,
            Self::LearningValidator => &mut models.learning_validator,
            Self::TranscriptionApi => &mut models.transcription_api,
        }
    }

    fn choice(self, models: &FeatureModels) -> &FeatureModel {
        match self {
            Self::Rewrite => &models.rewrite,
            Self::RewriteFallback => &models.rewrite_fallback,
            Self::LearningValidator => &models.learning_validator,
            Self::TranscriptionApi => &models.transcription_api,
        }
    }
}

pub fn guess_reasoning(model: &str) -> bool {
    let lowercase = model.to_ascii_lowercase();
    let model = lowercase.rsplit('/').next().unwrap_or_default();
    [
        "gpt-oss",
        "o1",
        "o3",
        "o4",
        "gpt-5",
        "qwq",
        "qwen3",
        "deepseek-reasoner",
        "magistral",
        "gemini-2.5",
        "claude-3-7",
        "claude-sonnet-4",
        "claude-opus-4",
        "claude-haiku-4",
        "claude-sonnet-5",
        "claude-opus-5",
        "claude-haiku-5",
        "claude-fable-5",
        "claude-4",
    ]
    .iter()
    .any(|prefix| model.starts_with(prefix))
        || model.contains("-r1")
}

fn guessed_model(provider_id: &str, id: String) -> ModelEntry {
    let name = id.to_ascii_lowercase();
    let speech = crate::cloud_asr::PROVIDERS
        .iter()
        .any(|(provider, models)| *provider == provider_id && models.contains(&id.as_str()))
        || name.starts_with("whisper-")
        || name.starts_with("gpt-4o-transcribe")
        || name.starts_with("gpt-4o-mini-transcribe");
    ModelEntry {
        reasoning: !speech && guess_reasoning(&id),
        id,
        speech,
    }
}

pub fn migrate(s: &mut AppSettings) -> bool {
    if s.features_migrated {
        return false;
    }

    let choice = |id: &str, models: &std::collections::HashMap<String, String>| FeatureModel {
        provider_id: id.to_owned(),
        model: models
            .get(id)
            .map(|model| model.trim().to_owned())
            .unwrap_or_default(),
        reasoning: Reasoning::Off,
    };
    let rewrite = choice(&s.post_process_provider_id, &s.post_process_models);
    let rewrite_fallback = choice(&s.post_process_fallback_provider_id, &s.post_process_models);
    let transcription_api = choice(&s.cloud_asr.provider_id, &s.cloud_asr.models);
    let legacy_judge = |id: &str| {
        let provider = s.post_process_provider(id)?;
        let model = if id == "groq" {
            crate::adaptive::GROQ_SPARE_MODEL.to_owned()
        } else {
            choice(id, &s.post_process_models).model
        };
        (provider.api_format == PostProcessApiFormat::OpenAi
            && id != APPLE_INTELLIGENCE_PROVIDER_ID
            && has_key(s, id)
            && !model.is_empty())
        .then(|| FeatureModel {
            provider_id: id.to_owned(),
            reasoning: validator_default(&model),
            model,
        })
    };
    let learning_validator = legacy_judge(&s.learning_judge_provider_id)
        .or_else(|| legacy_judge("groq"))
        .or_else(|| {
            s.post_process_enabled
                .then(|| legacy_judge(&s.post_process_provider_id))
                .flatten()
        })
        .unwrap_or_default();

    let mut add = |provider: &str, model: &str, speech| {
        let model = model.trim();
        if !model.is_empty() {
            let entries = s.provider_models.entry(provider.to_owned()).or_default();
            let entry = ModelEntry {
                id: model.to_owned(),
                speech,
                reasoning: !speech && guess_reasoning(model),
            };
            if let Some(existing) = entries.iter_mut().find(|existing| existing.id == model) {
                *existing = entry;
            } else {
                entries.push(entry);
            }
        }
    };
    for (provider, models) in crate::cloud_asr::PROVIDERS {
        for model in *models {
            add(provider, model, true);
        }
    }
    for (provider, model) in &s.post_process_models {
        add(
            provider,
            model.as_str(),
            guessed_model(provider, model.trim().to_owned()).speech,
        );
    }
    for (provider, model) in &s.cloud_asr.models {
        add(provider, model, true);
    }
    add(
        &learning_validator.provider_id,
        &learning_validator.model,
        false,
    );
    s.features = FeatureModels {
        rewrite,
        rewrite_fallback,
        learning_validator,
        transcription_api,
    };
    s.features_migrated = true;
    true
}

fn validator_default(model: &str) -> Reasoning {
    if guess_reasoning(model) {
        Reasoning::Medium
    } else {
        Reasoning::Off
    }
}

fn keyless(id: &str) -> bool {
    id == "custom" || id.starts_with("custom-") || id == APPLE_INTELLIGENCE_PROVIDER_ID
}

fn has_key(s: &AppSettings, id: &str) -> bool {
    keyless(id)
        || s.post_process_api_keys
            .get(id)
            .is_some_and(|key| !key.trim().is_empty())
}

#[derive(Clone, Copy, Debug, Serialize, Type, PartialEq, Eq)]
pub enum FeatureProblem {
    NotChosen,
    NoProvider,
    NoKey,
    NoModel,
    RewriteOff,
    LearningOff,
    ValidatorOff,
    ContextOff,
    NotSpeechModel,
    NotChatModel,
    UnsupportedProvider,
}

#[derive(Serialize, Type)]
pub struct FeatureStatus {
    pub feature: Feature,
    pub provider_label: Option<String>,
    pub model: Option<String>,
    pub automatic: bool,
    pub problem: Option<FeatureProblem>,
    pub reasoning: Reasoning,
    pub model_reasons: bool,
}

fn compatible_provider(p: &PostProcessProvider, feature: Feature) -> bool {
    match feature {
        Feature::LearningValidator | Feature::TranscriptionApi => {
            p.api_format == PostProcessApiFormat::OpenAi && p.id != APPLE_INTELLIGENCE_PROVIDER_ID
        }
        Feature::Rewrite => true,
        Feature::RewriteFallback => p.id != APPLE_INTELLIGENCE_PROVIDER_ID,
    }
}

fn model_entry<'a>(s: &'a AppSettings, choice: &FeatureModel) -> Option<&'a ModelEntry> {
    s.provider_models
        .get(&choice.provider_id)?
        .iter()
        .find(|m| m.id == choice.model)
}

fn validate_choice<'a>(
    s: &'a AppSettings,
    f: Feature,
    choice: &FeatureModel,
) -> Result<(&'a PostProcessProvider, &'a ModelEntry), FeatureProblem> {
    use FeatureProblem::*;
    if choice.provider_id.is_empty() {
        return Err(NotChosen);
    }
    let p = s
        .post_process_provider(&choice.provider_id)
        .ok_or(NoProvider)?;
    if !compatible_provider(p, f) {
        return Err(UnsupportedProvider);
    }
    let m = model_entry(s, choice).ok_or(NoModel)?;
    match (f == Feature::TranscriptionApi, m.speech) {
        (true, false) => Err(NotSpeechModel),
        (false, true) => Err(NotChatModel),
        _ => Ok((p, m)),
    }
}

pub type Target = (PostProcessProvider, String, String, Reasoning);

fn resolved_choice(s: &AppSettings, f: Feature) -> (&FeatureModel, bool) {
    let choice = f.choice(&s.features);
    let automatic = f == Feature::LearningValidator && choice.provider_id.is_empty();
    (
        if automatic {
            &s.features.rewrite
        } else {
            choice
        },
        automatic,
    )
}

fn resolve(s: &AppSettings, f: Feature) -> Result<Target, FeatureProblem> {
    use FeatureProblem::*;
    let (choice, automatic) = resolved_choice(s, f);
    if automatic && !s.post_process_enabled {
        return Err(RewriteOff);
    }
    let (p, m) = validate_choice(s, f, choice)?;
    if !has_key(s, &p.id) {
        return Err(NoKey);
    }
    let key = s
        .post_process_api_keys
        .get(&p.id)
        .map(|k| k.trim().to_owned())
        .unwrap_or_default();
    let reasoning = if !m.reasoning {
        Reasoning::Off
    } else if automatic {
        Reasoning::Medium
    } else {
        choice.reasoning
    };
    Ok((p.clone(), key, m.id.clone(), reasoning))
}

pub fn feature_target(s: &AppSettings, f: Feature) -> Option<Target> {
    resolve(s, f).ok()
}

pub fn feature_status(s: &AppSettings, f: Feature) -> FeatureStatus {
    use FeatureProblem::*;
    let (choice, automatic) = resolved_choice(s, f);
    let model_reasons = model_entry(s, choice).is_some_and(|m| m.reasoning);
    let problem = match f {
        Feature::LearningValidator if !s.learning_judge_enabled => Some(ValidatorOff),
        Feature::LearningValidator if !s.learning_enabled => Some(LearningOff),
        Feature::LearningValidator if !s.context_send_enabled => Some(ContextOff),
        Feature::Rewrite if !s.post_process_enabled => Some(RewriteOff),
        _ => resolve(s, f).err(),
    };
    FeatureStatus {
        feature: f,
        automatic,
        problem,
        model_reasons,
        provider_label: s
            .post_process_provider(&choice.provider_id)
            .map(|p| p.label.clone()),
        model: (!choice.model.is_empty()).then(|| choice.model.clone()),
        reasoning: if !model_reasons {
            Reasoning::Off
        } else if automatic {
            Reasoning::Medium
        } else {
            choice.reasoning
        },
    }
}

fn set_feature_model_in(
    s: &mut AppSettings,
    feature: Feature,
    mut choice: FeatureModel,
) -> Result<(), String> {
    if choice.provider_id.is_empty() {
        choice = FeatureModel::default();
    } else {
        let (_, entry) =
            validate_choice(s, feature, &choice).map_err(|problem| format!("{problem:?}"))?;
        if !entry.reasoning {
            choice.reasoning = Reasoning::Off;
        }
    }
    *feature.choose(&mut s.features) = choice;
    Ok(())
}

fn remove_provider_model_in(
    s: &mut AppSettings,
    provider_id: &str,
    model_id: &str,
) -> Vec<Feature> {
    if let Some(models) = s.provider_models.get_mut(provider_id) {
        models.retain(|m| m.id != model_id);
    }
    let mut reset = Vec::new();
    for feature in Feature::ALL {
        let choice = feature.choose(&mut s.features);
        if choice.provider_id == provider_id && choice.model == model_id {
            *choice = FeatureModel::default();
            reset.push(feature);
        }
    }
    reset
}

fn provider_users(s: &AppSettings, provider_id: &str) -> Vec<Feature> {
    Feature::ALL
        .into_iter()
        .filter(|feature| feature.choice(&s.features).provider_id == provider_id)
        .collect()
}

fn remove_custom_provider_in(s: &mut AppSettings, provider_id: &str) -> Result<(), Vec<Feature>> {
    let used = provider_users(s, provider_id);
    if !used.is_empty() || !(provider_id == "custom" || provider_id.starts_with("custom-")) {
        return Err(used);
    }
    s.post_process_providers
        .retain(|provider| provider.id != provider_id);
    s.provider_models.remove(provider_id);
    Ok(())
}

use crate::settings::{get_settings, write_settings};
use tauri::AppHandle;

static EDIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn edit_settings<T, E>(
    app: &AppHandle,
    edit: impl FnOnce(&mut AppSettings) -> Result<T, E>,
) -> Result<T, E> {
    let _lock = EDIT_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    let mut settings = get_settings(app);
    let result = edit(&mut settings)?;
    write_settings(app, settings);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn get_feature_statuses(app: AppHandle) -> Vec<FeatureStatus> {
    let settings = get_settings(&app);
    Feature::ALL
        .into_iter()
        .map(|feature| feature_status(&settings, feature))
        .collect()
}

#[tauri::command]
#[specta::specta]
pub fn set_feature_model(
    app: AppHandle,
    feature: Feature,
    choice: FeatureModel,
) -> Result<(), String> {
    edit_settings(&app, |settings| {
        set_feature_model_in(settings, feature, choice)
    })
}

#[tauri::command]
#[specta::specta]
pub fn set_provider_models(
    app: AppHandle,
    provider_id: String,
    models: Vec<ModelEntry>,
) -> Result<(), String> {
    edit_settings(&app, |settings| {
        if settings.post_process_provider(&provider_id).is_none() {
            return Err("NoProvider".into());
        }
        let mut ids = std::collections::HashSet::new();
        let models: Vec<_> = models
            .into_iter()
            .filter_map(|mut model| {
                model.id = model.id.trim().to_owned();
                let known = settings
                    .provider_models
                    .get(&provider_id)
                    .is_some_and(|entries| entries.iter().any(|entry| entry.id == model.id));
                if !known && !model.speech && !model.reasoning {
                    model = guessed_model(&provider_id, model.id);
                }
                model.reasoning &= !model.speech;
                (!model.id.is_empty() && ids.insert(model.id.clone())).then_some(model)
            })
            .collect();
        settings.provider_models.insert(provider_id.clone(), models);
        for feature in provider_users(settings, &provider_id) {
            let choice = feature.choice(&settings.features);
            if choice.model.is_empty() {
                continue;
            }
            let (_, entry) =
                validate_choice(settings, feature, choice).map_err(|_| "ModelInUse".to_owned())?;
            if !entry.reasoning {
                feature.choose(&mut settings.features).reasoning = Reasoning::Off;
            }
        }
        Ok(())
    })
}

#[tauri::command]
#[specta::specta]
pub fn remove_provider_model(
    app: AppHandle,
    provider_id: String,
    model_id: String,
) -> Result<Vec<Feature>, String> {
    edit_settings(&app, |settings| {
        Ok(remove_provider_model_in(settings, &provider_id, &model_id))
    })
}

#[tauri::command]
#[specta::specta]
pub fn upsert_provider(
    app: AppHandle,
    provider_id: Option<String>,
    label: String,
    base_url: String,
) -> Result<String, String> {
    let label = label.trim();
    let base_url = base_url.trim();
    let url = reqwest::Url::parse(base_url).map_err(|_| "InvalidUrl")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("InvalidUrl".into());
    }
    edit_settings(&app, |settings| {
        if let Some(id) = provider_id {
            let p = settings
                .post_process_provider_mut(&id)
                .ok_or("NoProvider")?;
            if !p.allow_base_url_edit {
                return Err("InvalidUrl".into());
            }
            p.base_url = base_url.to_owned();
            return Ok(id);
        }
        if label.is_empty() {
            return Err("MissingLabel".into());
        }
        let id = (1..)
            .map(|n| format!("custom-{n}"))
            .find(|id| settings.post_process_provider(id).is_none())
            .expect("finite provider list");
        settings.post_process_providers.push(PostProcessProvider {
            id: id.clone(),
            label: label.to_owned(),
            base_url: base_url.to_owned(),
            allow_base_url_edit: true,
            models_endpoint: Some("/models".into()),
            supports_structured_output: false,
            api_format: PostProcessApiFormat::OpenAi,
        });
        Ok(id)
    })
}

fn set_api_format_in(
    s: &mut AppSettings,
    provider_id: &str,
    api_format: PostProcessApiFormat,
) -> Result<(), String> {
    if provider_id != "custom" {
        return Err("UnsupportedProvider".into());
    }
    if api_format == PostProcessApiFormat::Anthropic
        && provider_users(s, provider_id).iter().any(|feature| {
            matches!(
                feature,
                Feature::LearningValidator | Feature::TranscriptionApi
            )
        })
    {
        return Err("FormatInUse".into());
    }
    let provider = s
        .post_process_provider_mut(provider_id)
        .ok_or("NoProvider")?;
    provider.api_format = api_format;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_api_format_setting(
    app: AppHandle,
    provider_id: String,
    api_format: PostProcessApiFormat,
) -> Result<(), String> {
    edit_settings(&app, |s| set_api_format_in(s, &provider_id, api_format))
}

#[tauri::command]
#[specta::specta]
pub fn change_post_process_api_key_setting(
    app: AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    let _lock = EDIT_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    if get_settings(&app)
        .post_process_provider(&provider_id)
        .is_none()
    {
        return Err("NoProvider".into());
    }
    crate::settings::update_provider_key(&app, &provider_id, &api_key)
}

#[tauri::command]
#[specta::specta]
pub fn remove_custom_provider(app: AppHandle, provider_id: String) -> Result<(), Vec<Feature>> {
    edit_settings(&app, |settings| {
        remove_custom_provider_in(settings, &provider_id)?;
        crate::settings::update_provider_key(&app, &provider_id, "").map_err(|_| Vec::new())?;
        Ok(())
    })
}

fn provider_connection(s: &AppSettings, id: &str) -> Result<(PostProcessProvider, String), String> {
    let provider = s.post_process_provider(id).ok_or("NoProvider")?;
    if !has_key(s, id) {
        return Err("NoKey".into());
    }
    let key = s.post_process_api_keys.get(id).cloned().unwrap_or_default();
    Ok((provider.clone(), key))
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_provider_models(
    app: AppHandle,
    provider_id: String,
) -> Result<Vec<ModelEntry>, String> {
    let settings = get_settings(&app);
    let (provider, key) = provider_connection(&settings, &provider_id)?;
    let ids = if provider_id == APPLE_INTELLIGENCE_PROVIDER_ID {
        vec![crate::settings::APPLE_INTELLIGENCE_DEFAULT_MODEL_ID.to_owned()]
    } else {
        crate::llm_client::fetch_models(&provider, key).await?
    };
    Ok(ids
        .into_iter()
        .map(|id| guessed_model(&provider_id, id))
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn test_provider(app: AppHandle, provider_id: String) -> Result<String, String> {
    let settings = get_settings(&app);
    let (provider, key) = provider_connection(&settings, &provider_id)?;
    if provider_id == APPLE_INTELLIGENCE_PROVIDER_ID {
        return Ok("ok".into());
    }
    if provider.models_endpoint.is_some() {
        crate::llm_client::fetch_models(&provider, key).await?;
    } else {
        let model = settings
            .provider_models
            .get(&provider_id)
            .and_then(|models| models.iter().find(|model| !model.speech))
            .ok_or("NoModel")?;
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            crate::llm_client::send_chat_completion(
                &provider,
                key,
                &model.id,
                "Reply OK.".into(),
                None,
            ),
        )
        .await
        .map_err(|_| "Timeout")?;
        if let Err(error) = response {
            return Err(if error.contains("status 401") {
                "HTTP 401"
            } else if error.contains("status 403") {
                "HTTP 403"
            } else {
                "ConnectionFailed"
            }
            .into());
        }
    }
    Ok("ok".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(provider_id: &str, model: &str) -> FeatureModel {
        FeatureModel {
            provider_id: provider_id.into(),
            model: model.into(),
            ..Default::default()
        }
    }

    fn legacy() -> AppSettings {
        let mut s = AppSettings::default();
        s.post_process_enabled = true;
        s.post_process_provider_id = "openai".into();
        s.post_process_fallback_provider_id = "groq".into();
        s.post_process_models
            .insert("openai".into(), "gpt-5-mini".into());
        s.post_process_models.insert("groq".into(), "llama".into());
        s.post_process_api_keys
            .insert("openai".into(), "test-key".into());
        s.post_process_api_keys
            .insert("groq".into(), "test-key".into());
        s.cloud_asr.provider_id = "groq".into();
        s
    }

    #[test]
    fn migration_keeps_the_rewrite_its_fallback_and_speech_api() {
        let mut s = legacy();
        migrate(&mut s);
        assert_eq!(s.features.rewrite, choice("openai", "gpt-5-mini"));
        assert_eq!(s.features.rewrite_fallback, choice("groq", "llama"));
        assert_eq!(
            s.features.transcription_api,
            choice("groq", "whisper-large-v3")
        );
        assert!(
            model_entry(&s, &s.features.transcription_api)
                .unwrap()
                .speech
        );
        assert!(model_entry(&s, &s.features.rewrite).unwrap().reasoning);
        assert_eq!(s.post_process_models["groq"], "llama");
    }

    #[test]
    fn migration_freezes_the_judge_groq_spare_model_once() {
        let mut s = legacy();
        assert!(migrate(&mut s));
        assert_eq!(
            s.features.learning_validator,
            FeatureModel {
                reasoning: Reasoning::Medium,
                ..choice("groq", crate::adaptive::GROQ_SPARE_MODEL)
            }
        );
        s.features.learning_validator = choice("openai", "gpt-5-mini");
        assert!(!migrate(&mut s));
        assert_eq!(
            s.features.learning_validator,
            choice("openai", "gpt-5-mini")
        );
    }

    #[test]
    fn automatic_validator_follows_the_rewrite_live_and_respects_its_switch() {
        let mut s = legacy();
        s.post_process_enabled = false;
        s.post_process_api_keys.remove("groq");
        migrate(&mut s);
        assert_eq!(s.features.learning_validator, FeatureModel::default());
        assert!(feature_target(&s, Feature::LearningValidator).is_none());
        s.post_process_enabled = true;
        assert_eq!(
            feature_target(&s, Feature::LearningValidator).unwrap().2,
            "gpt-5-mini"
        );
        s.provider_models
            .get_mut("openai")
            .unwrap()
            .push(guessed_model("openai", "gpt-4o-mini".into()));
        s.features.rewrite.model = "gpt-4o-mini".into();
        assert_eq!(
            feature_target(&s, Feature::LearningValidator).unwrap().2,
            "gpt-4o-mini"
        );
        assert_eq!(
            feature_target(&s, Feature::LearningValidator).unwrap().3,
            Reasoning::Off
        );
    }

    #[test]
    fn validator_can_differ_from_rewrite_even_when_rewriting_is_off() {
        let mut s = legacy();
        migrate(&mut s);
        s.post_process_enabled = false;
        let (provider, _, model, reasoning) =
            feature_target(&s, Feature::LearningValidator).unwrap();
        assert_eq!(provider.id, "groq");
        assert_eq!(model, crate::adaptive::GROQ_SPARE_MODEL);
        assert_eq!(reasoning, Reasoning::Medium);
        s.context_send_enabled = false;
        assert_eq!(
            feature_status(&s, Feature::LearningValidator).problem,
            Some(FeatureProblem::ContextOff)
        );
    }

    #[test]
    fn speech_feature_only_accepts_speech_models_and_reports_missing_keys() {
        let mut s = legacy();
        migrate(&mut s);
        assert!(
            set_feature_model_in(&mut s, Feature::TranscriptionApi, choice("groq", "llama"))
                .is_err()
        );
        s.post_process_api_keys.remove("groq");
        assert_eq!(
            feature_status(&s, Feature::TranscriptionApi).problem,
            Some(FeatureProblem::NoKey)
        );
        s.features.transcription_api = choice("groq", "llama");
        assert_eq!(
            feature_status(&s, Feature::TranscriptionApi).problem,
            Some(FeatureProblem::NotSpeechModel)
        );
        assert!(feature_target(&s, Feature::TranscriptionApi).is_none());
    }

    #[test]
    fn remove_provider_refuses_active_features_and_remove_model_clears_them() {
        let mut s = legacy();
        s.post_process_provider_id = "custom".into();
        s.post_process_models
            .insert("custom".into(), "local".into());
        migrate(&mut s);
        assert_eq!(
            remove_custom_provider_in(&mut s, "custom"),
            Err(vec![Feature::Rewrite])
        );
        assert_eq!(
            remove_provider_model_in(&mut s, "custom", "local"),
            vec![Feature::Rewrite]
        );
        assert_eq!(s.features.rewrite, FeatureModel::default());
        assert_eq!(remove_custom_provider_in(&mut s, "custom"), Ok(()));
        assert!(s.post_process_provider("custom").is_none());
    }

    #[test]
    fn rejects_format_changes_that_would_break_the_speech_api() {
        let mut s = legacy();
        s.cloud_asr.provider_id = "custom".into();
        s.cloud_asr
            .models
            .insert("custom".into(), "local-whisper".into());
        migrate(&mut s);
        assert_eq!(
            set_api_format_in(&mut s, "custom", PostProcessApiFormat::Anthropic),
            Err("FormatInUse".into())
        );
        assert!(feature_target(&s, Feature::TranscriptionApi).is_some());
    }

    #[test]
    fn guess_reasoning_knows_the_families() {
        for model in [
            "openai/gpt-oss-120b",
            "o4-mini",
            "gpt-5-mini",
            "deepseek-r1-distill-llama",
            "qwen3-32b",
            "claude-opus-4-6",
        ] {
            assert!(guess_reasoning(model), "{model}");
        }
        for model in ["llama-3.3-70b-versatile", "gpt-4o-mini", "whisper-large-v3"] {
            assert!(!guess_reasoning(model), "{model}");
        }
    }
}
