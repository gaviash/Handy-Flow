# Handoff — apprentissage adaptatif, contexte enrichi, contexte ASR (2026-11-16)

Branche : `feat/adaptive-learning-context` (depuis `main` @ `eef248ee`).
Spec : `docs/superpowers/specs/2026-11-16-adaptive-learning-context-design.md`
Plan : `docs/superpowers/plans/2026-11-16-adaptive-learning-context.md` (13 tâches)

## Ce qui est fait (tâches 1 à 12, chacune commitée avec ses tests)

| Commit | Contenu |
|---|---|
| `5c239fb` | Prompt de reformulation : suppression des répétitions orales, reprises, idées redites. |
| `8284ae5` | Notice « mots appris » en coche verte dans la fenêtre principale (plus de jaune, plus compté comme avertissement). |
| `e57856d` | Mémoire `learned.bin` : `{ words, rules }`, mots en attente, règles de style, plafonds 2000/100, ancien format relu. |
| `f93a86b` | Couche locale : mots courants mal entendus (« gag » → « log ») appris après 2 corrections ; liste de mots porteurs de sens jamais appris (jours, nombres, oui/non…) ; motif de chaque refus loggé. |
| `bef13d9` | Historique : colonne `kept_text` (texte gardé par l'utilisateur après correction). |
| `21a541a` | `recall.rs` : mots appris pertinents (≤ 40) et dictées passées similaires (≤ 5). |
| `a4a58cd` | Contexte LLM : app, titre de fenêtre (caviardé), site, 1500/500 caractères autour du curseur, dictées passées, règles de style ; prompt anti-tics. |
| `89a3198` | Juge LLM en arrière-plan (`judge.rs`, Groq `gpt-oss-20b`, 15 s, sans retry) : mot mal entendu / règle de style / tic / changement d'avis. |
| `78f5efa` | Réglages : liste des règles de style (suppression), marque « en attente ». i18n 26 langues. |
| `da3cb34` | Banc ASR `tests/asr_context_bench.rs` + `adaptive/bench/asr_cases.py` + résultats (`adaptive/bench/asr-context-results.md`). |
| `94f4413` | `asr_context.rs` : contexte ASR par capacité du modèle (vocabulaire, prompt, prompt initial Whisper). |
| `bacf9f5` | Branchement dans la transcription : rien n'est calculé ni attendu si le modèle n'accepte pas de contexte ou si le réglage est coupé. Garde-fou anti-écho étendu au prompt. |
| `fdbde7f` | Réglages « Contexte de transcription » (4 interrupteurs, 2 limites validées, état du modèle chargé). |
| dernier | `cargo fmt` sur les fichiers touchés + cette note. |

Résultat du banc ASR (20 dictées réelles) : parakeet n'accepte aucun contexte (inchangé) ; Qwen3-ASR 0.6B avec contexte passe de 1/7 à 4/7 termes justes, WER 0,506 → 0,460, latence stable.

## Ce qui reste (tâche 13 et revue)

1. **`cargo clippy --all-targets`** : interrompu avant la fin. Relancer depuis `src-tauri`, corriger les avertissements des fichiers touchés.
2. **Vérification manuelle** avec `bun run tauri dev` (fermer d'abord la version portable) :
   - corriger « gag » → « log » deux fois dans Bloc-notes : « log » doit apparaître actif dans Réglages → Apprentissage, puis être écrit directement à la dictée suivante ;
   - `Ctrl+Shift+D` → contexte envoyé : `app:`, `window:`, `past dictations:` présents ;
   - « je pense que je pense que c'est bon » → « Je pense que c'est bon. » ;
   - notice verte dans la fenêtre principale ;
   - avec Qwen3-ASR et contexte ASR actif : log `asr_context: … kind=context`.
3. **Bench de latence de la reformulation** (`adaptive/bench/scenarios.ps1`) avant/après : écart attendu < 300 ms (le prompt est plus long).
4. **Revue de toute la branche** par un relecteur neuf (skill `superpowers:requesting-code-review`), avec le ledger `.superpowers/sdd/2026-11-16-adaptive-learning-context/progress.md` (local, non versionné) pour les décisions prises.
5. **Merge** dans `main` après revue.

## Décisions prises en cours de route (à vérifier)

- Mots courants proches mais de sens différent (« Rapport » → « Rappel ») deviennent des candidats **en attente** ; ils ne s'activent qu'à la 2ᵉ occurrence. Protection supplémentaire : liste de mots porteurs de sens + juge.
- Le juge tourne sur **chaque** correction (pas seulement celles que la couche locale n'a pas prises) : un appel Groq 20b par correction.
- Les règles de style ne sont pas liées à une app (le juge n'en donne pas) : une règle apprise dans Slack s'applique partout.
- Le banc ASR n'a pas de WAV commités : il tourne sur les enregistrements de l'utilisateur via `asr_cases.py` (fichier de cas jamais commité).
- `asr_context_wanted()` reflète le modèle de la dernière dictée : la première dictée après un changement de modèle n'attend pas l'écran.
- `bindings.ts` régénéré sans interface via `target/debug/handy.exe --list-models`.

## Pièges connus de l'environnement (Windows, sans droits admin)

- Vulkan portable : `%LOCALAPPDATA%\vulkan-portable\Library` (conda-forge + headers Khronos 1.4.368), `VULKAN_SDK` et `CMAKE_PREFIX_PATH` pointent dessus. CMake portable dans `%LOCALAPPDATA%\Programs\cmake`.
- Les tests Rust ont besoin de `src-tauri/transcribe-libs` dans le `PATH` (sinon `STATUS_DLL_NOT_FOUND`).
- `managers::model::download::tests::http_cancel_during_stalled_body_keeps_partial` échoue sur cette machine, sans lien avec la branche (code non touché) : lancer avec `--skip`.
- **Ne pas lancer `bun run format` / `format:frontend`** : Prettier réécrit tout le repo en LF. Formater seulement les fichiers touchés.

## Reprise du 2026-11-16 au soir (autre machine)

- Fait : clippy (2 avertissements de la branche corrigés, les autres existaient déjà sur `main`), 398 tests unitaires OK, relecture de la branche.
- Corrigé à la relecture (`df2101e`, revue Codex) : `same_as` du juge retrouvé par le texte de la règle (une règle supprimée ou élaguée pendant l'appel décalait les numéros), et un verdict sur une règle supprimée est ignoré (elle ne revient pas) ; l'anti-écho ASR n'efface plus une dictée courte dont les mots courants sont dans le texte avant le curseur (il faut une récitation dans l'ordre). Limite connue : redire mot pour mot un passage du texte avant le curseur (3 mots ou plus) est encore pris pour un écho.
- Banc ASR refait sur **Qwen3-ASR 1.7B** (modèle de test choisi par l'utilisateur, dans `Desktop\Handy Flow`) : termes 33/48 → 43/48 avec contexte, WER 0,117 → 0,099, latence quasi inchangée, avec les termes attendus fournis au modèle (contexte favorable, pas celui de l'application) (voir `adaptive/bench/asr-context-results.md`).
- Reste : vérifications manuelles (points 2 et 3 ci-dessus), à faire par l'utilisateur ou avec son accord ; puis merge dans `main`.
