# Handy Flow — Refonte UI/UX (plan C) : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. For every UI task, also load the `impeccable` skill and read its `reference/craft-floor.md` before editing. Before each task, re-check the file:line references below against the current code: they come from a static audit and may have shifted.

**Goal:** Rendre les réglages de Handy Flow lisibles. Chaque onglet s'ouvre sur un **en-tête d'état** (une phrase qui dit ce qui va se passer). Les sujets sont présentés en **cartes à icônes** qui résument la valeur actuelle et ouvrent une **sous-page**. Les **réglages experts sont repliés**. On corrige au passage les défauts P1 de l'audit : rétention « 30 jours » rejetée, échecs silencieux, suppression d'historique sans retour arrière.

**Non-goal :** aucune fonctionnalité n'est supprimée. L'audit a montré qu'aucun réglage n'est mort (tous ont un effet). On **déplace**, on **replie**, on **renomme**, on **corrige**. Seuls les points d'entrée redondants disparaissent.

**Entrées (à lire avant de commencer) :**
- Audit Codex complet, avec inventaire, preuves fichier:ligne et rangement cible : `docs/superpowers/specs/2026-11-17-ui-ux-audit.md`. **La matrice de sa section 2 est la référence de rangement.** Ce plan la reprend, sauf les écarts listés dans « Décisions ».
- Maquettes validées par l'utilisateur (canvas claude.ai, privé) : https://claude.ai/artifact/2HNUmg3rNyfTm3kH2M2ZL1. On y trouve l'écran actuel, la vue d'ensemble d'Amélioration (en-tête d'état rose, grille 2×2 de cartes, « Réglages experts » replié) et la sous-page Ton (lien retour, pastilles).

**Architecture :** quatre primitives UI nouvelles dans `src/components/ui/` : `StatusHeader`, `SettingCard`, `SubPage`, `ExpertSection`. Chaque page garde son composant, et sa sous-page ouverte vit dans un `useState` local. Quitter l'onglet démonte la page, ce qui referme la sous-page sans état global. Les identifiants de section existants (`postprocessing`, etc.) **ne changent pas**, seuls les libellés changent. Deux sections nouvelles : `vocabulary` et `context`. Les composants de réglage existants (`ToggleSwitch`, `ShortcutInput`, `Dropdown`, `Segmented`…) sont **réutilisés tels quels** : on déplace leurs rendus, pas leur logique.

**Tech Stack :** Tauri 2, Rust, React 18 + Tailwind 4, Zustand (`settingsStore`, `noticeStore`), i18next, Playwright (headless, IPC Tauri simulé, dossier `tests/`).

## Global Constraints

- **Branche :** créer `feat/ui-refonte` depuis `feat/handy-flow-brand` **locale** (elle a 2 commits non poussés, la 0.2.4 : ne pas la réinitialiser sur `origin`). Ne jamais pousser vers `upstream`. Pousser vers `origin` seulement si l'utilisateur le demande.
- **Tests :** logique Rust pure avec `cargo test --release --lib` (dans `src-tauri`). Logique TS avec un petit script `bun` à asserts, comme `src/lib/utils/appearance.test.ts`. UI avec Playwright headless (`bun run test:playwright`).
- **Builds :** `bun run tauri build --no-bundle -- --profile fast` → `src-tauri/target/fast/handy.exe`. **Deux builds au total**, un par incrément (voir plus bas). Ne jamais modifier les sources pendant un build.
- **Après chaque build :** remplacer `C:\Users\gavri\Desktop\Handy Flow\Handy Flow.exe`, les DLL de `src-tauri/transcribe-libs` et `resources\`. Sauvegarder d'abord `Data\` et l'ancien exe, fermer `Handy Flow.exe`, puis relancer. **Ne jamais toucher `Data\`** ni `Desktop\Handy`.
- **Aucun test qui prend le clavier, le focus, la souris ou l'audio du PC.** Pas de dictée simulée. Les essais réels sont faits par l'utilisateur.
- **i18n :** tout texte visible passe par i18next (ESLint l'impose). Les nouvelles clés vont dans `en` et `fr`, avec la valeur anglaise dans les 24 autres locales, pour que `bun run check:translations` passe. **Ne pas supprimer** une clé tant qu'une locale ou un composant l'utilise encore.
- **`bindings.ts`** se régénère en lançant `bun run tauri dev` après tout changement de commande ou de type Rust. Ce plan ne change aucune signature de commande.
- **Mouvement :** `cubic-bezier(.22,1,.36,1)`, 150 à 450 ms, rien sous `prefers-reduced-motion: reduce`.
- **Avant chaque commit :** `bun run lint`, `bun run format:check`, `bun run build` (tsc), et `cargo clippy` si du Rust a bougé.
- **Accessibilité :** une carte est un `<button>` (jamais un `div onClick`). Le retour de sous-page est un `<button>` qui rend le focus à la carte d'origine. Une description qui conditionne un choix de confidentialité est **visible**, pas en tooltip.
- **Portée 99 % :** pas de nouvelle fonctionnalité au-delà de ce plan. Ne pas ajouter de bouton « Tester », d'aperçu de ton ou de statistiques.

## Décisions déjà prises (avec l'utilisateur, ne pas rediscuter)

1. Plan **C** : structure réorganisée et nouveau look (cartes, en-têtes d'état, sous-pages).
2. Réglages experts **repliés** dans les pages (`<details>`), pas déplacés dans Debug. Exception : ce que l'audit classe « Debug » (backend clavier, `experimental_enabled`, logs, previews).
3. Bug « 30 jours » : on **ajoute `days30` au parseur Rust**, et on ne retire pas l'option.

**Décisions par défaut prises dans ce plan** (l'agent les applique, et l'utilisateur peut les inverser à la revue) :
- **Apparence reste un onglet.** Elle vient d'être livrée par la branche brand. L'audit proposait de la ranger dans Général, on ne le fait pas.
- **Snippets → onglet Vocabulaire** (comme dans l'audit), alors que la maquette les montrait dans Amélioration.
- Libellé FR de l'onglet `postprocessing` : **« Amélioration »**, qui remplace « Post-traitement ».

## Navigation cible

```
Général       raccourcis · langues · micro · audio · démarrage
Modèles       transcription locale / API (structure actuelle conservée) + Expert moteur
Amélioration  état · Fournisseur · Ton · Transformations · Nettoyage · Expert (prompt, délai, secours, format/URL)
Vocabulaire   NOUVEAU — mots perso · mots appris · règles de style · snippets · aide à la reconnaissance
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

---

## Incrément 1 — Défauts P1, primitives, Amélioration, Vocabulaire, Contexte

### Task 0 : Préparer la branche

- [ ] `git switch feat/handy-flow-brand && git switch -c feat/ui-refonte`
- [ ] `bun install`, puis `bun run build` et `bun run test:playwright` pour enregistrer l'état de départ (noter les tests déjà en échec, s'il y en a).

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

**Files :** réécrire `S/post-processing/PostProcessingSettings.tsx`. Extraire `PromptEditor` dans `S/post-processing/PromptEditor.tsx`, avec **un seul** formulaire pour créer et éditer (supprimer le JSX dupliqué `:367` / `:438`). Garder `ProviderSettings` (le contenu actuel de `PostProcessingSettingsApiComponent`) dans son propre fichier.

Écran principal :
- [ ] En-tête : titre « Amélioration », sous-titre « Un modèle de langage réécrit ta dictée avant de la coller. », et à droite le `ToggleSwitch` de `post_process_enabled`, déplacé depuis Avancé > Expérimental (supprimer `<PostProcessingToggle>` d'`AdvancedSettings.tsx:~80`).
- [ ] `StatusHeader` :
  - désactivé → tone `off` : « Désactivé — la dictée est collée telle quelle. »
  - pas de clé ou de modèle (hors Apple) → `warn` : « À configurer — choisis un fournisseur et une clé. »
  - sinon `ok` : « Prêt — {raccourci amélioré} réécrit avec {fournisseur} ». Détail : « Modèle {modèle} · secours : {secours ou « aucun »} · ton adapté à l'app ». Le raccourci se lit dans `bindings.transcribe_with_post_process.current_binding`.
- [ ] `CardGrid` de cartes (icône lucide → résumé) :
  - **Fournisseur** (`Server`) → « {fournisseur} · {modèle} » (+ « · secours {x} » si défini)
  - **Ton** (`MessageCircle`) → « E-mail {ton} · travail {ton} · … » (valeurs de `styles`)
  - **Transformations** (`Wand2`) → « Polish · Instruction vocale · {n} transformations »
  - **Nettoyage** (`Eraser`) → « Hésitations retirées » ou « conservées » (`filler_word_removal_enabled`, déplacé depuis Avancé)
  - **Raccourci** (`Keyboard`) → la combinaison actuelle. La sous-page contient le seul `ShortcutInput` `transcribe_with_post_process`, plus un lien « Tous les raccourcis → Général ».
- [ ] `ExpertSection` « prompt, délai, secours, format » : `PostProcessTimeout`, carte **Prompt** (sous-page `PromptEditor`). Le prompt maison reste en lecture seule et le dernier prompt reste non supprimable.

Sous-pages :
- [ ] **Fournisseur** : fournisseur, clé, modèle avec rafraîchissement. Dans une `ExpertSection` : format API (custom), URL (si `allow_base_url_edit`), fournisseur de secours. Sous la clé, ajouter un texte visible : « Cette clé sert aussi à la transcription par API (onglet Modèles). » quand le fournisseur est `groq`, `openai` ou `custom`. Doublon voulu, désormais annoncé.
- [ ] **Ton** : une ligne par catégorie (`email`, `work`, `personal`, `other`) avec icône, libellé et `Segmented` à 5 tons. Le composant existe et gère déjà les flèches, Home et End (`U/Segmented.tsx`). On remplace les 4 `Dropdown`. Si 5 options débordent à la largeur minimale de la fenêtre, laisser le `Segmented` passer à la ligne (`flex-wrap`) plutôt que revenir au menu déroulant.
- [ ] **Transformations** : `TransformSettings` tel quel.
- [ ] Supprimer de cette page ce qui part ailleurs : `StyleSettings` hors sous-page Ton, `SnippetSettings`, `ContextSettings` et `LearningSettings` (→ Tasks 8 et 9).
- [ ] Tests Playwright (`tests/ui-refonte.spec.ts`, avec les fixtures IPC existantes de `tests/fixtures`) :
  - l'onglet est visible avec `post_process_enabled=false`, et l'état « Désactivé » s'affiche ;
  - ouvrir Ton, choisir « Formel » pour Travail : l'IPC reçoit l'update de `styles` ;
  - « ‹ Amélioration » ramène le focus sur la carte Ton ;
  - clé vide → état « À configurer ».
- [ ] Commit.

### Task 8 : Page Vocabulaire (nouvelle)

**Files :** Create `S/vocabulary/VocabularySettings.tsx` ; déplacer les rendus de `S/CustomWords.tsx`, `UserName` (aujourd'hui dans `ContextSettings.tsx:52`), `S/post-processing/LearningSettings.tsx`, `SnippetSettings.tsx`, et `AsrContextSettings.tsx` (sauf `screen`, `max_terms`, `capture_wait_ms`)

- [ ] `StatusHeader` `ok` : « {n} mots personnels · {m} mots appris · apprentissage {actif|en pause} ». Si l'apprentissage est coupé, tone `off`.
- [ ] Visible : `ToggleSwitch` apprentissage (`learning_enabled`), champ nom (`user_name`).
- [ ] Cartes :
  - **Mots personnels** (`BookA`) → « {n} mots » → `CustomWords`, retiré d'Avancé
  - **Mots appris** (`Sparkles`) → « {m} mots, dont {k} en attente » → liste avec supprimer, actualiser, exporter, effacer (confirmation conservée)
  - **Règles de style** (`PenLine`) → « {r} règles apprises » → liste avec suppression
  - **Snippets** (`TextQuote`) → « {s} snippets » → `SnippetSettings`
  - **Aide à la reconnaissance** (`AudioLines`) → « Activée · mots appris + expressions » → toggles ASR `enabled`, `learned_words`, `expressions`, liste repliable d'expressions et ligne de compatibilité du modèle (`support`)
- [ ] `ExpertSection` : son du mot appris (`learned_sound_enabled`) et seuil de correction (`WordCorrectionThreshold`, retiré de Debug).
- [ ] Les composants déplacés gardent leurs clés i18n actuelles (`settings.postProcessing.learning.*`…). On ne renomme pas les clés dans ce plan, seulement les valeurs FR si le texte change.
- [ ] Test Playwright : la page s'affiche, la carte Mots personnels ouvre la sous-page et un ajout appelle l'update `custom_words`.
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
- [ ] **Revue Codex** de l'incrément : `codex exec -s read-only --skip-git-repo-check -o <scratch>/review1.md "Revois le diff git feat/handy-flow-brand..HEAD : réglage perdu, garde plateforme cassée, clé i18n manquante, contrôle sans nom accessible, sauvegarde silencieuse. Fichier:ligne."` Corriger les constats confirmés.
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

**Files :** `S/models/ModelsSettings.tsx`

- [ ] La structure actuelle est conservée : elle a déjà un résumé d'état, et l'audit dit de ne pas la reconstruire. Seul le bloc d'état passe sur `StatusHeader`, pour la cohérence visuelle.
- [ ] Ajouter en bas une `ExpertSection` « moteur » : `ModelUnloadTimeout`, `LongFormChunking` (retirés d'Avancé), `AccelerationSelector` (retiré d'Expérimental), `LanguageDetection` et `LanguageDetectionWindow` (retiré de Debug).
- [ ] Le lien « reformulation » s'appelle désormais « Amélioration ».
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
- [ ] `bun run check:translations` passe.
- [ ] Commit.

### Task 16 : Contrôle de non-régression du rangement

- [ ] Script `bun scripts/check-settings-coverage.ts` (court, avec asserts) : pour chaque champ de `Settings` dans `bindings.ts`, vérifier qu'il est rendu au moins une fois sous `src/components/settings/**` ou figure dans une liste blanche commentée (champs internes). Il échoue si un réglage n'a plus d'UI après les déplacements.
- [ ] Grep avant/après des gardes plateforme : `AirPodsMode`, `TypingTool`, `ClamshellMicrophoneSelector`, `ChineseScript`, annulation Linux, Apple Intelligence, VS Code. Chaque garde existe encore.
- [ ] Vérifier que plus aucun composant n'est rendu à deux endroits par accident : un seul `<CustomWords`, `<FillerWordRemoval`, etc. (grep `-c`).
- [ ] Commit.

### Task 17 : Vérification finale, build, handoff

- [ ] Toutes les commandes de « Fin d'incrément 1 », plus le script de la Task 16.
- [ ] Lancer le build sur une **copie** de `Data\` (dossier portable temporaire), sans toucher l'original. Vérifier que tous les réglages de l'utilisateur s'affichent avec leurs valeurs (Review Focus 1).
- [ ] Revue Codex du diff complet (même commande qu'à l'incrément 1). Corriger les constats confirmés.
- [ ] Build `fast` (2/2) et remplacer Handy Flow.
- [ ] Note de handoff `docs/superpowers/handoff/2026-11-17-ui-refonte.md` : ce qui est fait, captures, décisions par défaut appliquées (Apparence, Snippets, libellé), `shortcut:` laissés, ce qui n'a pas été testé en réel. Liste de 5 vérifications manuelles au maximum pour l'utilisateur.

## Estimation

Incrément 1 : ~5–7 h agent, build compris. Incrément 2 : ~4–5 h. Les pages Amélioration (Task 7) et Vocabulaire (Task 8) font l'essentiel du risque.
