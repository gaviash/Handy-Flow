# Reprise — déploiement de la transcription par API (2026-11-16, soir)

Branche `feat/adaptive-learning-context`, poussée à `4d8f426`. Tout est commité, les 408 tests unitaires passent.

## Où je me suis arrêté

Je livrais sur le bureau la version avec la transcription par API (Groq `whisper-large-v3`). La revue Codex est faite, l'utilisateur ne veut pas d'autre passe. La compilation finale a été interrompue avant la fin.

Le bureau (`C:\Users\gavri\Desktop\Handy Flow`) tourne encore le build `release` de 20:22, **sans** l'API.

## Ce qu'il reste à faire, dans l'ordre

1. Compiler : `cd Handy && bun run tauri build --no-bundle -- --profile fast` (environ 3 à 8 min). Le `target/fast/handy.exe` de 22:03 est trop ancien : il précède `4d8f426`.
2. Remplacer seulement l'exécutable :
   - fermer `Handy Flow.exe` ;
   - sauvegarder `Data\` sans `huggingface\`, plus l'ancien exe, dans `handy_remastered\backups\handy-flow-<date>-<heure>\` ;
   - copier `target/fast/handy.exe` vers `Handy Flow\Handy Flow.exe` ;
   - relancer.

   Ne jamais toucher `Data\` : il contient les clés, l'historique et les mots appris. Vérifier avec `cmp` que les DLL de `src-tauri/transcribe-libs` et `resources\` sont restées identiques.
3. Vérifier dans `Handy Flow\Data\logs\handy.log` (heures en UTC) :
   - le fournisseur passe à `groq` (migration des réglages v4, possible parce que la clé Groq existe) ;
   - à la première dictée, une ligne `Speech API 'groq whisper-large-v3' transcribed …` ;
   - `Turned on Screen Reader Optimized mode in Code` (VS Code).
4. Dire à l'utilisateur que c'est en place. Il testera lui-même les dictées : pas de test qui prend le clavier ou le micro.

## Ce que contient cette version (pour répondre à l'utilisateur)

- **Transcription par API** (`src-tauri/src/cloud_asr.rs`, branchement dans `actions.rs`) :
  - réglage dans Général → « Transcription par API », avec les mêmes clés que la reformulation ;
  - le contexte ASR (vocabulaire, expressions, app et texte) est envoyé comme prompt Whisper ;
  - repli sur le modèle local en cas d'erreur, de texte vide, d'écho du prompt ou de délai dépassé (20 s) ;
  - Échap coupe l'appel ;
  - un champ mot de passe, exclu ou non vérifié (`Capture.password_checked`) reste en local.
- **Panneaux « Dernières dictées »** dans Contexte : ce qui est parti au modèle de reconnaissance et au LLM, avec l'erreur de l'API s'il y en a une.
- **Polish, instructions vocales et transformations** reçoivent le même contexte que la reformulation (interrupteurs `transform_context`).
- **Mises à jour du Handy officiel** coupées en dur.
- **VS Code** : passage en mode lecteur d'écran au démarrage (`src-tauri/src/vscode.rs`, réglage `vscode_screen_reader`).

## Pièges rencontrés

- Le Bash de cet environnement réduit `\\` en `\` dans les heredocs : écrire les scripts d'édition dans un fichier.
- L'outil Edit interprète `\u002e`.
- Ne pas lancer `bun run format` : il réécrit tout le dépôt en LF.
- `bindings.ts` se régénère avec `cargo build` (debug) puis `target/debug/handy.exe --list-models`.

## Déploiement terminé — reprise du 2026-11-16 à 22:21 (Paris)

- `bun run tauri build --no-bundle -- --profile fast` terminé avec succès (Rust : 2 min 27 s ; frontend TypeScript/Vite OK). Aucun changement de code dans cette reprise ; pas de nouvelle revue ni de nouvelle exécution des 408 tests.
- Ancien exécutable et `Data/` hors `huggingface/` sauvegardés dans `C:\Users\gavri\Desktop\handy_remastered\backups\handy-flow-20261116-221854`. Réglages, mémoire, historique, modèles et enregistrements : 29 fichiers comparés par SHA256, identiques. Le fichier temporaire `webview/EBWebView/lockfile` a disparu après l'arrêt du WebView ; pas de différence sur les données persistantes vérifiées.
- Seul `Desktop\Handy Flow\Handy Flow.exe` remplacé. SHA256 identique au build : `58EFBAF1BB2347D949AC658D7BAA373E73EC587FCF112704F966206395FE5357`. Les 40 DLL/fichiers de ressources du bureau sont identiques au relevé avant remplacement (`runtime-sha256.json` dans la sauvegarde).
- Attention pour une prochaine reprise : les DLL du bureau ne sont pas identiques à celles de `src-tauri/transcribe-libs` régénérées par la compilation. Elles ont été conservées selon la consigne ; le démarrage initialise transcribe-cpp avec Vulkan et CPU, mais le chargement du modèle et le repli local n'ont pas été exercés.
- Application relancée à 22:21:08 ; migration persistée : `adaptive_settings_version=4`, `cloud_asr.provider_id=groq`, modèle `whisper-large-v3`, `vscode_screen_reader=true`.
- `%APPDATA%\Code\User\settings.json` contient `editor.accessibilitySupport: "on"` et a été modifié à 22:21:09. La ligne « Turned on Screen Reader… » n'apparaît pas dans le journal consulté ; le réglage lui-même est confirmé.
- Journal au démarrage : erreurs sur les trois raccourcis de transformations vides (aucune touche configurée) ; raccourcis de dictée initialisés. Pas de correction hors périmètre.
- À faire par l'utilisateur : première dictée et contrôle de `Speech API 'groq whisper-large-v3' transcribed …` dans `Data/logs/handy.log`. Aucun test prenant le clavier ou le micro n'a été lancé.
