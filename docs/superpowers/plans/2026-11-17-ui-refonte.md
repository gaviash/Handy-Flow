# Handy Flow — Refonte UI/UX (plan C) : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. For every UI task, also load the `impeccable` skill and read its `reference/craft-floor.md` before editing. Before each task, re-check the file:line references below against the current code: they come from a static audit and may have shifted.

**Goal:** Rendre les réglages de Handy Flow lisibles. Chaque onglet s'ouvre sur un **en-tête d'état** (une phrase qui dit ce qui va se passer). Les sujets sont présentés en **cartes à icônes** qui résument la valeur actuelle et ouvrent une **sous-page**. Les **réglages experts sont repliés**. On corrige au passage les défauts P1 de l'audit : rétention « 30 jours » rejetée, échecs silencieux, suppression d'historique sans retour arrière.

**Non-goal :** aucune fonctionnalité n'est supprimée. L'audit a montré qu'aucun réglage n'est mort (tous ont un effet). On **déplace**, on **replie**, on **renomme**, on **corrige**. Seuls les points d'entrée redondants disparaissent.

**Entrées (à lire avant de commencer) :**
- Audit Codex complet, avec inventaire, preuves fichier:ligne et rangement cible : `docs/superpowers/specs/2026-11-17-ui-ux-audit.md`. **La matrice de sa section 2 est la référence de rangement.** Ce plan la reprend, sauf les écarts listés dans « Décisions ».
- Maquettes validées par l'utilisateur (canvas claude.ai, privé) : https://claude.ai/artifact/2HNUmg3rNyfTm3kH2M2ZL1. On y trouve l'écran actuel, la vue d'ensemble d'Amélioration (en-tête d'état rose, grille 2×2 de cartes, « Réglages experts » replié) et la sous-page Ton (lien retour, pastilles).

**Architecture :** quatre primitives UI nouvelles dans `src/components/ui/` : `StatusHeader`, `SettingCard`, `SubPage`, `ExpertSection`. Chaque page garde son composant, et sa sous-page ouverte vit dans un `useState` local. Quitter l'onglet démonte la page, ce qui referme la sous-page sans état global. Les identifiants de section existants (`postprocessing`, etc.) **ne changent pas**, seuls les libellés changent. Deux sections nouvelles : `vocabulary` et `context`. Les composants de réglage existants (`ToggleSwitch`, `ShortcutInput`, `Dropdown`, `Segmented`…) sont **réutilisés tels quels** : on déplace leurs rendus, pas leur logique.

**Tech Stack :** Tauri 2, Rust, React 18 + Tailwind 4, Zustand (`settingsStore`, `noticeStore`), i18next, Playwright (headless, IPC Tauri simulé, dossier `tests/`).

## Révision du 2026-11-18 (après le plan B, choix du fournisseur du juge)

L'audit date d'avant le juge des corrections (0.2.6) et d'avant le plan B. Ce qu'il ajoute à l'UI, et que ce plan range désormais :
- `learning_judge_enabled` (interrupteur « Juge des corrections »), `learning_judge_provider_id` (menu « Fournisseur du juge » : Automatique + `get_judge_choices`) et la ligne d'état (`get_judge_provider` : « Utilise {fournisseur} » ou « Inactif »). Tout est dans `S/post-processing/LearningSettings.tsx` → rangé en **Vocabulaire · carte Juge des corrections** (Task 8).
- Deux comportements du plan B à **garder tels quels** en déplaçant ce bloc : la relecture du juge après chaque écriture terminée (garde `saving` sur `isUpdating` du store, réponses périmées ignorées) et l'affichage « Automatique » quand le choix enregistré n'est plus utilisable.
- Tests existants à suivre : `tests/learning.spec.ts` (6 tests juge) ouvre la page par le bouton « Post Process ». Ils doivent ouvrir Vocabulaire puis la carte Juge (Task 8).
- Les clés `settings.postProcessing.learning.judge.provider.*` n'existent qu'en `en` et `fr` : les ajouter aux 24 autres locales en Task 15. `bun run check:translations` échoue déjà à la base (1/25 langues), à noter en Task 0.

## Révision du 2026-11-19 (fournisseurs par fonction, validateur d'apprentissage)

Deux plans changent les Réglages et passent **avant** celui-ci :
- **Plan A** `docs/superpowers/plans/2026-11-19-A-providers-per-feature.md` (spec `specs/2026-11-19-providers-per-feature.md`) : les fournisseurs et leurs modèles se configurent **une seule fois**, dans l'onglet Modèles. Chaque fonction IA choisit ensuite son fournisseur et son modèle : Reformulation, Secours, Validateur d'apprentissage, Transcription par API. Il livre les composants `S/models/ProvidersCard.tsx` et `S/models/FeatureModelsCard.tsx`, les commandes `get_feature_statuses`, `set_feature_model`, `set_provider_models`, `remove_provider_model`, `remove_custom_provider`, `test_provider`, et **supprime** `get_judge_choices`, `get_judge_provider` et le menu « Fournisseur du juge ».
- **Plan B** `docs/superpowers/plans/2026-11-19-B-wispr-parity-absorption.md` : le juge des mots devient le **validateur d'apprentissage** (le juge ne garde que les règles de style). Il ajoute la commande `get_learning_stats` et une ligne de compteurs sur 30 jours.

**Ordre d'exécution : Plan A → Plan B → ce plan.** Ce plan ne recrée rien de ce que A et B livrent : il **range** leurs composants dans les cartes et sous-pages. Conséquences, appliquées dans les tâches ci-dessous :
1. **Modèles** (Task 11) devient l'unique endroit des fournisseurs : en-tête d'état, cartes **Fournisseurs** et **Utilisation par fonction** (sous-pages = composants du plan A), puis la transcription locale et l'Expert moteur.
2. **Amélioration** (Task 7) : la carte Fournisseur n'a **plus de formulaire**. Sa sous-page montre la reformulation et son secours en lecture (« {fournisseur} · {modèle} ») avec un bouton « Modifier dans Modèles ».
3. **Vocabulaire** (Task 8) : la carte « Juge des corrections » devient **« Validateur d'apprentissage »**. On garde l'interrupteur (`learning_judge_enabled`) et la ligne d'état lue dans `get_feature_statuses` (`LearningValidator`), avec un lien « Choisir le modèle → Modèles », sans menu de fournisseur. On ajoute la ligne de compteurs du plan B.
4. La section « Révision du 2026-11-18 » ci-dessus est **remplacée** pour tout ce qui touche au menu du fournisseur du juge. La relecture de l'état après chaque écriture (garde `saving`) reste due, désormais sur `get_feature_statuses`.

## Révision du 2026-11-21 (plans A et B livrés)

Les plans A (`4164a0e`) et B (`8c50245`…`197d24e`) sont sur `feat/handy-flow-brand` et `main`. Ce qu'ils changent pour ce plan :
1. **Mots appris : seuls les mots en attente expirent** (`learning.rs:291`, « Wispr never expires its dictionary »). `LearningSettings.tsx:20-29` dit encore le contraire : encre qui pâlit et infobulle « expire le … » sur tous les mots. Corrigé en Task 8.
2. **Validateur sans modèle utilisable ≠ inactif** : sans réponse, `validator::fallback` décide en local avec le lexique (terme rare ou technique et proche de l'entendu → appris, sinon en attente). Le texte partagé `settings.features.problem.NoModel` (« Inactif · aucun modèle utilisable ») reste pour les autres fonctions. La carte Validateur a son propre texte (Task 8).
3. **`learning_judge_enabled` coupe deux choses** : le validateur des mots (`features.rs:341`) et le juge des règles de style (`judge.rs:124`). Le texte FR actuel (`settings.postProcessing.learning.judge.description`) le dit déjà : on le garde.
4. **`FeatureSummary`** (`S/models/FeatureModelsCard.tsx`, déjà rendu dans `LearningSettings.tsx:103`) produit la ligne d'état du validateur : la carte le réutilise, on ne recode pas la logique.
5. **`src/bindings.ts` a été modifié à la main** par le plan B (`Learned.source`, `Source`, `LearningStats`, `getLearningStats`). À régénérer sur Windows avant de commencer (Task 0).
6. **Vérifications manuelles du plan B (Task 9) pas encore faites** (jonction Bloc-notes et Windows Terminal, « point oeuvre » → `.env`, lignes `Learning: candidate …`). Ce plan peut démarrer sans elles, mais elles doivent être faites avant le build 1/2 : un défaut d'apprentissage trouvé après la refonte serait plus dur à isoler.
7. **Pas d'UI pour `Memory.refused`** (annulation définitive) : volontairement hors portée de ce plan.

## Global Constraints

- **Branche :** créer `feat/ui-refonte` depuis `feat/handy-flow-brand`, **une fois les plans A et B fusionnés sur cette branche** (vérifier avec `git log --oneline | grep -i "per feature\|validator"`). Ne jamais pousser vers `upstream`.
- **Machine :** sur ce PC, `bunx` n'existe pas et Playwright le lance (`playwright.config.ts`). Mettre sur le `PATH` un `bunx.cmd` contenant `@bun x %*` (dans le scratchpad, sans toucher au repo). Il n'y a pas non plus de `Desktop\Handy Flow` installé : avant chaque étape « remplacer Handy Flow », demander à l'utilisateur le dossier cible. Pousser vers `origin` seulement si l'utilisateur le demande.
- **Machine Linux (copie `/home/joseph/Claude-Remote/Handy-Flow`) :** le code et les tests peuvent se faire ici, les builds et le remplacement de Handy Flow restent sur le PC Windows. Ne pas toucher `/home/joseph/Handy-Flow` (commit CI jamais poussé), sauf comme `CARGO_TARGET_DIR`. Environnement :
  ```
  source /tmp/handy-plan-a-env            # sysroot ; /tmp peut être vidé au redémarrage, le recréer alors
  export CARGO_TARGET_DIR=/home/joseph/Handy-Flow/src-tauri/target
  export PATH=$HOME/.bun/bin:$PATH
  ```
  Playwright : `./node_modules/.bin/playwright test --trace off`. Les tests de capture d'écran échouent sous Linux (le dépôt n'a que des références `-win32.png`) : les ignorer, puis supprimer les `*-chrome-linux.png` générés avant chaque commit.
- **Tests :** logique Rust pure avec `cargo test --lib` (dans `src-tauri`, `--release` sous Windows). Logique TS avec un petit script `bun` à asserts, comme `src/lib/utils/appearance.test.ts`. UI avec Playwright headless (`bun run test:playwright` sous Windows, commande ci-dessus sous Linux).
- **Builds :** `bun run tauri build --no-bundle -- --profile fast` → `src-tauri/target/fast/handy.exe`. **Deux builds au total**, un par incrément (voir plus bas). Ne jamais modifier les sources pendant un build.
- **Après chaque build :** remplacer `C:\Users\gavri\Desktop\Handy Flow\Handy Flow.exe`, les DLL de `src-tauri/transcribe-libs` et `resources\`. Sauvegarder d'abord `Data\` et l'ancien exe, fermer `Handy Flow.exe`, puis relancer. **Ne jamais toucher `Data\`** ni `Desktop\Handy`.
- **Aucun test qui prend le clavier, le focus, la souris ou l'audio du PC.** Pas de dictée simulée. Les essais réels sont faits par l'utilisateur.
- **i18n :** tout texte visible passe par i18next (ESLint l'impose). Les nouvelles clés vont dans `en` et `fr`, avec la valeur anglaise dans les 24 autres locales, pour que `bun run check:translations` passe. **Ne pas supprimer** une clé tant qu'une locale ou un composant l'utilise encore.
- **`bindings.ts`** se régénère en lançant `bun run tauri dev` après tout changement de commande ou de type Rust. Ce plan ne change aucune signature de commande : il utilise celles des plans A et B telles qu'elles sont livrées.
- **Mouvement :** `cubic-bezier(.22,1,.36,1)`, 150 à 450 ms, rien sous `prefers-reduced-motion: reduce`.
- **Avant chaque commit :** `bun run lint`, `bun run format:check`, `bun run build` (tsc), et `cargo clippy` si du Rust a bougé.
- **Accessibilité :** une carte est un `<button>` (jamais un `div onClick`). Le retour de sous-page est un `<button>` qui rend le focus à la carte d'origine. Une description qui conditionne un choix de confidentialité est **visible**, pas en tooltip.
- **Portée 99 % :** pas de nouvelle fonctionnalité au-delà de ce plan et des plans A et B. Le bouton « Tester » d'un fournisseur (plan A) et la ligne de compteurs d'apprentissage (plan B) existent déjà : on les range, on n'en ajoute pas d'autres (pas d'aperçu de ton).

## Décisions déjà prises (avec l'utilisateur, ne pas rediscuter)

1. Plan **C** : structure réorganisée et nouveau look (cartes, en-têtes d'état, sous-pages).
2. Réglages experts **repliés** dans les pages (`<details>`), pas déplacés dans Debug. Exception : ce que l'audit classe « Debug » (backend clavier, `experimental_enabled`, logs, previews).
3. Bug « 30 jours » : on **ajoute `days30` au parseur Rust**, et on ne retire pas l'option.

**Décisions par défaut prises dans ce plan** (l'agent les applique, et l'utilisateur peut les inverser à la revue) :
- **Apparence reste un onglet.** Elle vient d'être livrée par la branche brand. L'audit proposait de la ranger dans Général, on ne le fait pas.
- **Snippets → onglet Vocabulaire** (comme dans l'audit), alors que la maquette les montrait dans Amélioration.
- Libellé FR de l'onglet `postprocessing` : **« Amélioration »**, qui remplace « Post-traitement ».
- **Validateur d'apprentissage (ex-juge des corrections) → Vocabulaire, en carte** (pas en Expert). Il envoie les corrections à une IA : c'est un choix de confidentialité, donc visible, avec sa description affichée. Son modèle se choisit dans Modèles (plan A), pas dans la carte.
- **Fournisseurs et modèles → onglet Modèles uniquement** (décision utilisateur du 2026-11-19). Aucune autre page n'a de formulaire de clé, d'URL ou de modèle : seulement un résumé et un lien.

## Navigation cible

```
Général       raccourcis · langues · micro · audio · démarrage
Modèles       état · Fournisseurs · Utilisation par fonction · transcription locale · Expert moteur
Amélioration  état · Fournisseur (résumé → Modèles) · Ton · Transformations · Nettoyage · Expert (prompt, délai)
Vocabulaire   NOUVEAU — mots perso · mots appris (+ compteurs) · règles de style · validateur d'apprentissage · snippets · aide à la reconnaissance
Contexte      NOUVEAU — lecture / envoi · exclusions · ce qui a été envoyé · Expert (sources, budget, VS Code)
Historique    (inchangé) + carte Conservation
Apparence     (inchangé)
Avancé        Insertion du texte · Expert Capture
Débogage      (si debug_mode) + backend clavier, Expérimental, logs
À propos      version · mises à jour · diagnostic
```

## Review Focus

1. **Store existant de l'utilisateur** (`Data\settings_store.json`) : aucun réglage n'est perdu ni réinitialisé par la réorganisation. Aucun champ Rust n'est renommé. Testé en Task 17 (lancement sur une copie de `Data\`).
2. **`post_process_enabled = false`** : l'onglet Amélioration reste visible et propose de réactiver. Le lien depuis Modèles fonctionne toujours. Testé en Task 7.
3. **Échec d'une sauvegarde** : la valeur revient à l'ancienne et une notice d'erreur s'affiche. Testé en Task 2.
4. **Suppression d'historique puis « Annuler »** : l'entrée et son WAV sont intacts. Testé en Task 4.
5. **Réglages conditionnels** (Linux, macOS, multicanal, chinois, Apple Intelligence) : leurs gardes sont conservées après déplacement. Vérifié en Task 16 (grep des gardes avant/après).
6. **Validateur déplacé** : la ligne d'état de la carte Vocabulaire suit ce que le validateur utilise vraiment (`get_feature_statuses`), y compris « Automatique : {fournisseur} · {modèle} », après une clé supprimée et après un modèle retiré dans Modèles. Testé en Task 8.
7. **Fournisseurs à un seul endroit** : changer le modèle de la reformulation dans Modèles se voit dans le résumé d'Amélioration et, en Automatique, dans la carte Validateur, sans recharger. Aucune page hors Modèles ne contient de champ de clé (grep en Task 16). Testé en Task 11.

---

## Incrément 1 — Défauts P1, primitives, Amélioration, Vocabulaire, Contexte

### Task 0 : Préparer la branche

- [ ] `git switch feat/handy-flow-brand && git switch -c feat/ui-refonte`
- [ ] Sur Windows : `bun run tauri dev` pour régénérer `src/bindings.ts` (modifié à la main par le plan B). Le diff doit être vide ou de forme seulement ; sinon, corriger le code qui utilisait les types écrits à la main, et faire un commit `chore: regenerate bindings`.
- [ ] `bun install`, puis `bun run build`, `bun run check:translations`, Playwright et `cargo test --lib` pour enregistrer l'état de départ. Noter les échecs déjà présents. Au 2026-11-21 : `cargo test --lib` donne 536 passés ; sous Linux, les 26 tests de capture d'écran échouent (références `-win32` seulement) ; refaire le compte de Playwright et de `check:translations`, les chiffres du 2026-11-18 (84/84, 1/25) sont périmés.
- [ ] Vérifier que les vérifications manuelles du plan B (Task 9) sont faites, ou les noter comme dues avant le build 1/2 (Révision du 2026-11-21, point 6).

### Task 1 : Rétention « 30 jours » acceptée

**Files :** `src-tauri/src/commands/history.rs:~125`

- [ ] Écrire d'abord un test Rust qui échoue. Extraire le `match` en `fn parse_retention_period(&str) -> Result<RecordingRetentionPeriod, String>` dans le même fichier, puis tester les 6 valeurs de l'UI (`never`, `preserve_limit`, `days3`, `days30`, `weeks2`, `months3`) et une valeur inconnue.
- [ ] Ajouter `"days30" => RecordingRetentionPeriod::Days30`. L'enum le connaît déjà (`settings.rs:228`), et c'est la valeur par défaut sous Windows (`settings.rs:961`).
- [ ] `cargo test --release --lib parse_retention` passe.
- [ ] Commit `fix(history): accept the 30-day retention choice`.

### Task 2 : Une sauvegarde qui échoue ne se fait plus passer pour un succès

**Files :** `src/stores/settingsStore.ts:~400-445`, `src/bindings.ts` (lecture seule)

- [ ] Dans `updateSetting`, les updaters renvoient un `Result` (`{status: "ok"|"error", error}`) qui est ignoré. Si `status === "error"`, lever une erreur pour passer dans le `catch` existant : rollback de la valeur et de l'apparence.
- [ ] Dans le `catch`, appeler `notify({ title: t("settings.saveFailed"), detail: String(error), level: "error" })` du `noticeStore`. Clé i18n nouvelle `settings.saveFailed` : « Réglage non enregistré ».
- [ ] Test : script `bun` (`src/stores/settingsStore.test.ts`) qui simule un updater renvoyant `{status:"error"}`. Vérifier que la valeur revient à l'originale et que `notify` est appelé. Si le store est difficile à isoler, faire un test Playwright avec l'IPC simulé qui renvoie une erreur pour `change_recording_retention_period_setting` (nom exact à prendre dans `bindings.ts`).
- [ ] Commit.

### Task 3 : Les autres échecs silencieux passent par les notices

**Files :** `S/post-processing/PostProcessingSettings.tsx:279,294` (modifier/supprimer prompt), `S/post-processing/LearningSettings.tsx:34` (export), `S/models/ModelsSettings.tsx` (lire `error` de `modelStore`), `S/history/HistorySettings.tsx:191,264,286,324` (chargement, favori, audio, dossier)

- [ ] Remplacer chaque `console.error` seul, et chaque `await commands.x()` dont le `status` n'est pas lu, par une vérification du `status` suivie d'un `notify(...)`. Garder le `console.error`.
- [ ] Historique : un échec de chargement affiche un état d'erreur (texte avec bouton « Réessayer »), pas l'état vide.
- [ ] Commit.

### Task 4 : Suppression d'historique annulable

**Files :** `S/history/HistorySettings.tsx:~293,511`

- [ ] Au clic sur supprimer : retirer l'entrée de la liste (comme aujourd'hui), afficher `notify({ level: "info", title: t("history.deleted"), action: { label: t("common.undo"), onClick: restore } })`, et **différer** l'appel à la commande de suppression de ~8 s, soit la durée du toast (`NoticePanel.tsx`). « Annuler » annule le timer et réinsère l'entrée. Démonter la page ou quitter l'app pendant le délai exécute la suppression (flush dans le cleanup de `useEffect`).
- [ ] `shortcut: une suppression en attente est perdue si l'app est tuée pendant les 8 s (l'entrée reste alors, ce qui est le sens sûr).`
- [ ] Test Playwright : supprimer, puis Annuler, et vérifier que l'IPC simulé n'a jamais reçu la commande de suppression. Supprimer sans annuler : la commande est reçue après le délai (horloge Playwright).
- [ ] Commit.

### Task 5 : Primitives UI

**Files :** Create `src/components/ui/StatusHeader.tsx`, `SettingCard.tsx`, `SubPage.tsx`, `ExpertSection.tsx` ; Modify `src/components/ui/index.ts`

S'appuyer sur ce qui existe : le bloc d'état de `S/models/ModelsSettings.tsx:~155` (role="status", bordure warning si problème) et les tokens `bg-wash`, `border-line`, `text-ink-2`, `logo-primary`, `--radius-card`, `--radius-panel`.

- [ ] `StatusHeader({ tone: "ok"|"warn"|"off", title, detail })` : `role="status"`, pastille colorée (moss, warning ou gris) **plus** un mot (« Prêt », « À configurer », « Désactivé »), pour que l'état ne passe pas que par la couleur. Fond rose doux (`color-mix` de `--color-logo-primary`) comme dans la maquette.
- [ ] `SettingCard({ icon, title, summary, onOpen })` : `<button type="button">` plein, icône lucide dans une puce 40×40, titre, résumé sur une ligne (ellipsis avec `title=` complet), chevron. Hauteur ≥ 64 px, focus visible `ring-logo-primary`.
- [ ] `CardGrid` (dans le même fichier) : `grid grid-cols-1 sm:grid-cols-2 gap-3`.
- [ ] `SubPage({ title, description, onBack, children })` : bouton « ‹ {parent} » (texte, pas icône seule), `h1.page-title`, description visible. Au montage, le focus va sur le titre (`tabIndex=-1`). `onBack` rend le focus à la carte d'origine : le parent garde une `ref` par carte.
- [ ] `ExpertSection({ summary, children })` : `<details>` avec `<summary>` « Réglages experts — {summary} », replié par défaut.
- [ ] Vérification : une page Playwright de démonstration n'est pas nécessaire. Ces primitives sont couvertes par les tests des Tasks 7 à 9.
- [ ] Commit.

### Task 6 : Navigation

**Files :** `src/components/Sidebar.tsx`, `src/App.tsx:~55-100`, `src/components/settings/index.ts`, `FR`/`EN` `sidebar.*`

- [ ] `SECTIONS_CONFIG` : ajouter `vocabulary` (icône `BookOpen`) et `context` (icône `Eye`) après `postprocessing`. Ordre final : general, models, postprocessing, vocabulary, context, history, appearance, advanced, debug, about.
- [ ] `postprocessing.enabled` → `() => true`. L'onglet est toujours visible et c'est sa page qui gère l'état désactivé.
- [ ] Libellés : `sidebar.postProcessing` devient « Amélioration » en FR (EN : « Enhancement »). Nouvelles clés `sidebar.vocabulary` (« Vocabulaire » / « Vocabulary ») et `sidebar.context` (« Contexte » / « Context »).
- [ ] `App.tsx` : router les deux nouvelles sections. Vérifier que le rendu d'une section respecte `enabled` (l'audit note que ce n'est pas le cas, `App.tsx:65-90`). Pour `debug`, si `enabled` est faux, retomber sur `general`.
- [ ] Commit.

### Task 7 : Page Amélioration

**Files :** réécrire `S/post-processing/PostProcessingSettings.tsx`. Extraire `PromptEditor` dans `S/post-processing/PromptEditor.tsx`, avec **un seul** formulaire pour créer et éditer (supprimer le JSX dupliqué `:367` / `:438`). Le formulaire fournisseur (`PostProcessingSettingsApi/*`) n'est **plus rendu ici** : après le plan A, il ne vit que dans Modèles (`ProvidersCard`).

Écran principal :
- [ ] En-tête : titre « Amélioration », sous-titre « Un modèle de langage réécrit ta dictée avant de la coller. », et à droite le `ToggleSwitch` de `post_process_enabled`, déplacé depuis Avancé > Expérimental (supprimer `<PostProcessingToggle>` d'`AdvancedSettings.tsx:~80`).
- [ ] `StatusHeader` :
  - désactivé → tone `off` : « Désactivé — la dictée est collée telle quelle. »
  - statut `Rewrite` de `get_feature_statuses` en problème (`NotChosen`, `NoKey`, `NoModel`), hors Apple → `warn` : « À configurer — choisis un fournisseur et un modèle dans Modèles. », avec le bouton « Ouvrir Modèles ».
  - sinon `ok` : « Prêt — {raccourci amélioré} réécrit avec {fournisseur} ». Détail : « Modèle {modèle} · secours : {secours ou « aucun »} · ton adapté à l'app ». Le fournisseur, le modèle et le secours se lisent dans `get_feature_statuses` (`Rewrite`, `RewriteFallback`). Le raccourci se lit dans `bindings.transcribe_with_post_process.current_binding`.
- [ ] `CardGrid` de cartes (icône lucide → résumé) :
  - **Fournisseur** (`Server`) → « {fournisseur} · {modèle} » (+ « · secours {x} » si défini)
  - **Ton** (`MessageCircle`) → « E-mail {ton} · travail {ton} · … » (valeurs de `styles`)
  - **Transformations** (`Wand2`) → « Polish · Instruction vocale · {n} transformations »
  - **Nettoyage** (`Eraser`) → « Hésitations retirées » ou « conservées » (`filler_word_removal_enabled`, déplacé depuis Avancé)
  - **Raccourci** (`Keyboard`) → la combinaison actuelle. La sous-page contient le seul `ShortcutInput` `transcribe_with_post_process`, plus un lien « Tous les raccourcis → Général ».
- [ ] `ExpertSection` « prompt, délai » : `PostProcessTimeout`, carte **Prompt** (sous-page `PromptEditor`). Le prompt maison reste en lecture seule et le dernier prompt reste non supprimable. (Le secours, le format API et l'URL sont désormais dans Modèles.)

Sous-pages :
- [ ] **Fournisseur** (lecture seule) : deux lignes « Reformulation : {fournisseur} · {modèle} » et « Secours : {fournisseur} · {modèle} | aucun », puis le bouton « Modifier dans Modèles » qui ouvre l'onglet Modèles sur la carte Utilisation par fonction. Texte visible : « Le même fournisseur peut servir à d'autres fonctions (transcription, validateur) : tout se règle dans Modèles. »
- [ ] **Ton** : une ligne par catégorie (`email`, `work`, `personal`, `other`) avec icône, libellé et `Segmented` à 5 tons. Le composant existe et gère déjà les flèches, Home et End (`U/Segmented.tsx`). On remplace les 4 `Dropdown`. Si 5 options débordent à la largeur minimale de la fenêtre, laisser le `Segmented` passer à la ligne (`flex-wrap`) plutôt que revenir au menu déroulant.
- [ ] **Transformations** : `TransformSettings` tel quel.
- [ ] Supprimer de cette page ce qui part ailleurs : `StyleSettings` hors sous-page Ton, `SnippetSettings`, `ContextSettings` et `LearningSettings` (→ Tasks 8 et 9).
- [ ] Tests Playwright (`tests/ui-refonte.spec.ts`, avec les fixtures IPC existantes de `tests/fixtures`) :
  - l'onglet est visible avec `post_process_enabled=false`, et l'état « Désactivé » s'affiche ;
  - ouvrir Ton, choisir « Formel » pour Travail : l'IPC reçoit l'update de `styles` ;
  - « ‹ Amélioration » ramène le focus sur la carte Ton ;
  - clé vide → état « À configurer » ;
  - la sous-page Fournisseur ne contient aucun champ de saisie, et « Modifier dans Modèles » ouvre l'onglet Modèles.
- [ ] Commit.

### Task 8 : Page Vocabulaire (nouvelle)

**Files :** Create `S/vocabulary/VocabularySettings.tsx` ; déplacer les rendus de `S/CustomWords.tsx`, `UserName` (aujourd'hui dans `ContextSettings.tsx:52`), `S/post-processing/LearningSettings.tsx`, `SnippetSettings.tsx`, et `AsrContextSettings.tsx` (sauf `screen`, `max_terms`, `capture_wait_ms`)

- [ ] `StatusHeader` `ok` : « {n} mots personnels · {m} mots appris · apprentissage {actif|en pause} ». Si l'apprentissage est coupé, tone `off`.
- [ ] Visible : `ToggleSwitch` apprentissage (`learning_enabled`), champ nom (`user_name`).
- [ ] Cartes :
  - **Mots personnels** (`BookA`) → « {n} mots » → `CustomWords`, retiré d'Avancé
  - **Mots appris** (`Sparkles`) → « {m} mots, dont {k} en attente » → ligne de compteurs du plan B (« 30 derniers jours : … », `get_learning_stats`) puis liste avec supprimer, actualiser, exporter, effacer (confirmation conservée). **Corriger l'expiration** (Révision du 2026-11-21, point 1) : l'encre qui pâlit (`inkLeft`) et l'infobulle `words.expires` ne s'appliquent qu'aux mots en attente (`!w.active`) ; un mot actif reste à pleine encre, sans date. Mettre à jour le commentaire `FORGET_AFTER_S` (« un mot en attente non confirmé en 30 jours est oublié »). Test Playwright : un mot actif ancien n'a pas d'infobulle d'expiration, un mot en attente en a une.
  - **Règles de style** (`PenLine`) → « {r} règles apprises » → liste avec suppression
  - **Validateur d'apprentissage** (`Gavel`) → résumé selon le cas, dans cet ordre : « Désactivé » (`learning_judge_enabled` faux) ; « Inactif : apprentissage coupé » (`learning_enabled` faux) ; « Inactif : envoi du contexte coupé (onglet Contexte) » (`context_send_enabled` faux) ; « Utilise {fournisseur} · {modèle} · raisonnement {niveau} » ou « Automatique : {fournisseur} · {modèle} · raisonnement {niveau} » (statut `LearningValidator` de `get_feature_statuses`). Si le modèle ne raisonne pas, ajouter en `warn` : « Un modèle qui raisonne se trompe moins : choisis-en un dans Modèles. » ; sinon (`NoKey`, `NoModel`, `NotChosen`) « Sans modèle : décision locale (lexique) », car `validator::fallback` apprend encore les termes rares ou techniques (Révision du 2026-11-21, point 2). Nouvelle clé dédiée au validateur (ex. `settings.postProcessing.learning.judge.local`), sans toucher à `settings.features.problem.NoModel`, partagée avec les autres fonctions. Le résumé réutilise `FeatureSummary` (point 4) avec ce seul cas surchargé. Sous-page : `ToggleSwitch` `learning_judge_enabled` avec la description **visible** existante `settings.postProcessing.learning.judge.description` (elle dit déjà que la correction part à une IA pour les règles de style **et** les mots, ce qui est exact : `judge.rs:124`), complétée d'une phrase « Coupé : rien n'est envoyé, les mots sont décidés sur ton PC. » ; ligne d'état, bouton « Choisir le modèle → Modèles ». **Pas de menu de fournisseur.** Le bloc vient de `LearningSettings.tsx` tel que le plan A l'a laissé.
  - **Snippets** (`TextQuote`) → « {s} snippets » → `SnippetSettings`
  - **Aide à la reconnaissance** (`AudioLines`) → « Activée · mots appris + expressions » → toggles ASR `enabled`, `learned_words`, `expressions`, liste repliable d'expressions et ligne de compatibilité du modèle (`support`)
- [ ] `ExpertSection` : son du mot appris (`learned_sound_enabled`) et seuil de correction (`WordCorrectionThreshold`, retiré de Debug).
- [ ] Les composants déplacés gardent leurs clés i18n actuelles (`settings.postProcessing.learning.*`…). On ne renomme pas les clés dans ce plan, seulement les valeurs FR si le texte change.
- [ ] Test Playwright : la page s'affiche, la carte Mots personnels ouvre la sous-page et un ajout appelle l'update `custom_words`.
- [ ] `tests/learning.spec.ts` (10 tests, dont 6 sur le validateur et 1 sur les compteurs) : `openLearning` (`:4`, clique encore « Post Process ») ouvre l'onglet Vocabulaire, puis la carte voulue (Mots appris, Règles de style ou Validateur). Les tests « context sharing prevents it » et « links to the model picker » existent déjà : ne pas les dupliquer. Seule attente à changer : « without a usable model the validator is inactive » (`:114`) attend désormais le texte de décision locale.
- [ ] Commit.

### Task 9 : Page Contexte (nouvelle)

**Files :** Create `S/context/ContextPage.tsx` ; reprendre `S/post-processing/ContextSettings.tsx`, `ContextReports.tsx`, et la partie `screen` / `max_terms` / `capture_wait_ms` d'`AsrContextSettings.tsx`

- [ ] `StatusHeader` :
  - lecture coupée → `off` « Handy Flow ne lit pas l'écran. »
  - lecture active mais envoi coupé → `ok` « Lu sur ton PC, rien n'est envoyé. »
  - les deux actifs → `ok` « Le texte autour du curseur est envoyé au modèle. {n} apps et {d} sites exclus. »
- [ ] Visible, avec **descriptions affichées** (pas en tooltip) : lecture (`context_enabled`), envoi (`context_send_enabled`), aide écran pour la reconnaissance (`asr_context.screen`).
- [ ] Cartes :
  - **Exclusions** (`ShieldOff`) → « {n} apps · {d} sites » → les deux `ExclusionList`. Ajouter la phrase visible : « Dans ces apps, le contexte n'est ni lu ni envoyé. » Vérifier dans `R/actions.rs:~301`, avant de l'écrire, si l'exclusion bloque aussi le collage, et l'ajouter si c'est le cas.
  - **Ce qui a été envoyé** (`Inbox`) → « Dernière dictée : {heure} » ou « Aucune » → `ContextReports` : résumé visible, prompt brut et détails ASR/LLM dans une `ExpertSection`
- [ ] `ExpertSection` : les 8 sources (`ContextParts` réécriture + transformations, en deux groupes titrés), `max_terms`, `capture_wait_ms`, et le lecteur VS Code (`vscode_screen_reader`) **seulement sous Windows** (`R/vscode.rs:171` lit `APPDATA`). Utiliser le helper plateforme déjà employé par `AirPodsMode.tsx:20`.
- [ ] Test Playwright : basculer l'envoi change la phrase de l'en-tête.
- [ ] Commit.

### Fin d'incrément 1 : vérification et build

- [ ] `bun run lint && bun run format:check && bun run build && bun run check:translations && bun run check:themes && bun run test:playwright` : tout est vert, sauf les échecs notés en Task 0.
- [ ] `cd src-tauri && cargo test --release --lib && cargo clippy`.
- [ ] Captures Playwright headless des onglets Amélioration (vue et sous-page Ton), Vocabulaire et Contexte, en clair et en sombre. Les joindre au handoff.
- [ ] **Revue Codex** de l'incrément : `codex exec -m gpt-6.1-terra -c model_reasoning_effort="medium" --dangerously-bypass-approvals-and-sandbox --skip-git-repo-check -o <scratch>/review1.md "Revois le diff git feat/handy-flow-brand..HEAD : réglage perdu, garde plateforme cassée, clé i18n manquante, contrôle sans nom accessible, sauvegarde silencieuse. Fichier:ligne."` Corriger les constats confirmés. Mode bypass exigé par l'utilisateur ; `-s read-only` échoue de toute façon sous Windows (erreur 1223). Le brief dit « ne modifie aucun fichier ».
- [ ] Build `fast` (1/2), remplacer Handy Flow (règles des Global Constraints) et demander à l'utilisateur de tester. Lui donner une liste de 5 points à vérifier au maximum.

---

## Incrément 2 — Général, Modèles, Avancé, Historique, À propos, lexique, accessibilité

### Task 10 : Général

**Files :** `S/general/GeneralSettings.tsx`

- [ ] `StatusHeader` : « {raccourci brut} : dictée telle quelle · {raccourci amélioré} : dictée améliorée · micro {nom} ». La partie améliorée est masquée si `post_process_enabled` est faux.
- [ ] Le compteur du jour reste en tête (livré en 0.2.4).
- [ ] Cartes :
  - **Raccourcis** (`Keyboard`) → les deux combinaisons → `transcribe`, `transcribe_with_post_process`, activation (`ShortcutActivation`), annulation (garde Linux), plus en Expert `HoldThreshold` (retiré de Debug)
  - **Langues** (`Languages`) → « {langues parlées} · interface {langue} » → `PreferredLanguages`, `ChineseScript` (garde), `AppLanguageSelector` (retiré d'À propos)
  - **Microphone** (`Mic`) → nom → `MicrophoneSelector`, en Expert `ChannelSelector` (si multicanal) et `ClamshellMicrophoneSelector` (garde)
  - **Sons** (`Volume2`) → « Activés · {volume} % » ou « Coupés » → `AudioFeedback`, `VolumeSlider`, `SoundPicker` (retiré de Debug), `MuteWhileRecording`, en Expert `OutputDeviceSelector`
  - **Démarrage** (`Power`) → « Au démarrage de Windows · caché » → `AutostartToggle`, `StartHidden`, `ShowTrayIcon` (retirés d'Avancé)
- [ ] Commit.

### Task 11 : Modèles

**Files :** `S/models/ModelsSettings.tsx` ; réutiliser `S/models/ProvidersCard.tsx` et `S/models/FeatureModelsCard.tsx` (plan A) comme contenus de sous-pages ; `S/models/CloudAsrSettings.tsx` (ce qu'il en reste après le plan A).

Écran principal, de haut en bas :
- [ ] `StatusHeader` : une phrase sur la transcription (« Transcription locale avec {modèle} » ou « Transcription par {fournisseur} · {modèle}, repli local »), et en détail le nombre de fonctions IA prêtes (« Reformulation, validateur et transcription prêts » ou la première fonction en problème, d'après `get_feature_statuses`). Tone `warn` si une fonction **activée** a un problème.
- [ ] `CardGrid` :
  - **Fournisseurs** (`Server`) → « {n} fournisseurs · {k} clés OK » (une clé refusée → résumé en `warn`) → sous-page `ProvidersCard` : liste, fiche fournisseur (clé, URL, modèles, case « transcription », bouton Tester), ajout d'un fournisseur compatible OpenAI ;
  - **Utilisation par fonction** (`Workflow`) → « Reformulation {fournisseur} · Validateur {Automatique|fournisseur} (raisonnement {niveau}) · Transcription {API|locale} » → sous-page `FeatureModelsCard` : une ligne par fonction (fournisseur → modèle → **raisonnement** quand le modèle sait raisonner → état). Les modèles qui raisonnent sont en tête du menu du validateur, et les phrases d'aide sur le raisonnement restent **visibles** (plan A) ;
  - **Transcription locale** (`Cpu`) → le modèle local actif → la liste et le téléchargement des modèles locaux (structure actuelle conservée, l'audit dit de ne pas la reconstruire).
- [ ] `ExpertSection` « moteur » en bas : `ModelUnloadTimeout`, `LongFormChunking` (retirés d'Avancé), `AccelerationSelector` (retiré d'Expérimental), `LanguageDetection` et `LanguageDetectionWindow` (retiré de Debug), délai de la transcription par API (`cloud_asr.timeout_secs`).
- [ ] Les liens entrants (Amélioration « Modifier dans Modèles », Vocabulaire « Choisir le modèle ») ouvrent directement la sous-page **Utilisation par fonction**. Ajouter un paramètre d'ouverture à la section `models` (état local initial passé par la navigation, comme les autres retours de sous-page).
- [ ] Le lien « reformulation » s'appelle désormais « Amélioration ».
- [ ] Tests Playwright : la carte Fournisseurs ouvre la liste ; changer le modèle de la reformulation dans Utilisation par fonction se voit dans le résumé d'Amélioration et, en Automatique, dans la carte Validateur (Review Focus 7) ; « Modifier dans Modèles » depuis Amélioration arrive sur Utilisation par fonction.
- [ ] Commit.

### Task 12 : Avancé

**Files :** `S/advanced/AdvancedSettings.tsx`

- [ ] Ce qui reste après les déplacements : carte **Insertion du texte** (`ClipboardPaste`) → « {méthode} · espace final {oui|non} » → `PasteMethod` (+ script et outil Linux, gardes conservées), `ClipboardHandling`, `AppendTrailingSpace`, `AutoSubmit`. En Expert : délais avant et après collage, `ReliablePaste` (retirés de Debug).
- [ ] `ExpertSection` « Capture audio » : `VoiceActivityDetection`, `VadBackendSelector`, `LazyStreamClose`, `AirPodsMode` (garde), `AlwaysOnMicrophone`, `RecordingBuffer`.
- [ ] Retirer de cette page : groupe Expérimental, `ExperimentalToggle` (→ Debug), `KeyboardImplementationSelector` (→ Debug), groupe Historique (→ Task 13), lien Apparence (l'onglet existe).
- [ ] Commit.

### Task 13 : Historique

**Files :** `S/history/HistorySettings.tsx`, `S/RecordingRetentionPeriod.tsx`, `S/HistoryLimit.tsx`

- [ ] Carte **Conservation** (`Archive`) en tête → « Textes : {période} · audio : 20 dernières » → `RecordingRetentionPeriod`, et `HistoryLimit` **seulement si** la période est `preserve_limit` (`R/managers/history.rs:563`). Corriger la description FR pour dire que les **textes** sont aussi supprimés (`FR:~998-1006`, `R/managers/history.rs:540-658`).
- [ ] Recharger les statistiques à chaque événement de mise à jour de l'historique (`:229` les entrées écoutent déjà, alors que `:412` ne charge qu'au montage).
- [ ] Ajouter `aria-pressed` à original/réécrit et à plus/moins (`:565,585`).
- [ ] Commit.

### Task 14 : À propos et Débogage

- [ ] À propos : carte **Mises à jour** → `ShowWhatsNewOnUpdate` et `UpdateChecksToggle` (retiré de Debug, verrou d'environnement conservé). Carte **Diagnostic** → dossier de l'app, et dossier des logs en Expert.
- [ ] Si la version ne peut pas être lue, afficher « Version indisponible » au lieu du `0.1.2` codé en dur (`S/about/AboutSettings.tsx:25`).
- [ ] Débogage : accueille `ExperimentalToggle` et `KeyboardImplementationSelector`. Si le groupe Expérimental n'a plus de contenu, garder le toggle mais **vérifier par grep** qu'`experimental_enabled` ne conditionne plus rien. S'il ne conditionne plus rien, le signaler dans le handoff sans le supprimer.
- [ ] Commit.

### Task 15 : Lexique, i18n, accessibilité transverses

- [ ] FR : un seul nom pour la fonction, « Amélioration ». Remplacer « Post-traitement », « reformulation », « mise au propre » et « affinage » dans les **valeurs** FR (`FR:19,709,728,1018,1172,1173`). Remplacer « Prompt sélectionné » par « Consigne utilisée ». Garder « Polish » et « Snippets » (noms de fonction), avec une description visible en sous-titre de leur carte.
- [ ] Ajouter la clé manquante `common.cancel`, ou corriger l'appel vers `cancel` (`src/overlay/RecordingOverlay.tsx:310`).
- [ ] Corriger le texte faux « Masquée reste disponible dans Avancé » (`FR:1158`).
- [ ] Traduire les restes en anglais : `ModelSelect.tsx:49` (`Use "…"`), `InfoTooltip.tsx:19`, `BaseUrlField.tsx:22`.
- [ ] Relier les labels aux champs (`htmlFor`/`id`, ou `aria-label`) : `SettingContainer.tsx:87`, `HistoryLimit.tsx:37`, champ nom, `PromptEditor`.
- [ ] Overlay : remplacer les `div onClick` de fermeture des notices par des `<button aria-label>` (`RecordingOverlay.tsx:432,501`).
- [ ] `SettingContainer` : passer `descriptionMode` à `"inline"` par défaut **uniquement** dans les nouvelles pages (prop explicite). Ne pas changer le défaut global.
- [ ] Les clés `settings.postProcessing.learning.judge.provider.*` ont été retirées par le plan A (plus de menu) : vérifier qu'aucun composant ne les utilise encore, puis les supprimer de toutes les locales. Ajouter les nouveaux résumés des cartes Validateur, Fournisseurs et Utilisation par fonction, avec la valeur anglaise, aux 24 locales sans traduction. FR : `learning.judge.label` vaut déjà « Validateur d'apprentissage » ; aucune valeur FR ne contient plus « juge » (vérifié le 2026-11-21), revérifier par grep en fin de plan.
- [ ] `bun run check:translations` passe.
- [ ] Commit.

### Task 16 : Contrôle de non-régression du rangement

- [ ] Script `bun scripts/check-settings-coverage.ts` (court, avec asserts) : pour chaque champ de `Settings` dans `bindings.ts`, vérifier qu'il est rendu au moins une fois sous `src/components/settings/**` ou figure dans une liste blanche commentée (champs internes). Il échoue si un réglage n'a plus d'UI après les déplacements.
- [ ] Grep avant/après des gardes plateforme : `AirPodsMode`, `TypingTool`, `ClamshellMicrophoneSelector`, `ChineseScript`, annulation Linux, Apple Intelligence, VS Code. Chaque garde existe encore.
- [ ] Vérifier que plus aucun composant n'est rendu à deux endroits par accident : un seul `<CustomWords`, `<FillerWordRemoval`, etc. (grep `-c`).
- [ ] Review Focus 7 : `ApiKeyField`, `BaseUrlField` et `ModelSelect` ne sont rendus que sous `S/models/` (grep sur `src/components/settings` hors `models/` → 0 résultat).
- [ ] Commit.

### Task 17 : Vérification finale, build, handoff

- [ ] Toutes les commandes de « Fin d'incrément 1 », plus le script de la Task 16.
- [ ] Lancer le build sur une **copie** de `Data\` (dossier portable temporaire), sans toucher l'original. Vérifier que tous les réglages de l'utilisateur s'affichent avec leurs valeurs (Review Focus 1).
- [ ] Revue Codex du diff complet (même commande qu'à l'incrément 1). Corriger les constats confirmés.
- [ ] Build `fast` (2/2) et remplacer Handy Flow.
- [ ] Note de handoff `docs/superpowers/handoff/2026-11-17-ui-refonte.md` : ce qui est fait, captures, décisions par défaut appliquées (Apparence, Snippets, libellé), `shortcut:` laissés, ce qui n'a pas été testé en réel. Liste de 5 vérifications manuelles au maximum pour l'utilisateur.

## Estimation

Incrément 1 : ~5–7 h agent, build compris. Incrément 2 : ~4–5 h. Les pages Amélioration (Task 7) et Vocabulaire (Task 8) font l'essentiel du risque.
