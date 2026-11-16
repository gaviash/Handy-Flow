# Suivi — Handy Flow : branding, thèmes et finitions

Branche locale : `feat/handy-flow-brand`. Plan : `docs/superpowers/plans/2026-11-16-handy-flow-brand.md`. Spec : `docs/superpowers/specs/2026-11-16-handy-flow-brand-design.md`. Les tâches 0 à 15 sont terminées.

## Livré

- Identité Handy Flow, fontes embarquées, huit palettes clair/sombre, icônes et updater amont désactivé. Identifiant, chemins de données et marqueur portable conservés.
- Écran Apparence : palettes avec aperçu propre, mode Système/Clair/Sombre, titres Éditorial/Moderne, densité et styles Pastille/Onde/Live. Application immédiate et persistance via les commandes Rust.
- Primitives, focus visible, sélection, scrollbars, contraste des boutons et réduction du mouvement ; navigation au clavier des sélecteurs.
- Overlay recoloré à chaud : libellé d’écoute en Pastille, masqué en Onde, nom accessible conservé. Les neuf barres restent pilotées par le vrai niveau audio ; dimensions natives inchangées.
- Carte de raccourci en tête, raccourci accessible avec bouton/kbd, actions d’historique au survol et au focus, titres de page et crédit MIT à Handy dans À propos.
- Icône de fenêtre et icône colorée du tray liées à la palette. Les variantes monochromes et les signaux d’enregistrement restent en place.

## Vérifications

- Rust : `cargo test --manifest-path src-tauri/Cargo.toml --profile fast --lib` : **419 réussis, 23 ignorés préexistants, 0 échec** (DLL de transcribe-libs dans PATH).
- Playwright headless : **45/45**, nouvelle exécution sans mise à jour des captures après leur examen. Captures Général/Apparence des 8 palettes × 2 modes, fenêtre française minimale 720 × 600, états de l’overlay Rose/Sauge/Contraste.
- Assertions Bun : apparence, clavier, installateur portable et presse-papiers réussies. Le test presse-papiers journalise intentionnellement son erreur simulée.
- `check:branding`, `check:themes`, `check:translations`, TypeScript et ESLint réussis.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --profile fast` réussi avec avertissements préexistants (20 dans la bibliothèque, dépendance rdev inutilisée dans le manifeste).
- Formatage ciblé Prettier et `cargo fmt --check` réussis. **Le contrôle global `bun run format:check` reste en échec sur 265 fichiers préexistants**, notamment à cause des fins de ligne du checkout Windows. Pas de reformatage massif.
- Revue indépendante de toute la branche : quatre points corrigés (rangée Onde, libellé Pastille, fixture Windows limitée à Windows, titre Modèles dupliqué), avec tests de régression.
- Impeccable était déjà installé dans `C:\Users\gavri\.agents\skills\impeccable`. Context et craft-floor chargés, détecteur exécuté : rebond de l’overlay corrigé. Deux alertes de bordure concernent le padding transparent des scrollbars ; deux alertes de police concernent Instrument Serif, imposée par la référence validée.

## Builds et limites

- Incrément 1 validé lors de la session précédente ; incrément 2 réussi en 2 min 13.
- Incrément 3 : **réussi en 2 min 15**. Sortie : `src-tauri/target/fast/Handy Flow.exe`.
- Les DLL et `resources/` ont été copiés à côté de l’exécutable, dans le worktree uniquement, puis vérifiés par SHA-256. L’exécutable n’a pas été lancé.
- **Aucune copie, installation ou modification dans `Desktop\Handy Flow` ou `Desktop\Handy`.** Le checkout principal et son travail cloud_asr restent intacts. Aucun push, PR ou merge.
- Aucun lancement de l’app native pour test, aucune prise du clavier, du focus ou du microphone. Les tests de navigateur sont headless et simulent l’IPC Tauri.
- macOS/Linux : revue des chemins et compilation conditionnelle ; pas de compilation ni de contrôle visuel sur ces systèmes. La fixture IPC est explicitement Windows.
- Police embarquée limitée au sous-ensemble latin ; les autres écritures gardent leur repli système, comme décidé précédemment.
- Pas de carte de statut du modèle supplémentaire dans la barre latérale : le footer existant l’affiche déjà.

## Vérifications manuelles restantes

À effectuer sur le build du worktree, au moment choisi par l’utilisateur :

1. Nom Handy Flow dans le Gestionnaire des tâches, titre, infobulle et icône de la barre des tâches.
2. Une seule entrée de démarrage pour cette installation ; Handy d’origine intact.
3. Changer de palette pendant que la pastille est ouverte, puis dicter : couleur et résultat corrects ; tester Pastille, Onde et Live.
4. Clair, Sombre et Système suivent le choix et Windows ; réduction des animations respectée.
5. Historique, modèles, réglages et clés API toujours disponibles dans l’installation utilisée.
6. Tray lisible sur barre claire et sombre ; icône de fenêtre suivant la palette.

Le journal détaillé reste dans `.superpowers/sdd/2026-11-16-handy-flow-brand/progress.md` (ignoré par git).
