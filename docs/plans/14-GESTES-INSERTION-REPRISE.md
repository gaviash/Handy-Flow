# Plan 14 : gestes, insertion et reprise (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : A5 (reste), W14, W25, W41, W42, V3. **Écartées : W35 et W17 (tes réponses), W18 second essai (doublon impossible à exclure : après une lecture, Windows garde le texte dans le presse-papiers).** Dépend des plans 8 et 13. Durée : 1 j. Rien n'est codé.
**Goal :** ne pas perdre une dictée. **Architecture :** une annulation réversible en mémoire, un raccourci surveillé. **Tech Stack :** Rust, `handy-keys`.

## A. But

Aucune dictée perdue par un mauvais geste ou un raccourci mort.

## B. Aujourd'hui (code lu)

- `hold_or_toggle`, seuil 300 ms : un appui court lance les mains libres ; dans le journal actuel, tes 26 dictées sur 26 ont commencé par un appui court.
- Échap annule, sans retour. Audio écrit à la fin de l'enregistrement.
- Collages sérialisés, historique enregistré avant le collage, notice si le texte n'est jamais lu.
- Aucune surveillance du raccourci global trouvée.

## C. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Échap → pastille « Annulé · Rétablir » 8 s ; l'audio reste en mémoire ; le WAV en cours d'écriture est attendu puis supprimé ; « Rétablir » transcrit et colle dans le champ actuel, revérifié |
| 2 | Raccourci surveillé toutes les 5 s et au réveil ; réarmé, notice seulement si cela échoue 3 fois (V3) |
| 3 | Touche restée « enfoncée » (bureau changé, fenêtre élevée, session verrouillée) : relâchée au bout de 2 s, dictée gardée (W41) |
| 4 | Heure des touches prise dans l'événement, pas à son traitement (W25), si `handy-keys` la donne ; sinon abandonné |
| 5 | Texte non lu : texte au presse-papiers et notice, comme aujourd'hui. Pas de second raccourci automatique |
| 6 | Collage interrompu : notice « Collé en partie », texte entier au presse-papiers (W42) |
| 7 | Gestes inchangés ; aucune écriture d'audio pendant la dictée |

## D. Tes réponses (25/11)

- **Q21 : appui court comme aujourd'hui** (mains libres).
- **Q22 : pas de protection contre un plantage.**

## E. Acceptation

| Cas | Attendu |
|---|---|
| Échap puis « Rétablir » | texte collé |
| Échap dans un champ mot de passe | aucune reprise possible |
| veille puis réveil | raccourci actif sans relancer |
| fenêtre administrateur pendant la dictée | dictée terminée proprement |
| terminal qui ignore Ctrl+V | notice, texte au presse-papiers, rien en double |

## F. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/gestes-reprise`, créée depuis la branche du plan 13.

## G. Review Focus

1. Deux Échap de suite : une seule annulation, « Rétablir » encore là. → tâche 1.
2. Nouvelle dictée pendant les 8 s : l'ancienne est abandonnée. → tâche 1.
3. « Rétablir » alors que le champ est devenu privé : rien de collé ni envoyé. → tâche 1.
4. Réarmement pendant que la touche est tenue : pas de double départ. → tâche 2.
5. Échap au moment exact du relâchement : une seule copie de l'audio, une seule issue. → tâche 1.

## H. Tâches

### Tâche 1 : Échap réversible
**Files :** Create `src-tauri/src/undo_cancel.rs` ; Modify `actions.rs`, `managers/audio.rs`, `utils.rs` (annulation), `overlay.rs`, `RecordingOverlay.tsx`, `lib.rs`, traductions.
**Produit :** `struct Cancelled { session: u64, samples: Vec<f32>, at: Instant }` (une seule, en mémoire, effacée à 8 s, à la dictée suivante, au verrouillage) ; commande `undo_cancel()`. Propriétaire unique : `AudioRecordingManager` pendant l'écoute, puis `actions.rs` pendant la transcription ; le passage de l'un à l'autre se fait sous le verrou de la session, et l'annulation prend les échantillons chez le propriétaire du moment.
- [ ] Échap pendant l'écoute ou la transcription ; pas pendant un collage commencé. Contrôle du champ (plan 2) refait avant tout envoi ; champ privé, exclu ou non vérifié à l'annulation **ou** au rétablissement → rien à rétablir.
- [ ] Tests : chaque point du Review Focus 1 à 3 ; `cancel_at_the_handover_keeps_one_copy` ; `the_wav_being_written_is_removed`.
- [ ] Commit `feat(dictation): Escape can be undone`.

### Tâche 2 : raccourci vivant
**Files :** Modify `shortcut/handy_keys.rs`, `shortcut/mod.rs`, `notices.rs`.
**Produit :** `fn watch(state: &mut Watch, now: Instant, alive: bool) -> Action` (`None`, `Rearm`, `Notify`) ; vérification toutes les 5 s et à la reprise de veille, hors touche tenue.
- [ ] Étape préalable : lire `handy-keys` 0.3.5 pour savoir s'il détecte un crochet désactivé et donne l'heure des événements. Sinon : réarmement à la reprise de veille seulement, W25 abandonné, écrit ici.
- [ ] Touche restée enfoncée 2 s après un changement de bureau, un verrouillage ou une fenêtre élevée → relâchée une fois.
- [ ] Tests de `watch` avec horloge factice. Commit `fix(shortcut): a dead hook is armed again`.

### Tâche 3 : insertion
**Files :** Modify `paste_tx/mod.rs`, `paste_tx/windows.rs`, `clipboard.rs`, `notices.rs`.
- [ ] Collage interrompu → « Collé en partie », texte entier au presse-papiers.
- [ ] Tests : jamais de second raccourci ; génération plus récente jamais écrasée.
- [ ] Commit `fix(paste): a partial paste says so`.

### Tâche 4 : revue, build, installation ; partie E avec toi.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/14-v2.md`. Bugs déplacés au plan 0 : 1.3, 3.4.
**À corriger dans ce plan**
- Faux : le texte non lu n'est pas laissé au presse-papiers, Handy restaure l'ancien (A1), corrigé au plan 0 (3.4).
- Tâche 1 : un emplacement écrit à l'arrêt plutôt qu'une propriété transférée sous verrou (C1, B3 Willow).
- Tâche 2 : l'étape préalable est répondue, `handy-keys` 0.3.5 réarme déjà au réveil et au déverrouillage ; il manque l'heure d'événement et un test de vitalité (C2, B2).
- Décision 3 : le « 2 s » est un `classifierVersion` Willow, pas un délai (C3) ; décision 4 sans source (C4).
- Critères d'acceptation à réécrire (C5) ; tâche 3 « collage interrompu » sans objet (C6).
**Améliorations retenues**
- Ctrl+Maj+V choisi d'avance dans les terminaux, sans second essai (B1, passe 1).
- Maintien du chord 100 ms → quelques ms, l'accusé arrive en 5–8 ms (A6) ; attendre les modificateurs relâchés (A5).
- Pastille d'échec cliquable « Copier » (B4).
- Détection du crochet mort par comparaison état physique / événements (B2).
