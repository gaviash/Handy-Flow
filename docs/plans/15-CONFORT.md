# Plan 15 : confort et interface (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : A6, W20, W22, W26 (recherche), W36. **Écartées (ta réponse) : W19, W21, W37, V12.** Dépend du plan 11. Durée : 1,5 j. Rien n'est codé.
**Goal :** une barre qui se place où tu veux, des chiffres utiles. **Architecture :** position enregistrée, calculs purs. **Tech Stack :** Rust, React.

## A. But

Rendre l'usage quotidien plus agréable, sans rien ajouter que tu n'as pas choisi.

## B. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Barre : glisser (4 px avant de bouger), 5 ancrages (bas, bas gauche, bas droite, gauche, droite) plus haut actuel ; position par écran, bornée à la zone visible ; écran disparu → bas au centre |
| 2 | Clic droit sur la pastille : annuler, copier la dernière dictée, réglages, position par défaut |
| 3 | La pastille laisse passer les clics hors de ses bords, comme aujourd'hui |
| 4 | Temps gagné : mots dictés / 50 (vitesse au clavier, réglable) moins temps parlé et traité, jamais négatif ; présenté comme une estimation |
| 5 | Mots par minute : mots / durée de **parole** (VAD), dictées de plus de 3 s ; jours de suite : au moins une dictée par jour local |
| 6 | Durées de parole et de traitement enregistrées par dictée (colonnes `speech_ms`, `work_ms`, celle-ci depuis les mesures du plan 8) à partir de ce plan ; les anciennes ne comptent que pour les mots |
| 7 | Notices : une à la fois, survol = pause, réglage « Tout couper » (les erreurs restent dans l'application) |
| 8 | Recherche dans les réglages : titres et descriptions de la langue affichée, accents ignorés, ouvre la bonne page |

## C. Ta réponse (25/11)

- **Q23 : barre déplaçable et statistiques.** Recoller, Entrée dictée, vraies listes, conseils et essai guidé sont écartés.

## D. Acceptation

| Cas | Attendu |
|---|---|
| 3 dictées de 100 mots en 2 min de parole | temps gagné affiché comme estimation, jamais négatif |
| minuit passé, changement d'heure | jours de suite justes |
| pastille déplacée, écran débranché | revient sur un écran visible |
| notices coupées | aucune, erreurs visibles dans l'application |

## E. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/confort`, créée depuis la branche du plan 14.

## F. Review Focus

1. Deux écrans de zooms différents : la barre garde sa place. → tâche 1.
2. Glisser pendant une dictée : la dictée continue. → tâche 1.
3. Historique vidé : statistiques à zéro, pas d'erreur. → tâche 2.
4. Dictée de fichier audio : exclue des statistiques. → tâche 2.
5. Recherche sans résultat : le dit, au clavier comme au lecteur d'écran. → tâche 3.

## G. Tâches

### Tâche 1 : barre
**Files :** Modify `overlay.rs`, `settings.rs` (`overlay_places`), `RecordingOverlay.tsx`, `.css`, `ShowOverlay.tsx`, `lib.rs`, traductions.
**Produit :** `enum Anchor { Bottom, BottomLeft, BottomRight, Left, Right, Top }` ; `struct Place { monitor: String, anchor: Anchor, dx: i32, dy: i32 }` ; `pub fn place(p: Option<&Place>, work: Rect, size: Size, scale: f64) -> Point` (pur).
- [ ] Tests : 6 ancrages, bornes, écran absent, zoom 100, 150 et 200 %. Ancien réglage haut ou bas repris.
- [ ] Commit `feat(overlay): put the pill where you like`.

### Tâche 2 : statistiques
**Files :** Create `src-tauri/src/stats.rs` ; Modify `managers/history.rs` (`speech_ms`, `work_ms`, à chaque chemin de sauvegarde), `commands/history.rs`, `HistorySettings.tsx`, `TodayCounter.tsx`, `bindings.ts`, traductions.
**Produit :** `pub fn summary(rows: &[Row], typing_wpm: u32, today: NaiveDate) -> Summary { minutes_saved, wpm, streak, by_category }`.
- [ ] Tests : cas de la partie D, vide, changement d'heure, fichiers exclus. Commit `feat(history): time saved, pace and streak`.

### Tâche 3 : notices et recherche
**Files :** Modify `notices.rs`, `overlay.rs`, `NoticePanel`, `Sidebar.tsx` ; Create `src/components/settings/SettingsSearch.tsx`.
- [ ] Tests : file d'une notice, pause au survol, « Tout couper » ; recherche « micro », « é » et « e », aucun résultat, Entrée ouvre la page et donne le focus au réglage.
- [ ] Commit `feat(ui): one notice at a time, find a setting`.

### Tâche 4 : revue, build, installation ; partie D avec toi.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/15-v2.md`. Bug déplacé au plan 0 : 5.4.
**À corriger dans ce plan**
- Le glisser est presque inatteignable : la pastille n'existe que pendant une dictée ; ajouter un mode « Placer la barre » (C1, B4).
- « Tout couper » ne doit pas couper les erreurs « rien n'est sorti » (C3).
- Clic droit : fenêtre 266×50 trop petite, risque pour la cible du collage (C2).
- Ancrage latéral : ne pas tourner la pastille (contredit la passe 1) (C4) ; position par écran à simplifier (C5).
- Statistiques : compteurs par jour hors historique, la rétention de 30 jours les casse (A5, B6, C6) ; mots comptés sur le texte livré (A4).
**Améliorations retenues**
- Notices à niveaux : préemption et abandon des notices périmées (B1, Willow).
- Pause au survol qui garde le temps restant (B2) ; fenêtre traversée par la souris (A2, B5).
- La pastille suit la fenêtre où l'on tape, pas la souris (A6) ; barre des tâches lue (A7).
- Glisser ne déclenche pas de clic ; replacement regroupé au changement d'écran ; temps gagné à 80 mots/min minimum (passe 1, B3, B7).
