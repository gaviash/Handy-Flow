# Plan 8 : vitesse et quota Groq (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : A7 (réduite), W18 (durée), W24 (échéances), H6. **Abandonnées (ta réponse du 25/11) : W27 et V18**, l'envoi pendant la parole ; V17 écartée. Dépend des plans 2, 4 (dit → écrit) et 6 (préférences). Durée : 1,5 j. Rien n'est codé.
**Goal :** moins d'attente après la parole, quota visible. **Architecture :** mesurer, sauter la reformulation inutile, accélérer le collage, compter le quota. **Tech Stack :** Rust, reqwest, React.

## A. But

Le texte arrive vite, même après une longue dictée, et tu sais où en est ton quota.

## B. Mesures (journal, 24 dictées ; horodatage à la seconde, donc ± 1 s sauf l'ASR)

| Durée parlée | ASR Groq | De l'ASR au collage | Total |
|---|---|---|---|
| moins de 10 s (11 dictées) | 0,27 à 1,17 s | 0 à 1 s | 1 à 2 s |
| 10 à 30 s (7) | 0,48 à 1,03 s | 0 à 2 s | 1 à 3 s |
| plus de 30 s (6) | 1,00 à 2,46 s | 1 à 6 s | 2 à 8 s |

- **Correction d'une affirmation :** j'avais présenté l'envoi pendant la parole comme « le plus gros gain de vitesse ». Sous 30 s, l'ASR prend 0,3 à 1,2 s ; c'est **la reformulation** qui pèse le plus souvent le plus (1 à 2 s). L'envoi pendant la parole n'aide vraiment qu'au-delà de 30 s.
- L'attente de lecture d'écran (0,34 s dans Gmail) est traitée par le plan 2.
- Une reformulation Groq a rendu une réponse vide le 24/11 à 18:01:31 ; le secours Cloudflare a pris le relais.

## C. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Mesurer d'abord : durées en millisecondes par étape dans le journal (contrôle, ASR, reformulation, collage), sans texte |
| 2 | L'audio part en une fois, après le contrôle du relâchement : envoi pendant la parole abandonné (3 dictées sur 24 concernées, confidentialité affaiblie) |
| 3 | L'appel porte l'identifiant de session du plan 2 ; un résultat dépassé est ignoré (W24) |
| 5 | Échéance ASR : 10 s + 15 % de la durée parlée, 40 s au plus ; prolongée si la machine a dormi |
| 6 | Reformulation : variantes essayées au laboratoire (Q13) avant tout choix |
| 7 | Touche de collage tenue 10 ms au lieu de 100 (W18) ; 30 ms si un collage sur 100 échoue à l'essai réel |
| 8 | Pas de course entre deux fournisseurs (V17) : elle double le quota ; le secours actuel reste |
| 9 | Quota : lu dans les en-têtes de réponse de Groq s'ils sont là (à vérifier), sinon compté localement ; visible dans Modèles ; une notice à 80 % et à la bascule sur le secours |
| 10 | Réponse vide de la reformulation : une reprise immédiate avant le secours, si le budget de temps le permet |

## E. Tes réponses (25/11)

- **Q12 : sauter la reformulation si la dictée est vraiment propre**, dans toutes les catégories, mails compris.
- **Q13 : essai au laboratoire quand on codera**, à un moment où tu ne dictes pas. Environ 100 appels de reformulation, étalés (plus de transcription à essayer) ; ton accord redemandé juste avant.

Jugé sur le texte **brut** de l'ASR, avant le retrait local des hésitations ; jamais si un message reçu (plan 9) accompagne la dictée.

« Vraiment propre » : 1 à 12 mots ; aucun mot d'hésitation (liste `FILLERS` fr/en existante, plus « donc voilà », « en fait », « du coup », « enfin bref ») ; aucun mot répété deux fois de suite ; aucune commande de mise en forme ni correction orale (« non pardon », « virgule », « à la ligne ») ; aucune forme entendue d'un mot appris ni formule « dit → écrit » ; aucun chiffre en lettres ; ton de la catégorie « défaut » ou « formel » sans préférence écrite active ; langue reconnue. Le moindre doute → reformulation.

## F. Acceptation

| Cas | Attendu |
|---|---|
| ASR bloqué | secours local à l'échéance, texte complet |
| quota à 80 % | une notice, une seule |
| collage 100 fois | 100 réussites |
| dictée « Merci, à demain. » | collée sans reformulation, en moins de 1 s |
| « euh merci à demain » | reformulée |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `perf/vitesse`, créée depuis la branche du plan 7.

## H. Review Focus

1. Dictée courte contenant un nom mal entendu : reformulée. → tâche 2.
2. Dictée courte dans une catégorie avec une préférence écrite : reformulée. → tâche 2.
3. En-têtes de quota absents ou illisibles : compte local, jamais d'erreur. → tâche 5.
4. Réponse vide deux fois : secours, puis texte brut, jamais rien de collé à moitié. → tâche 4.
5. Horloge changée à la main : le quota du jour ne devient pas négatif. → tâche 5.

## I. Tâches

### Tâche 1 : mesurer
**Files :** Modify `actions.rs`, `llm_client.rs`, `cloud_asr.rs`.
**Produit :** `struct Timings { check, asr, rewrite, paste: Duration }`, une ligne par dictée : `dictation timings: check 12 ms, asr 430 ms, rewrite 1100 ms, paste 118 ms`.
- [ ] Test `timings_hold_no_text`. Commit `chore(perf): per-step timings`.

### Tâche 2 : sauter la reformulation
**Files :** Modify `adaptive.rs`, `actions.rs`, `settings.rs` (`skip_clean_rewrite`, défaut vrai), `PostProcessingSettings.tsx`, traductions.
**Produit :** `pub fn needs_no_rewrite(text: &str, g: &CleanGate) -> bool` ; `struct CleanGate<'a> { learned: &'a [Learned], spoken: &'a [(String, String)], tone: Tone, has_preferences: bool, language: Option<&'a str>, must_rewrite: bool }` (`must_rewrite` : posé par le plan 9 quand un message reçu accompagne la dictée, et par le plan 10 quand un nom de projet est entendu).
- [ ] `Transcription` transporte le texte brut (`raw: String`) jusqu'à `actions.rs` ; test du chemin complet `a_hesitation_removed_locally_still_counts`.
- [ ] Test en tableau : vrai pour « Merci, à demain. », « Codex. », « Alors, diagnostic, que proposes-tu ? » ; faux pour « euh merci », « le le plan », « mardi non pardon mercredi », « ouvre le point oeuvre », « demande à cloud », « rendez-vous à quinze heures », 13 mots, texte vide, langue inconnue.
- [ ] La finition locale (`finish_rewrite`, mots appris, jonction) s'applique. L'historique note « sans reformulation ».
- [ ] Commit `perf(rewrite): a clean short dictation skips it`.

### Tâche 3 : échéance
**Files :** Modify `cloud_asr.rs`, `actions.rs`.
**Produit :** `pub fn deadline(audio: Duration) -> Duration` (10 s + 15 % de la durée parlée, 40 s au plus). Le réglage `timeout_secs` devient un plafond.
- [ ] Tests : `deadline_is_10_s_plus_15_percent_capped_at_40` ; `an_old_session_is_ignored` ; veille simulée (horloge qui saute de 60 s) → échéance relancée une fois.
- [ ] Commit `fix(asr): a deadline that follows the recording's length`.

### Tâche 4 : reformulation
- [ ] Laboratoire (avec ton accord) : 20 textes ; modèle 120b et 20b, consigne actuelle et raccourcie ; mesure du temps, des mots ajoutés, des mots perdus, des noms. Résultats écrits ici.
- [ ] Changement seulement si une variante est au moins 25 % plus rapide **sans** perte sur les noms ni les demandes ; sinon rien. Réponse vide → une reprise si plus de 3 s restent. Test `an_empty_reply_is_asked_once_more_in_time`.
- [ ] Commit `perf(rewrite): …` selon le résultat.

### Tâche 5 : quota
**Files :** Create `src-tauri/src/quota.rs` ; Modify `llm_client.rs`, `cloud_asr.rs`, `notices.rs`, `ModelsSettings.tsx`, `bindings.ts`, traductions.
**Produit :** `pub fn read_headers(h: &HeaderMap) -> Option<Quota>` (`x-ratelimit-*`, noms vérifiés sur une vraie réponse avant de coder) ; compte local par modèle et par jour (jetons annoncés par la réponse, secondes d'audio envoyées), chaque tentative comptée au point de réponse, échecs et reprises compris ; cumul protégé par un verrou, écrit par fichier temporaire dans `Data/quota.json`, sans texte ; commande `get_quota`.
- [ ] Tests : `concurrent_replies_add_up` ; `headers_absent_use_the_local_count` ; `a_new_day_resets` ; `one_notice_at_80_percent`.
- [ ] Commit `feat(models): provider quota`.

### Tâche 6 : collage, revue, build, installation
- [ ] `CHORD_HOLD_MS` 100 → 10. Essai réel par toi : 100 collages dans tes applications courantes ; un échec → 30.
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation ; partie F avec toi, comparée aux mesures de la partie B.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/08-v2.md`. Bugs déplacés au plan 0 : 5.1, 5.2.
**À corriger dans ce plan**
- Config réelle : reformulation sur Cloudflare, secours identique, donc aucun vrai second essai (A2, décision 10).
- Tâche 5 (quota Groq) surdimensionnée : l'ASR consomme ~2 % du quota (décision 9).
- Décision 5 : `timeout_secs` vaut 20 s par défaut alors que la formule monte à 40 s.
- Tâche 2 : `FILLERS` n'existe pas (`UNIVERSAL_FILLER_WORDS`, `gated_filler_words_for_language`) ; « cloud » est dans le lexique.
- Rejouer la règle « dictée propre » sur `history.db` avant de coder : 58 dictées ≤ 12 mots, 13 changent de mots ; trous : majuscule initiale, ouverture de mail, guillemets parasites, noms vus à l'écran (B1, B2).
- Tâche 6 : 100 collages manuels inutiles, l'accusé arrive en 5–6 ms (A6).
**Améliorations retenues**
- Délai cloud proportionnel, course cloud contre local sans quota (A4, B4 ; plan 1 B1).
- Connexion chaude pendant la parole (A7) ; mesurer envoi et attente serveur séparément avant de compresser (B3).
- Échec ASR cloud : réessai ou modèle frère avant le local (A3) ; filtre d'hallucinations Whisper (A5).
- Maintien de touche 100 ms réduit (A6, plan 14).
