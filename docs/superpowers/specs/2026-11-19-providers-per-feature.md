# Cahier des charges : fournisseurs configurés une fois, modèle choisi par fonction

**Demande** : dans les Réglages, configurer tous les fournisseurs et tous les modèles voulus à un seul endroit, puis choisir pour **chaque fonction IA** (reformulation, validateur d'apprentissage, transcription par API…) le fournisseur et le modèle parmi ceux configurés.
**Aujourd'hui, c'est confus** :
- La reformulation a un fournisseur (`post_process_provider_id`), un modèle par fournisseur (`post_process_models`) et un fournisseur de secours.
- La transcription par API a ses propres choix (`cloud_asr.provider_id`, `cloud_asr.models`), mais **partage les clés** de la reformulation (`post_process_api_keys`).
- Le juge des corrections choisit un fournisseur (`learning_judge_provider_id`, « Automatique » par défaut) et prend **le modèle de reformulation de ce fournisseur**. Ce lien est invisible pour l'utilisateur.

**Liens** :
- Remplace, dans `plans/2026-11-17-ui-refonte.md`, le menu « Fournisseur du juge » de la carte Juge (Task 8) et la partie « Fournisseur » de la page Amélioration (Task 6).
- Répond à Q1 de `specs/2026-11-19-wispr-parity-absorption.md` : le validateur d'apprentissage devient une fonction comme les autres.

---

## 1. Modèle de données

### 1.1 Fournisseurs configurés (un seul endroit)

```
ProviderConfig {
  id: String                // "groq", "openai", "cloudflare", "custom-1"…
  label: String
  base_url: String          // éditable si allow_base_url_edit
  api_format, supports_structured_output, models_endpoint   // inchangés (PostProcessProvider)
  // clé : dans le coffre existant (SecretMap), jamais dans settings_store.json en clair
  models: Vec<ModelEntry>   // les modèles que l'utilisateur a ajoutés pour ce fournisseur
}
ModelEntry { id: String, capabilities: Set<Chat | Speech> }   // Speech = transcription audio
```

- La liste de modèles vient de `models_endpoint` (bouton « Actualiser ») ou d'une saisie manuelle.
- Un modèle est marqué `Speech` s'il est connu pour la transcription (`whisper-*`, `gpt-4o-transcribe`…) ou si l'utilisateur le coche. Sinon il est `Chat`.

### 1.2 Choix par fonction

```
FeatureModel { provider_id: String, model: String }   // vide = fonction désactivée ou "Automatique"

features: {
  rewrite:            FeatureModel   // reformulation (+ transformations)
  rewrite_fallback:   FeatureModel   // secours de la reformulation (optionnel)
  learning_validator: FeatureModel   // validateur d'apprentissage (EX-2 du cahier absorption) + règles de style
  transcription_api:  FeatureModel   // transcription par API ; vide = modèle local seulement
}
```

- **Automatique** n'existe que pour le validateur : il prend le choix de `rewrite`. C'est le comportement d'aujourd'hui, mais affiché en clair (« Automatique : Groq · openai/gpt-oss-120b »).
- Une fonction ne propose que les modèles compatibles : `Speech` pour la transcription, `Chat` pour les autres.

### 1.3 Migration (DOIT, sans perte)

Au premier démarrage de la nouvelle version :
- `post_process_providers` + `post_process_models` → `ProviderConfig.models` (le modèle choisi devient une `ModelEntry`) ;
- `post_process_provider_id` + son modèle → `features.rewrite` ; le secours → `features.rewrite_fallback` ;
- `cloud_asr.provider_id` + `cloud_asr.models[provider]` → `features.transcription_api` (les modèles de `cloud_asr.models` deviennent des `ModelEntry` `Speech`) ;
- juge : on fige **le choix effectif actuel**, calculé par `judge_target` (`judge.rs:170`), dans `features.learning_validator`. Attention : l'« Automatique » actuel prend Groq en priorité, avec le modèle `GROQ_SPARE_MODEL`, puis la reformulation. Si `judge_target` ne renvoie rien → Automatique (= la reformulation, désormais affiché en clair) ;
- les clés restent où elles sont.

Les anciens champs sont lus pour migrer, puis ignorés : ils ne sont pas supprimés tout de suite, pour pouvoir revenir à une version précédente.

### 1.4 Niveau de raisonnement par fonction (demande du 2026-11-19)

Aujourd'hui, **tous** les appels LLM désactivent le raisonnement (`llm_client::reasoning_disable_params`, et le juge passe `disable_reasoning = true` à `send_chat_completion_with_schema`). C'est bon pour la reformulation, qui doit être rapide. C'est mauvais pour le validateur, qui doit juger finement si une correction est un nom, un terme technique ou un changement de sens.

- `ModelEntry` gagne `reasoning: bool` : le modèle sait raisonner. Valeur proposée d'après le nom (`gpt-oss*`, `o1*`, `o3*`, `o4*`, `gpt-5*`, `*-r1*`, `qwq*`, `qwen3*`, `deepseek-reasoner`, `magistral*`, `gemini-2.5*`, `claude-*` récents), modifiable par une case « raisonnement » dans la fiche du fournisseur.
- `FeatureModel` gagne `reasoning: Reasoning` ∈ {`Off`, `Low`, `Medium`, `High`}. Il n'est proposé que si le modèle choisi a `reasoning: true`, sinon il est forcé à `Off`.
- **Valeurs par défaut** : reformulation, secours et transcription → `Off`. Validateur → `Medium` si le modèle sait raisonner.
- **Envoi** : le paramètre propre à chaque fournisseur remplace la désactivation actuelle.
  - OpenAI et compatibles, Groq (`gpt-oss`) : `reasoning_effort: "low" | "medium" | "high"`.
  - OpenRouter : `reasoning: { effort, exclude: true }`.
  - DeepSeek : `thinking: { type: "enabled" }`.
  - Cloudflare Workers AI : `chat_template_kwargs: { enable_thinking: true }`.
  - `Off` garde exactement le comportement actuel.
  - Le texte de raisonnement est toujours retiré de la réponse (`strip_think_block`, `exclude`).
- **Délai** : le validateur tourne en arrière-plan. Son délai passe de 15 s à 45 s quand le raisonnement est actif.

## 2. Interface (Réglages)

### 2.1 Onglet « Modèles » → section **Fournisseurs**

- Liste des fournisseurs configurés : nom, état de la clé (OK / manquante / refusée), nombre de modèles.
- Fiche d'un fournisseur :
  - clé (champ masqué) et URL (si éditable) ;
  - modèles : « Actualiser depuis le fournisseur », ajout manuel, suppression, case « transcription » ;
  - bouton **Tester**, qui envoie une petite requête et affiche OK ou l'erreur.
- « Ajouter un fournisseur compatible OpenAI » (custom).

### 2.2 Section **Utilisation par fonction**

Une ligne par fonction, avec deux menus (fournisseur, puis modèle de ce fournisseur), un troisième menu **Raisonnement** (Désactivé / Faible / Moyen / Élevé) quand le modèle sait raisonner, et une ligne d'état.

**Modèles qui raisonnent en premier** : dans le menu des modèles du **validateur**, les modèles `reasoning: true` sont listés en tête, sous un intertitre « Raisonnement (recommandé) » avec un badge. Pour la reformulation, l'ordre reste alphabétique, avec le même badge. Phrases d'aide **visibles** sous chaque ligne :
- Validateur : « Le validateur décide si une correction doit être apprise. Un modèle qui raisonne se trompe moins : choisis-en un, avec un raisonnement moyen. Il tourne en arrière-plan, la lenteur ne se voit pas. »
- Reformulation : « La reformulation doit être rapide. Laisse le raisonnement désactivé, sauf si la qualité compte plus que l'attente. »

Les lignes de fonction :

| Fonction | Choix | État affiché |
|---|---|---|
| Reformulation | fournisseur + modèle | « Prêt » / « À configurer » |
| Secours de la reformulation | fournisseur + modèle, ou « Aucun » | — |
| Validateur d'apprentissage | Automatique ou fournisseur + modèle | « Utilise {fournisseur} · {modèle} » / « Inactif : … » (mêmes cas que la carte Juge actuelle) |
| Transcription par API | fournisseur + modèle `Speech`, ou « Modèle local seulement » | « Repli sur le modèle local en cas d'échec » |

Les pages Amélioration et Vocabulaire (carte Juge) gardent un **résumé** et un lien vers cette section, sans menu de fournisseur en double.

## 3. Côté Rust

- Une fonction unique `feature_target(settings, Feature) -> Option<(ProviderConfig, key, model)>` remplace `judge_target`, la résolution du fournisseur de reformulation et celle de la transcription API. « Automatique » est résolu ici seulement.
- `get_feature_status(Feature)` alimente les lignes d'état (remplace `get_judge_provider`).
- Commandes : `upsert_provider`, `remove_provider` (refusé si une fonction l'utilise, avec la liste des fonctions), `refresh_provider_models`, `test_provider`, `set_feature_model(Feature, FeatureModel)`.

## 4. Critères d'acceptation

1. **Migration** : un `settings_store.json` réel de 0.2.6 ou 0.2.7 (fixture) donne exactement les mêmes fournisseur et modèle effectifs pour la reformulation, son secours, le juge et la transcription API. Test Rust.
2. Choisir pour le validateur un fournisseur et un modèle **différents** de ceux de la reformulation : les requêtes du validateur partent vers ce fournisseur et ce modèle. Test Rust sur `feature_target`.
3. La transcription ne propose que des modèles `Speech` ; supprimer le modèle choisi bascule la fonction sur « Modèle local seulement », avec un avertissement.
4. Supprimer un fournisseur utilisé est refusé, avec la liste des fonctions concernées.
5. Tests Playwright : configurer deux fournisseurs, affecter chaque fonction, recharger : les choix sont conservés. Les 6 tests Juge existants sont adaptés à la nouvelle ligne Validateur.
6. Aucune clé n'apparaît dans `settings_store.json`, dans les logs ou dans `bindings.ts`.
7. **Raisonnement** :
   - le validateur réglé sur `Medium` avec un modèle `gpt-oss` envoie `reasoning_effort: "medium"` ;
   - la reformulation sur `Off` envoie la désactivation actuelle ;
   - un modèle sans raisonnement n'envoie aucun paramètre ;
   - le texte de raisonnement n'apparaît jamais dans la réponse utilisée.

   Tests Rust sur le corps de requête, comme les tests existants de `llm_client.rs`.
8. Dans le menu des modèles du validateur, les modèles qui raisonnent sont listés en premier (test Playwright).

## 5. Hors périmètre

- Choisir un modèle différent par application ou par style.
- Les modèles locaux de transcription (onglet Modèles actuel) : inchangés.
- Plusieurs secours en cascade.
