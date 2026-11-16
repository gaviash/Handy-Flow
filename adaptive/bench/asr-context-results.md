# Contexte ASR : mesure préalable (2026-11-16)

Banc : `src-tauri/tests/asr_context_bench.rs`, cas construits par `adaptive/bench/asr_cases.py` à partir de 20 dictations réelles de l'utilisateur (non commitées). Termes suivis : Claude, Claude Code, Handy Flow, log, commit, prompt, data, credentials, soit 7 occurrences dans les textes attendus. GPU Vulkan, langue non forcée.

| Modèle | Capacités | WER sans | WER avec | Termes sans | Termes avec | ms sans | ms avec |
|---|---|---|---|---|---|---|---|
| parakeet-ultra Q8_0 | aucune | 0.383 | 0.383 | 2/7 | 2/7 | 47448* | 36528 |
| Qwen3-ASR 0.6B Q8_0 | vocabulary, context prompt | 0.506 | 0.460 | 1/7 | 4/7 | 81855 | 79555 |

\* Le premier passage de parakeet inclut le préchauffage du GPU.

## Lecture

- **Parakeet** : le contexte ne change rien, comme attendu (aucune capacité).
- **Qwen3-ASR** : avec contexte, il écrit 3 termes appris de plus (4/7 contre 1/7), et son WER baisse de 4,6 points, sans surcoût de latence. Il passe la règle du spec (termes améliorés, WER pas dégradé de plus d'un point, latence < +15 %). **Recommandé avec contexte.**
- Malgré tout, Qwen3-ASR avec contexte reste moins bon que parakeet en WER global (0,460 contre 0,383), et deux fois plus lent.

## Limites

- Le texte attendu est le texte gardé ou réécrit, pas une transcription mot à mot. Le WER absolu est donc gonflé pour les deux modèles ; seule la comparaison avec / sans contexte compte.
- L'échantillon est petit (20 dictations, 7 termes). À refaire avec plus de dictations une fois `kept_text` rempli par l'usage.
- Whisper (prompt initial) n'a pas été mesuré : aucun modèle Whisper n'est téléchargé.

## Qwen3-ASR 1.7B, modèle de test retenu par l'utilisateur (2026-11-16, câblage final)

67 cas : 18 dictées réelles de `Handy Flow` (texte gardé ou réécrit) + 49 phrases synthétiques (voix SAPI, 41 du corpus + 8 riches en termes techniques). 48 occurrences de termes. GPU Vulkan, langue non forcée. Les WAV et les cas restent hors du dépôt.

| Modèle | Capacités | WER sans | WER avec | Termes sans | Termes avec | ms sans | ms avec |
|---|---|---|---|---|---|---|---|
| parakeet-ultra Q8_0 | aucune | 0.115 | 0.115 | 28/48 | 28/48 | 26922 | 25623 |
| Qwen3-ASR 1.7B Q8_0 | vocabulary, context prompt | 0.117 | **0.099** | 33/48 | **43/48** | 162433 | 162948 |

- Avec contexte, le 1.7B écrit 10 termes justes de plus (43/48 contre 33/48) et passe sous parakeet en WER (0,099 contre 0,115). Le contexte ne coûte rien en latence.
- Il reste environ 6 fois plus lent que parakeet : ≈ 2,4 s par dictée contre ≈ 0,4 s sur ces cas.
- Les mêmes gains que la mesure préalable (0.6B), en plus nets : la règle du spec est respectée.
