# Plan 13 : audio et micro (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : W15, W16, W34, W38, W26 (essai micro), V1, V2. Dépend du plan 8. Durée : 1 à 1,5 j. Rien n'est codé.
**Goal :** dire tout de suite qu'on n'entend pas, et pourquoi. **Architecture :** des fonctions pures sur les échantillons, branchées sur la capture. **Tech Stack :** Rust, Silero existant, React.

## A. But

Quand Handy n'entend pas bien, il le dit tout de suite et explique pourquoi.

## B. Aujourd'hui (code lu)

- Notice « micro silencieux » **après** une dictée vide. Micro par défaut de Windows suivi (F106). 120 ms gardées après le relâchement. Aucune limite de durée trouvée. Son coupé pendant la dictée en option, état d'avant rétabli.

## C. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Aucun son **depuis le début** de la dictée (zéros numériques) : alerte discrète à 5 s, nette à 15 s. Après un premier son : alerte nette seulement, à 15 s de zéros (micro coupé en route), jamais pour une pause plus courte (V1) |
| 2 | Fin de parole : jusqu'à 300 ms après le relâchement, au moins 120 ms, puis coupée dès 120 ms de silence continu (Silero déjà chargé) ; jamais plus long que le réglage actuel si tu l'as changé (V2) |
| 3 | Dictée vide d'au moins 1 s : diagnostic local — bande étroite (entrée ≤ 8 kHz ou rolloff ≤ 3 800 Hz), silence, saturation, voix faible, parole sans mots — avec cause, « Réécouter » et « Réglages du micro » (W15) |
| 4 | Après une dictée réussie mais mauvaise : un seul avertissement par session et par cause, voix faible à la 3ᵉ fois, « Ne plus afficher » |
| 5 | Micro débranché pendant la dictée : ce qui est déjà enregistré est transcrit ; micro suivant proposé (intégré, filaire, sans fil), retour au premier s'il revient (W16) |
| 6 | Nouveau micro : « Utiliser ce micro ? » 10 s ; sans réponse, rien ne change |
| 7 | Moins de 300 ms de parole : « Trop court » au lieu de rien ; avertissement 30 s avant 20 min, arrêt et transcription à 20 min (W34) |
| 8 | Micro gardé prêt entre deux dictées (`lazy_stream_close`) : mesuré d'abord, activé par défaut seulement si le démarrage gagne plus de 100 ms |
| 9 | Essai micro avec onde dans Général (W26) ; l'audio d'essai n'est jamais enregistré |
| 10 | Filtre de bruit de fond : pas de traitement ajouté (effet incertain sur Whisper) |

## D. Questions

Aucune. Les seuils s'inspirent de ceux relevés chez Willow et Wispr ; ils sont à vérifier sur tes enregistrements.

## E. Acceptation

| Cas | Attendu |
|---|---|
| micro coupé avant de parler, 6 s | alerte pendant la dictée |
| pause de 10 s en parlant | aucune alerte |
| dernier mot dit en relâchant | gardé |
| micro débranché à 5 s | 5 s transcrites, notice |
| micro Bluetooth en mode appel | cause affichée |
| casque branché | question 10 s, rien ne change sans réponse |

## F. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/audio-micro`, créée depuis la branche du plan 12.

## G. Review Focus

1. Micro qui envoie un bruit de fond constant : ni alerte de silence, ni fin de parole bloquée. → tâches 1 et 2.
2. Réglage « tampon après relâchement » changé par toi : respecté. → tâche 2.
3. Champ privé : « Réécouter » n'existe pas, l'audio est jeté. → tâche 3.
4. Micro virtuel (câble audio) : jamais proposé en secours avant un vrai micro. → tâche 4.
5. Enregistrement de 20 min : mémoire bornée, texte complet. → tâche 5.

## H. Tâches

### Tâche 1 : silence pendant la dictée
**Files :** Create `src-tauri/src/audio_toolkit/health.rs` ; Modify `managers/audio.rs`, `overlay.rs`, `RecordingOverlay.tsx`, traductions.
**Produit :** `pub struct SilenceWatch` ; `pub fn feed(&mut self, samples: &[f32]) -> Option<Silence>` (avant tout son : `Soft` à 5 s, `Loud` à 15 s ; après un son : `Loud` à 15 s de zéros d'affilée ; une fois chacun).
- [ ] Tests : zéros 5 s et 15 s ; souffle à −60 dB → rien ; son puis 10 s de zéros → rien ; son puis 15 s → `Loud`. Commit `feat(audio): no sound is said while speaking`.

### Tâche 2 : fin de parole
**Files :** Modify `managers/audio.rs:1250`, `settings.rs`.
**Produit :** `pub fn tail(samples_after_release: &[f32], vad: impl FnMut(&[f32]) -> f32, max: Duration) -> Duration` ; entre 120 et 300 ms, arrêt après 120 ms continues sous 0,35. Test : 80 ms de silence puis un mot → le mot est gardé.
- [ ] Migration : 120 ms (valeur par défaut) → automatique ; toute autre valeur gardée telle quelle.
- [ ] Vérification sur tes enregistrements sauvegardés : durée moyenne ajoutée ; si elle dépasse 150 ms, plafond à 200 ms.
- [ ] Commit `feat(audio): the last word is kept, silence is not`.

### Tâche 3 : diagnostic
**Produit :** `pub enum Cause { NarrowBand, Silence, Clipping, Quiet, NoWords }` ; `pub fn diagnose(samples: &[f32], device_rate: u32, speech: f32) -> Cause`, dans cet ordre : entrée ≤ 8 kHz ; parole VAD < 5 % ; > 5 % d'échantillons saturés (≥ 0,985, 3 de suite) ; rolloff 95 % ≤ 3 800 Hz ; < −45 dB ; sinon sans mots.
- [ ] Tests sur signaux fabriqués, un par cause, plus voix normale. Notices avec « Réécouter » (mémoire, 2 min) et « Micro » ; avertissement doux : une fois par cause et par session.
- [ ] Commit `feat(audio): why a dictation came out empty`.

### Tâche 4 : micros
**Produit :** `pub fn next_device(lost: &Device, left: &[Device]) -> Option<Device>` (intégré, filaire, sans fil, virtuel, inconnu) ; notice « Utiliser <micro> ? » 10 s, Oui et Non ; sans réponse, rien.
- [ ] Tests : ordre, retour du premier, aucun micro, micro choisi à la main jamais remplacé sans Oui. Micro perdu en cours : échantillons déjà pris transcrits.
- [ ] Commit `feat(audio): a lost or new microphone`.

### Tâche 5 : durées, essai micro, micro prêt
- [ ] Trop court (< 300 ms de parole) ; 19 min 30 → avertissement ; 20 min → arrêt et transcription. Essai micro avec onde dans Général, jamais enregistré. Mesure de `lazy_stream_close` sur 20 démarrages simulés par le journal ; réglage changé seulement si le gain dépasse 100 ms.
- [ ] Commit `feat(audio): limits and a microphone test`.

### Tâche 6 : revue, build, installation ; partie E avec toi.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/13-v2.md`. Bugs déplacés au plan 0 : 3.1, 3.2.
**À corriger dans ce plan**
- Tâche 3 impossible : `samples` est déjà filtré par le VAD, une dictée vide donne un tampon vide. Analyser l'audio brut en direct, comme Willow (A3, C5, B2).
- #8 `lazy_stream_close` par défaut déclencherait le bug du volume (C4) : seulement après la correction 3.2.
- #6 « Utiliser ce micro ? » contredit le suivi du micro par défaut (F106) : seulement pour un micro choisi à la main (C2, B9).
- Tâche 2 : « drapeau de relâchement dans le sink », pas un second VAD (A6, C3).
- Le « seuil de silence pendant la dictée » vient de Wispr, pas de Willow (C1) ; seuil de parole incohérent (A5).
- Micros à porte de bruit : fausses alertes de silence (A8) ; son de fin dans la queue enregistrée (A7) ; mémoire et envoi pour 20 min (A9).
- Passe 1 « échec propre à 5 s » remplacé par la correction 3.1.
**Améliorations retenues**
- Bascule automatique vers le micro de secours en cours de dictée (B1, Willow).
- Avertissements doux, un par cause, « voix faible » à la 3ᵉ fois (B4) ; « micro utilisé » au démarrage (B5).
- « Trop court » sur la durée de maintien (B6) ; échéance de durée avec avertissement 30 s avant (B7) ; identifiant de session (B8).
