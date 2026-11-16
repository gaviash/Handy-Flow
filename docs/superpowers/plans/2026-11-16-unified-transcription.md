# Transcription unifiée dans Modèles — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Centraliser la configuration et l'explication de la transcription dans Modèles, en gardant la reformulation dans sa page actuelle.

**Architecture:** Réorganiser les composants React existants autour de `cloud_asr` et de `useModelStore`. Le choix API/local reste distinct du modèle local sélectionné ; préserver les commandes et le routage Rust. Réutiliser les champs API existants et leurs sauvegardes.

**Tech Stack:** React, TypeScript, Zustand, i18next, Tailwind, Tauri existants ; aucune dépendance supplémentaire.

**Spec:** `docs/superpowers/specs/2026-11-16-unified-transcription-design.md` (proposition à valider avant exécution).

## Global Constraints

- La reformulation conserve sa page actuelle, ses fournisseurs, ses modèles et son secours.
- Aucun changement du routage Rust ni migration des clés ou des réglages.
- Réutiliser le catalogue et respecter le verrouillage des modèles pendant les tâches.
- Tous les textes d'interface utilisent i18next ; français et anglais rédigés, clés présentes dans toutes les langues.
- Aucun test qui prend le clavier, le focus ou le micro de l'utilisateur sans demande explicite.
- Pas de nouvelle dépendance, de nouveau fournisseur ou de refonte de l'onboarding.

## Review Focus

- API sans clé ou modèle : configuration incomplète affichée, recours local expliqué (tâche 1).
- API personnalisée sans clé : ne pas la marquer incomplète pour ce seul motif (tâche 1).
- Sélection locale en mode API : change le secours, conserve le fournisseur et le modèle API (tâche 2).
- Modèle local supprimé ou téléchargement annulé : absence de secours affichée sans état « prêt » (tâche 2).
- Clé ou URL partagée modifiée : effet partagé expliqué ; aucun changement des modèles de reformulation (tâches 1 et 3).

### Tâche 1 : Unifier le choix et l'explication dans Modèles

**Files:**
- Déplacer et modifier : `src/components/settings/general/CloudAsrSettings.tsx` vers `src/components/settings/models/CloudAsrSettings.tsx`.
- Modifier : `src/components/settings/models/ModelsSettings.tsx`, `src/components/settings/general/GeneralSettings.tsx`.
- Réutiliser : `src/components/settings/PostProcessingSettingsApi/{ProviderSelect,ModelSelect,ApiKeyField,BaseUrlField}.tsx`, `src/hooks/useSettings.ts`.
- Modifier : `src/i18n/locales/*/translation.json`.

**Interfaces:** `CloudAsrSettings: React.FC` conserve les commandes de `useSettings` ; `ModelsSettings` reçoit un callback optionnel `onShowReformulation?: () => void`, raccordé par l'appelant existant.

- [ ] Déplacer le formulaire, retirer son rendu de Général et composer le titre, le résumé et le choix local/API dans Modèles ; garder l'accès API disponible même si le catalogue local est en chargement.
- [ ] Afficher les champs fournisseur/modèle/clé pour l'API, avec état incomplet visible. Réutiliser `updatePostProcessBaseUrl` pour l'URL personnalisée ; indiquer les champs partagés avec la reformulation. Le délai et l'URL sont dans les réglages secondaires.
- [ ] Réutiliser les erreurs de sauvegarde existantes ; une modification échouée ne doit pas paraître enregistrée. Conserver les noms de modèles personnalisés et les valeurs mémorisées par fournisseur.
- [ ] Vérifier dans un rendu navigateur isolé avec données simulées : mode local, Groq configuré, clé absente, modèle absent, serveur personnalisé sans clé, sauvegarde en erreur. Ne pas appeler une API réelle pour cette vérification.

### Tâche 2 : Donner au modèle local son rôle explicite

**Files:**
- Modifier : `src/components/settings/models/{ModelsSettings,ModelRow}.tsx`, `src/components/settings/general/ModelSettingsCard.tsx`.
- Lire/réutiliser : `src/components/settings/models/modelEntries.ts`, `src/components/settings/{LanguageSelector,TranslateToEnglish}.tsx`.
- Modifier : `src/i18n/locales/*/translation.json`.

**Interfaces:** `ModelRow` accepte `role?: "primary" | "fallback"` ; son action continue d'utiliser `pickModel(entry.id)` sans modifier `cloud_asr`. Le catalogue existant conserve ses filtres et téléchargements.

- [ ] Nommer le bloc et les actions selon le rôle principal/secours ; garder le modèle choisi visible avant le catalogue et expliquer les conditions du recours local.
- [ ] Déplacer les options propres au modèle depuis Général vers Modèles ; faire apparaître clairement si une option s'applique au moteur principal ou au secours. Vérifier les capacités API existantes avant de réutiliser les conditions locales de `ModelSettingsCard`.
- [ ] Vérifier avec des données simulées : choisir un secours conserve les réglages API ; sans modèle disponible le téléchargement est proposé ; suppression et annulation actualisent le résumé ; les tâches en cours conservent leurs verrous.

### Tâche 3 : Corriger les repères globaux et valider

**Files:**
- Modifier : `src/components/footer/Footer.tsx`, `src/App.tsx` et `src/components/settings/models/ModelsSettings.tsx`.
- Lire/réutiliser : `src/components/model-selector/ModelSelector.tsx`, `src/components/footer/TranscriptionStatus.tsx`.
- Modifier : `src/i18n/locales/*/translation.json`.

**Interfaces:** Ajouter `onShowModels: () => void` aux props du pied de page, raccordé à la navigation de `App`. Le résumé est un bouton vers Modèles ; il décrit le mode configuré, sans prétendre mesurer la disponibilité réseau ou le moteur de la dernière dictée.

- [ ] Remplacer le sélecteur local du pied de page par ce résumé, conserver le suivi des téléchargements et le statut des tâches, puis raccorder le lien vers la reformulation.
- [ ] Vérifier que l'accueil n'offre plus de choix concurrent, que la reformulation reste autonome et que choisir un modèle ASR ne modifie pas son modèle LLM. Aucun secret dans les captures ou fixtures.
- [ ] Exécuter `bun run build`, `bun run lint`, `bun run check:translations` et `git diff --check` ; corriger les erreurs introduites. Ajouter un petit test comportemental avec l'outillage existant si une nouvelle logique de calcul d'état est extraite, sans installer de framework.
- [ ] Inspecter le rendu isolé : tailles de fenêtre habituelle et réduite, défilement, thèmes clair/sombre, navigation clavier, états incomplets. Lancer une fois `C:/Users/gavri/.agents/skills/impeccable/scripts/impeccable.cmd detect --json` sur les composants modifiés.
- [ ] Mettre à jour le suivi et le handoff, puis commit et push selon l'instruction de session. La compilation/installaton du bureau se traite après validation de l'interface ; aucun déploiement dans cette phase de planification.
