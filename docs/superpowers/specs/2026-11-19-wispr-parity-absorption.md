# Cahier des charges : un apprentissage aussi efficace que Wispr Flow

**Objet** : Handy Flow doit absorber les corrections de l'utilisateur comme Wispr Flow 1.6.1483 : mêmes sources d'apprentissage, même algorithme, mêmes décisions sur les cas de référence.
**Références** : `wispr_insights/IMPLEMENTATION_SPEC.md` (§5, §8, §9, règles R1–R11), `EVIDENCE_REGISTER.md` (E54, E70–E81, E146–E150), `RESEARCH_ADDENDUM.md` (E213–E227), `fixtures/golden/cases.json` (familles `junction`, `alignment`, `learning_filter`, `observation`).
**Déjà fait** (plan `2026-11-19-wispr-parity-learning.md`, version 0.2.7) : le dictionnaire entier est envoyé à la reformulation, les mots ne disparaissent plus, le terminal est observé, les noms vus à l'écran vont au modèle de transcription.

Convention : **DOIT** = indispensable à la parité ; **DEVRAIT** = recommandé ; **PEUT** = optionnel.

---

## 1. Comment Wispr apprend (référence)

1. **Une seule source : tes corrections à la main.** Après chaque collage, Wispr observe le champ pendant 60 s, sur une fenêtre fixe (E150). L'observation se clôt aussi à la dictée suivante, quand le champ est vidé (envoi) ou quand le texte autour change (E72, E208, E214).
2. **Alignement mot à mot** entre le texte collé et le texte final. Le coût d'une substitution est la distance entre les deux mots, ce qui aligne « Jon » avec « John » (E75).
3. **Candidats** : un mot changé entouré de mots inchangés (mode « mot isolé ») ou un groupe de 1 à 4 mots (mode « phrase »). On retire les mots vides, ceux déjà au dictionnaire, les possessifs et les contractions. Les candidats avec majuscule passent en premier, 4 au maximum (E76).
4. **Validation par un LLM** (`/llm/extract_edited_words`). Il reçoit les candidats et renvoie ceux à retenir. Une réponse qui cite un candidat inconnu ou en double est rejetée entière (E76). Il garde les noms propres et les termes rares ou techniques (« postgéré » → PostgreSQL, E220). Il refuse un changement de sens (mardi → jeudi) et une faute de mot courant (projet → projets) (E221–E222).
5. **Mot retenu = actif tout de suite**, dès la 1re correction. Il est disponible à la dictée qui suit la clôture (E146, E213–E216). Une notification avec « Annuler » est affichée (E80).
6. **Aucune expiration.** L'usage est compté et sert à trier le dictionnaire (E78–E79).
7. **Ce que Wispr n'apprend pas** : la casse seule (E149), les réécritures (E148). Le style n'est pas appris non plus : c'est un réglage par catégorie d'application (formel, décontracté, très décontracté, enjoué), plus une détection de formalité limitée à WhatsApp et iMessage (E92, E100). **Aucun profil de « personnalité » n'est construit côté application.**
8. **Ce que Wispr corrige sans rien apprendre : la jonction.** Au collage, des règles déterministes adaptent la casse, la ponctuation et les espaces au texte autour du curseur (E54, spec §5). Exemple : « Peux-tu résumer ▏ en trois points ? » reçoit « le compte rendu » en minuscule et sans point (E238).

## 2. Écarts mesurés sur le poste de l'utilisateur

Historique du 16 au 19/11/2026 : 155 dictées, 48 observées, 36 modifiées après le collage, dont 28 corrections courtes.

| Corrections de l'utilisateur | Nombre | Handy aujourd'hui | Wispr |
|---|---|---|---|
| 1re lettre mise en minuscule (« Tu » → « tu », « Donc » → « donc ») | 14 | rien : Handy n'a pas de règles de jonction | aucune correction à faire : la jonction le fait au collage |
| Nom ou terme avec majuscule (sum → SOM, Jeff → Jev, Stéphane → Stefan, BrokerName → Broker Name) | 5 | appris ✅ | appris |
| Terme technique en minuscules (spikes → specs, slope → slop, l'ultra-face → ultrafast) | 3 | **en attente** d'une 2e correction | appris si le LLM le valide |
| Terme technique éloigné à l'oreille (« point oeuvre » → `.env`, corrigé 2 fois) | 2 | **rejeté** avant que le juge le voie | probablement appris (cas proche E220) |
| Changement de sens, réécriture, ajout de mots | 4 | rejeté ✅ | rejeté |

**Causes dans le code :**
- **C1** : aucune règle de jonction (spec §5 non implémentée).
- **C2** : le filtre local tranche avant le LLM. Un mot sans majuscule, chiffre ni `_` est classé « mot courant » et reste en attente. Au-delà de 0,7 de distance, il est rejeté. Le juge ne peut le promouvoir que s'il est assez proche (`learning.rs`, `candidates`, `apply_verdicts`, `close_pair`).
- **C3** : le juge LLM est un analyste général (mots, règles de style, tics). Ce n'est pas un validateur strict de candidats comme `extract_edited_words`. Ses verdicts ne sont pas journalisés : impossible de savoir pourquoi `specs` n'a pas été promu.
- **C4** : l'observation s'arrête parfois trop tôt. Exemple : « le texte autour de la dictée a changé » apprend de la dernière lecture, mais une lecture finale au moment de l'envoi n'est pas garantie (spec R7).

---

## 3. Exigences

### EX-1 — Règles de jonction de Wispr (DOIT) — causes C1

Appliquer au texte final, juste avant le collage, les règles déterministes de la spec §5, dans l'ordre :
1. **Garde** : ne rien changer si le curseur est inconnu (`caret_known = false`) ou si le contexte n'a pas pu être lu (R4).
2. **Suite de phrase** : si la ligne avant le curseur se termine par une lettre ou un chiffre (guillemets et parenthèses fermants ignorés), mettre la 1re lettre en minuscule, sauf si le mot est un nom connu (noms du contexte, dictionnaire, prénom de l'utilisateur).
3. **Texte qui suit** : si la ligne après le curseur commence par une minuscule ou par `, . : ; ! ? %`, retirer `.`, `!` et `?` en fin de texte.
4. **Sélection remplacée** : reprendre la casse et la ponctuation finale de la sélection.
5. **Espaces** : ajouter un espace avant si nécessaire (pas après une ouverture, un saut de ligne ou du CJK), et un espace après si le texte suivant commence par une lettre ou un chiffre.
6. **Point final en messagerie** selon le style : très décontracté → toujours retiré ; décontracté → retiré jusqu'à 10 phrases ; défaut + messagerie → retiré jusqu'à 2 phrases ; formel → jamais.

**Acceptation :**
- Les 12 cas `junction` de `cases.json` (J01–J12) passent en tests unitaires, J08 compris (minuscule en continuation : choix Handy).
- Rejoués sur l'historique de l'utilisateur, les 14 corrections « minuscule » sont produites par la jonction quand le texte avant le curseur était connu.

### EX-2 — Validation de chaque candidat par le LLM, comme `extract_edited_words` (DOIT) — causes C2, C3

1. **Génération locale** (déjà proche de Wispr) : alignement, groupes de 1 à 4 mots bornés par des mots inchangés, filtres de la §9.2 (mots vides, déjà au dictionnaire, possessifs et contractions, casse ou ponctuation seule). **Supprimer le rejet par distance avant le LLM** : un candidat éloigné (`.env` ← « point oeuvre ») va au LLM.
2. **Garde-fous locaux conservés** (ils tranchent seuls, sans LLM) :
   - mots de sens (jours, nombres, négations : `MEANING_WORDS`) ;
   - paires grammaticales et terminaisons verbales ;
   - troncature d'un nom (E227) ;
   - réécriture (> 30 mots, ou plus de la moitié d'une dictée d'au moins 4 mots).
3. **Appel dédié** : un prompt de validation séparé du juge de style. Entrée : `potential_words` (les candidats) et le texte avant/après. Sortie JSON stricte : `{"accepted": [{"term", "heard", "kind": "name"|"technical"|"common"}]}`. Toute réponse qui cite un candidat absent de la liste ou en double est **rejetée entière** (E76).
4. **Décision, comme Wispr** : tout candidat accepté est **actif tout de suite**, dès la 1re correction (E146). Wispr n'a pas d'état « en attente » : son validateur accepte ou refuse. D'après les essais, il garde les noms, termes rares et termes techniques (E218–E220) et refuse les fautes de mots courants et les changements de sens (E221–E222). Le prompt du validateur DOIT donc demander de refuser une correction de mot courant (grammaire, accord, synonyme) et de n'accepter que les noms, termes rares ou techniques, et les mots que la reconnaissance vocale déforme (`specs` ← spikes, `.env` ← point oeuvre). Il n'y a plus d'état « en attente » quand le validateur répond. Il ne reste que dans le repli sans LLM (point 5).
5. **Sans LLM disponible** (pas de fournisseur, délai de 15 s dépassé) : repli local de la §9.2.4, c'est-à-dire actif si le mot corrigé a une majuscule hors début de phrase, un chiffre ou `_`, ou s'il est absent du lexique (EX-4), **et** si la distance est ≤ 0,6. Sinon en attente.
6. Le juge de style actuel (règles de style, tics) reste séparé et optionnel : c'est un ajout Handy, Wispr n'a pas d'équivalent.

**Acceptation :**
- L01–L08 conformes à `expected_handy` avec une réponse LLM simulée (réponse JSON figée dans le test) et sans LLM (repli).
- Avec une réponse simulée `technical`, « point oeuvre » → `.env` est actif dès la 1re correction.
- Une réponse qui invente un candidat ne produit aucun apprentissage.

### EX-3 — Observation fiable (DOIT) — cause C4

1. **Lecture finale** sur tout événement de fin (dictée suivante, champ vidé, texte autour changé, 60 s) : relire le champ immédiatement, sans attendre le tour de 300 ms. Si la lecture échoue, utiliser la dernière lecture et marquer `final_uncertain` : **un résultat incertain n'apprend rien** (R7).
2. **Ancrage** à partir du texte avant le curseur relu juste avant le collage (R6). S'il y a plusieurs occurrences, abandonner (`anchor_ambiguous`).
3. **Fin « texte autour changé »** : apprendre de la dernière région valide seulement si le texte collé est encore présent tel quel ou si la région est restée bornée (équivalent de `anchor_mismatch_transcript_intact`).
4. **Envoi ou effacement** (DEVRAIT, spec §8.5) : un hook clavier ne retient que des catégories (`enter`, `backspace`, `select_all_delete`, `undo`), jamais les caractères. Un champ vidé après `enter` → apprendre ; après effacement ou annulation → abandonner.

**Acceptation :** O01–O08 de `cases.json` (raisons de fin) en test d'intégration sur un champ simulé ; L08 (correction puis Entrée immédiate) apprend.

### EX-4 — Lexique de fréquence fr/en (DEVRAIT) — spec R11

Une liste libre des ~50 000 mots les plus fréquents (par exemple *wordfreq*, licence MIT), en fr et en, embarquée dans l'application. Elle sert à :
- savoir si un mot corrigé est **rare** (EX-2.5) ;
- refuser un **mot courant comme forme source** (L07, « quelle » → Kelmorine) ;
- autoriser le remplacement local d'une forme entendue rare mais sans majuscule (amélioration de la tâche 7 du plan précédent).

**Acceptation :** L07 rejeté ; « Corvanti » (rare) remplaçable localement une fois confirmé ; « cloud » (courant) jamais.

### EX-5 — Réévaluer les mots en attente (DOIT)

À la mise à jour, puis à chaque démarrage tant qu'il en reste, envoyer au validateur (EX-2.3) les mots en attente créés avant cette version, en un seul appel. Ceux validés `name` ou `technical` deviennent actifs. Sur le poste de l'utilisateur : `specs`, `slop`, `ultrafast`, `jev ultrafast`, `devra correspondre`.

### EX-6 — Dictionnaire et contrôle (DOIT)

1. Chaque entrée porte sa **source** : `manual` (mots de l'utilisateur), `user_edits` (corrections), `app_context` (réservé). Tri du dictionnaire envoyé : `manual` d'abord, puis par ancienneté et par usage (E79).
2. Notification « mot appris » avec **Annuler** pendant 10 s (existe déjà) ; un mot annulé n'est plus jamais réappris de la même paire (« heard » → « term »).
3. **Chaîne de corrections** (`Orblax → Zorblax → Zorblux`) : garder les deux entrées comme Wispr (E223) et l'indiquer dans la liste des mots appris (PEUT, demande de l'interface).

### EX-7 — Traçabilité (DOIT)

1. Journal DEBUG, **sans le texte**, de chaque candidat : raison du rejet local, verdict du LLM (`name` / `technical` / `common` / refusé), décision finale.
2. Dans Réglages → Apprentissage : compteurs « corrections observées / mots appris / en attente / rejetés » sur les 30 derniers jours.

**Acceptation :** pour chaque correction de l'historique, on peut dire depuis le journal pourquoi elle a été apprise ou non.

### EX-8 — Style (PEUT, hors parité stricte)

Wispr n'apprend pas le style à partir des corrections : il applique un style choisi par catégorie, qui ne touche que la ponctuation et la casse (E246). Pour la parité :
- **DOIT** : le style par catégorie agit sur la jonction (EX-1, règle 6) dès qu'il est choisi, sans onboarding caché (à ne pas reproduire, E244) ;
- **PEUT** : garder les règles de style apprises par le juge (ajout Handy), activées après 2 observations comme aujourd'hui.

---

## 4. Critères de réussite globaux

1. Tous les cas `junction` (12), `alignment` (6), `learning_filter` (8) et `observation` (8) de `cases.json` passent en tests automatiques. Les réponses LLM sont simulées par des JSON figés : aucun appel réel à un modèle.
2. **Rejeu de l'historique réel** (test d'intégration sur un jeu anonymisé tiré des 28 corrections) :
   - 14/14 minuscules produites par la jonction quand le curseur était connu ;
   - 5/5 noms appris ;
   - `.env`, `specs`, `slop`, `ultrafast` actifs avec un validateur qui les juge techniques ;
   - 0 apprentissage sur les changements de sens et les réécritures.
3. Session manuelle d'une semaine : au moins 80 % des corrections de mots faites à la main deviennent des mots actifs ou des rejets justifiés dans le journal.

## 5. Hors périmètre

- Apprendre de l'OCR d'écran (`app_context`, E77) : désactivé par défaut chez Wispr et jamais testé.
- Détection automatique de formalité (WhatsApp et iMessage seulement chez Wispr).
- Synchronisation cloud du dictionnaire ou dictionnaire d'équipe.
- Apprendre d'une **nouvelle dictée** qui reformule la précédente, sans correction à la main : Wispr ne le fait pas non plus.

## 6. Décisions et question ouverte

1. **Q1 — Fournisseur du validateur : tranché.** C'est une fonction comme les autres. L'utilisateur choisit son fournisseur et son modèle parmi ceux configurés, ou « Automatique » (le choix de la reformulation). Voir `specs/2026-11-19-providers-per-feature.md`, à livrer **avant** EX-2.
2. **Q2 — Mots courants : tranché, comme Wispr.** Pas d'état « en attente » quand le validateur répond : accepté = actif, refusé = rien (EX-2.4). Le validateur refuse les fautes de mots courants, comme Wispr (E222).
3. **Q3 — Jonction dans le terminal : tranché, comme ailleurs.** Les règles EX-1 s'appliquent aussi au terminal (casse, ponctuation, espaces), avec la même garde : rien si le curseur est inconnu.
