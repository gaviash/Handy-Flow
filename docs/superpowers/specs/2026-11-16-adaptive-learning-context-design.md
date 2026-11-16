# Apprentissage adaptatif et contexte enrichi — design

Date : 2026-11-16 · Statut : à approuver

## Intention

Handy Flow doit donner l'impression de savoir ce que l'utilisateur veut dire, au niveau de Wispr Flow. Aujourd'hui :

- l'apprentissage ne retient que les noms et les termes techniques (majuscule hors début de phrase, chiffre ou `_`). Une correction de mot courant mal entendu (« gag » → « log ») est ignorée, et aucune préférence de formulation n'est apprise ;
- le LLM reçoit peu de contexte : une catégorie (`email`, `work_chat`…) au lieu du nom de l'app, 300 caractères avant le curseur et 150 après, au plus 30 termes appris, et aucune dictée passée ;
- les mots appris sont envoyés sous forme de liste et le LLM les ignore parfois (dictées #12 et #13 : « cloud » non corrigé alors que « Claude » était appris).

## Objectifs

1. Apprendre trois sortes de corrections :
   - **noms et termes** (déjà en place) ;
   - **mots mal entendus**, même courants (« gag » → « log ») ;
   - **règles de style** : formulations préférées, tics à supprimer, tournures (« de manière claire et concise »).
2. Envoyer au LLM un contexte riche mais pertinent : nom de l'app, titre de fenêtre, domaine, plus de texte autour du curseur, dictées passées similaires, mots et règles appris pertinents.
3. Supprimer les tics de langage par défaut, et apprendre ceux propres à l'utilisateur pour les supprimer aussi.
4. Garder une latence de dictée inchangée : tout apprentissage par LLM se fait en arrière-plan, après le collage.

## Non-objectifs

- Corriger le cyrillique sur les dictées courtes. Garde-fou à faire à part.
- Synchronisation multi-machine (l'export JSON existe déjà).

## Anti-objectifs (échec même si ça « marche »)

- Un changement d'avis appris comme une règle (« mardi » corrigé en « jeudi » devient permanent).
- Un prompt qui grossit à chaque dictée jusqu'à ralentir ou dégrader la reformulation.
- Une règle apprise que l'utilisateur ne voit pas et ne peut pas supprimer.

## Contraintes

- Choix de l'utilisateur (option A) : tout le contexte peut partir chez Groq, sauf les apps exclues et les champs sensibles (filtrage déjà en place dans `llm_context`).
- La mémoire reste chiffrée DPAPI au repos (`learned.bin`, D11). Les logs restent anonymisés (longueurs, jamais le texte).
- Fork d'upstream Handy : le code nouveau reste dans les fichiers adaptatifs (`learning.rs`, `adaptive.rs`, `context/`) autant que possible.
- Windows uniquement pour ce fork.

## Approches envisagées

1. **Règles locales assouplies, sans LLM.** On apprend les mots courants s'ils sont proches phonétiquement et si la correction revient 2 fois. Gratuit et local, mais n'apprend jamais une règle de style.
2. **Un LLM juge chaque correction.** Couvre tout, mais un appel par correction, même pour les noms que le code repère déjà seul.
3. **Hybride.** Le code apprend les noms et termes, le LLM juge le reste.

**Choix de l'utilisateur : 3 + 1.** La couche locale apprend les noms et termes (actuel) **et** les mots mal entendus courants (approche 1). Le LLM juge en arrière-plan uniquement les corrections que la couche locale n'a pas prises, et en tire des règles de style. Avantage : les mots mal entendus s'apprennent même sans réseau.

## Design

### 1. Couche locale (`learning.rs::candidates`)

- On garde le filtre actuel pour les noms et termes.
- Nouveau type **mot mal entendu** : une substitution de 1 à 3 mots, sans majuscule, chiffre ni `_`, est retenue si :
  - les deux formes sont proches : distance de caractères ≤ 0,7 (« gag » / « log » = 0,67) ;
  - aucune des deux formes n'est un mot porteur de sens figurant dans une liste fixe : jours, mois, nombres en lettres, oui/non/si, négations (ne, pas, jamais, rien), pronoms personnels. C'est ce qui exclut « mardi » → « jeudi » (distance 0,6). *(Correction du 2026-11-16 : la distance seule ne sépare pas ces deux cas, et la clé phonétique est abandonnée.)*
  - ce n'est pas une troncature ni un pluriel (règle `affix` existante) ;
  - la paire est vue **2 fois** avant d'être active. Une première occurrence est stockée en attente (`count = 1, active = false`).
- Chaque refus est loggé avec un motif court sans texte (`rejected: too_far`, `rejected: rewrite`, `rejected: no_substitution`…), pour diagnostiquer ce que l'analyse des logs n'a pas pu montrer.

### 2. Juge LLM en arrière-plan (nouveau, dans `learning.rs` ou `learning/judge.rs`)

- Déclencheur : fin d'observation avec `edited: true`, quand la couche locale n'a rien appris de la correction, ou une partie seulement.
- Appel Groq, modèle `adaptive::GROQ_SPARE_MODEL` (`gpt-oss-20b`), avec la clé Groq déjà configurée, hors du chemin de dictée, délai max 15 s, aucun retry. *(Décision de Claude : modèle rapide fixe, indépendant du fournisseur de reformulation.)* Sans clé Groq, le juge est désactivé : seule la couche locale apprend.
- Entrée : texte collé, texte final gardé par l'utilisateur, nom de l'app, règles de style existantes, 5 dictées passées similaires.
- Sortie JSON stricte :
  - `kind` : `misheard` | `style_rule` | `filler` | `content_change` | `unclear` ;
  - pour `misheard` : `heard`, `term` ;
  - pour `style_rule` / `filler` : `rule`, une instruction courte et générale en français (« Préférer "de manière claire et concise" à "clairement" ») ;
  - `consistent_with_history` : booléen.
- `content_change` et `unclear` : rien n'est appris.
- Un `misheard` du juge suit **les mêmes règles que la couche locale** : contrôle de proximité à l'oreille, puis activation à 2 occurrences. Le juge ne peut pas activer seul un mot. C'est ce qui protège de « mardi » → « jeudi ». *(Décision de Claude.)*
- Une règle est **active** si elle est vue 2 fois (le juge la reconnaît comme identique à une règle en attente), ou dès la première fois si `consistent_with_history = true`. Sinon elle reste en attente.
- Une réponse invalide ou un échec réseau est ignoré et loggé.

### 3. Mémoire

- `Learned` gagne `kind` (`term` | `misheard`) et `active` (par défaut `true` pour les entrées existantes, rétrocompatible).
- Nouvelle liste `StyleRule { rule, count, active, last_seen, apps: Vec<String> }` dans le même fichier chiffré : `learned.bin` passe d'un tableau à un objet `{ words, rules }`, et l'ancien format est lu comme `{ words: [...], rules: [] }`.
- Plafonds de stockage : **2000 mots, 100 règles**. Au-delà, on supprime l'entrée active la moins utilisée et la plus ancienne. Les entrées en attente non revues depuis 30 jours expirent.

### 4. Ce qui est envoyé au LLM à chaque dictée

Le bloc `<context>` de `house_user_message` gagne :

- `app` : nom lisible de l'app (`process`), `window` : titre de fenêtre, `site` : domaine (déjà capturés, pas envoyés) ;
- texte autour du curseur : **1500 caractères avant, 500 après** (contre 300 et 150) ;
- `past dictations` : au plus **5** dictées passées, le texte final gardé par l'utilisateur, sélectionnées ainsi :
  - les 3 plus proches lexicalement de la transcription brute (mots communs pondérés, calcul local) ;
  - les 2 plus récentes dans la même app ;
  - celles de moins de 30 jours, chacune tronquée à 300 caractères ;
- `terms` : au plus **40** mots appris actifs, choisis par proximité avec la transcription (une forme entendue proche d'un mot dicté passe en premier), puis par fréquence. Format conservé : `log (heard as "gag")` ;
- `style rules` : toutes les règles actives (≤ 100, typiquement < 20), filtrées par app si la règle est liée à des apps.

Portée *(décision de Claude)* : comme aujourd'hui, ce contexte enrichi ne concerne que le prompt maison (`HOUSE_PROMPT_ID`). Un prompt personnalisé n'est pas touché, et le chemin d'upstream reste inchangé. Les dictées passées et les règles ne sont envoyées que si `context_send_enabled` **et** `learning_enabled` sont actifs.

Le prompt (`HOUSE_PROMPT`, règle 10) est complété :

- les dictées passées montrent le vocabulaire et les tournures de l'utilisateur. Elles servent à reconnaître ce qu'il veut dire, jamais à être recopiées ;
- un mot dicté qui ressemble à une forme « heard as » est remplacé par le terme : la règle passe de « sound like » à « must » quand la forme entendue correspond ;
- les règles de style s'appliquent sauf si elles contredisent le sens de la dictée ;
- les tics de langage (« donc voilà », « en fait », « du coup » en ouverture ou en béquille) sont supprimés.

### 5. Historique

- La dictée stocke aussi le **texte final gardé** par l'utilisateur quand l'observation l'a lu : nouvelle colonne `kept_text` dans `transcription_history`, migration ajoutée à la liste existante.
- Rétention inchangée *(choix de l'utilisateur)* : le mode Windows `Days30` garde 30 jours de texte, ce qui suffit pour les dictées passées. `history_limit` et le nettoyage d'upstream ne sont pas touchés. Les mots et règles appris ne dépendent pas de cette rétention.

### 6. Interface

- Réglages → Apprentissage : deux listes, « Mots » (existante, plus le type et l'état en attente) et « Règles de style » (nouvelle), chacune avec suppression unitaire. L'annonce sur la pilule existante (« appris : … ») couvre aussi les règles activées.
- Textes via i18n (`en` d'abord, puis les autres locales, comme l'exige `check:translations`).

### 7. Contexte envoyé au modèle ASR (ajout du 2026-11-16, demandé par l'utilisateur)

**Intention :** que le modèle de transcription entende juste dès le départ (« log », « Claude », expressions habituelles), au lieu de compter sur la reformulation pour réparer. Tout est paramétrable, et l'effet est mesuré avant d'être activé par défaut.

**Ce que le moteur permet** (`transcribe-cpp` 0.4.2, `RunOptions`) :
- `vocabulary: Vec<String>` pour les modèles qui ont `Feature::Vocabulary`. Déjà câblé, mais seulement avec les mots personnalisés ;
- `prompt: Option<String>` pour les modèles qui ont `Feature::ContextPrompt` (texte de conditionnement, Qwen3-ASR probablement). Non câblé ;
- `InitialPrompt` (Whisper), exposé par l'extension de run Whisper. Non câblé.
- Parakeet (modèle actuel) n'a aucune de ces capacités : pour lui, le contexte ASR ne change rien. C'est attendu, il n'y a rien à corriger.

**Sources du contexte ASR :**
- mots personnalisés, puis mots appris actifs (le terme seul, les plus fréquents d'abord) ;
- expressions fréquentes de l'utilisateur : n-grammes de 2 à 4 mots vus au moins 3 fois dans les dictées des 30 derniers jours (texte gardé), hors mots vides seuls ;
- en option, l'écran : nom de l'app et 200 caractères avant le curseur. La capture de contexte tourne aujourd'hui **en parallèle** de la transcription (`actions.rs:1007` et `:1149`). L'utiliser oblige l'ASR à l'attendre, avec un délai borné.

**Formatage par capacité :**
- `Vocabulary` → liste de termes, plafonnée ;
- `ContextPrompt` / `InitialPrompt` → une phrase de conditionnement courte, dans la langue de la dictée, qui cite les termes et les expressions (et l'app et le texte précédent si l'option est active), plafonnée en caractères (Whisper limite son prompt à environ 224 tokens) ;
- aucune capacité → rien n'est construit et rien n'est loggé en avertissement.
- Le garde-fou existant `echoes_vocabulary` (l'ASR qui récite la liste au lieu de transcrire) s'étend au prompt.

**Paramètres** (Réglages → Apprentissage → « Contexte de transcription ») :
| Réglage | Défaut |
|---|---|
| `asr_context_enabled` | `true` |
| `asr_context_learned_words` | `true` |
| `asr_context_expressions` | `true` |
| `asr_context_screen` (app + texte avant le curseur) | `false` (ajoute de la latence) |
| `asr_context_max_terms` | `50` |
| `asr_context_capture_wait_ms` | `150` |

**Factorisation :** un module pur `asr_context.rs` (`AsrCaps`, `AsrSources`, `AsrContextSettings` → `AsrContext { vocabulary, prompt }`), sans dépendance à Tauri, testable seul. `transcription.rs::run_setup` le consomme ; `worker.rs` sonde en plus `Feature::ContextPrompt`. Les expressions fréquentes sont calculées dans `recall.rs`, à côté des dictées similaires.

**Mesure préalable :** un banc d'intégration rejoue des enregistrements réels (WAV de `data/recordings` avec le texte gardé de `history.db`, ou des fixtures) sur chaque modèle disponible, avec et sans contexte. Il mesure le WER global, le taux de termes appris bien écrits et la latence. Le contexte ASR n'est recommandé pour un modèle que si les termes s'améliorent sans dégrader le WER de plus d'un point, ni la latence de plus de 15 %.

## Laissé au builder

- La mesure de proximité lexicale pour les dictées passées (Jaccard sur les mots hors mots vides convient).
- Le texte exact du prompt du juge, tant qu'il respecte le schéma JSON ci-dessus.
- Le découpage en fichiers à l'intérieur de `learning`.

## Tests attendus

- `candidates` : « gag » → « log » vu 2 fois donne un mot actif ; « mardi » → « jeudi » ne donne rien ; les noms et termes actuels ne régressent pas (tests existants).
- Désérialisation de l'ancien `learned.bin` (tableau) vers le nouveau format.
- `house_user_message` : présence et plafonds de `app`, `past dictations`, `terms` et `style rules`.
- Juge : JSON invalide → rien d'appris ; `content_change` → rien d'appris.
- Bench existant (`adaptive/bench`) : relancé pour vérifier la latence et la qualité de reformulation avant et après.
