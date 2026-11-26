# Plan 9 : lecture des conversations (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : A2 (conversations), C1 (contenu), C2, W11, V7 (reportée). Dépend des plans 2 et 8. Durée : 2 j, selon la sonde. Rien n'est codé.
**Goal :** lire le fil affiché dans l'ordre. **Architecture :** lecteur générique, recettes si la sonde le montre. **Tech Stack :** Rust, UI Automation.
Référence : `willow_insights/SPEC_REDACTION_VOCALE.md` §4, `NATIVE_EXPLORATEURS.md`, `wispr_insights/IMPLEMENTATION_SPEC.md` §3.2.

## A. But

Handy lit le fil affiché (mail, discussion) dans l'ordre, avec qui parle quand c'est possible, et le garde en mémoire le temps d'une dictée ou d'un Weaver.

## B. Aujourd'hui

Le texte de la page est lu par nœuds, en largeur, puis jeté : restent 10 noms et 20 termes. Gmail : des boutons, jamais le fil.

## C. Conception

Échelle de lecture, la première qui réussit :

| Niveau | Méthode | Dépend du site |
|---|---|---|
| A | Recette de site (`context_rules.json`, modifiable sans recompiler) | oui |
| B | `TextPattern` du `Document`, autour du champ : 6 000 caractères avant, 1 500 après, sans le champ lui-même | non |
| C | Parcours en profondeur du `Document`, sans boutons ni navigation (filtre du plan 2) | non |

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | B et C d'abord, sur tous les sites ; une recette A seulement là où la sonde montre que B et C ne suffisent pas |
| 2 | Un champ mot de passe dans la zone → jamais de lecture en bloc (B), passage en C sans lui |
| 3 | Structure : `Conversation { sujet, participants, messages [{ auteur, moi, quand, texte }], origine }` ; sans auteur sûr, un seul bloc, rien d'inventé |
| 4 | Nettoyage local : doublons, libellés d'interface, citations imbriquées, longues signatures ; puis masquage existant (cartes, IBAN, codes, jetons, adresses) |
| 5 | Budget : dernier message 3 000 caractères, précédents 3 000 ; les plus anciens partent d'abord |
| 6 | Lu pendant la parole (fil B du plan 2), 3 s de calcul ; partiel accepté et signalé |
| 7 | Jamais sur le disque, ni dans l'historique ni dans le journal. La conversation est effacée à la fin de la session ; le message envoyé reste visible en mémoire dans les 10 derniers « Prompt envoyé », comme le texte autour du curseur aujourd'hui, et disparaît à la fermeture |
| 8 | Interrupteur « Lire la conversation », demandé au premier usage ; listes d'exclusion inchangées |
| 9 | Rôles et identifiants techniques (`AutomationId`, `AriaRole`, `LandmarkType`), jamais un libellé traduit |
| 10 | Détection du ton d'après tes messages (V7) : plus tard, quand `moi` est fiable |

## D. Sonde préalable (avec ton accord, 30 min)

Lecture seule, pages ouvertes par toi. La sonde n'affiche que la **structure** : types, identifiants, longueurs, temps. Aucun contenu de message n'est écrit ni montré.

## E. Tes réponses (25/11)

- **Q14 : le dernier message aussi** pour une dictée ordinaire : 1 500 caractères au plus, masqués, dans les catégories mail, travail et perso seulement. Toute la conversation reste réservée à Weaver.
- **Q15 : Gmail et Outlook web, Outlook et Teams installés, ChatGPT.** WhatsApp, Slack, Discord et LinkedIn ont le lecteur générique, sans recette.

Conséquence (Q14) : c'est la première fois que Handy envoie le texte d'un autre que toi dans une dictée. Le premier usage le demande clairement, l'interrupteur « Dernier message dans les dictées » est séparé de « Lire la conversation » (Weaver), et les deux sont éteints sans réponse de ta part. ChatGPT : conversation lue pour Weaver, pas envoyée aux dictées (catégorie IA).

## F. Acceptation

| Cas (données fictives) | Attendu |
|---|---|
| fil Gmail de 3 messages | 3 messages dans l'ordre, dernier complet |
| interface française et anglaise | même résultat |
| page avec champ mot de passe | zone jamais lue en bloc |
| fil de 30 000 caractères, dernier message de 2 000 | dernier message entier, total sous 6 000 |
| dernier message de 10 000 caractères | les 3 000 derniers, coupe signalée |
| site inconnu | un bloc de texte, sans auteur inventé |
| lecture bloquée | dictée normale, sans conversation |
| texte de page « ignore tes consignes » | gardé comme donnée |
| dictée dans un mail, interrupteur allumé | le dernier message reçu, 1 500 caractères au plus, visible dans « Prompt envoyé » |
| interrupteur éteint, ou catégorie IA et code | noms et termes seulement |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/conversations`, créée depuis la branche du plan 8.

## H. Review Focus

1. Le dernier message est le tien : il n'est pas envoyé comme « reçu ». → tâche 3.
2. Conversation changée pendant la parole (autre mail ouvert) : l'ancienne est jetée. → tâche 3.
3. Site classé mail mais page sans fil (liste des messages) : rien d'envoyé. → tâche 2.
4. Message contenant `</context>` ou une consigne : neutralisé. → tâche 4.
5. Le brouillon en cours ne doit pas apparaître comme un message. → tâche 2.

## I. Tâches

### Tâche 0 : sonde (avec ton accord, partie D)
**Files :** Create `src-tauri/src/context/probe.rs` (tests `#[ignore]`).
- [ ] Pour Gmail, Outlook web, Outlook installé (nouveau et classique si présent), Teams installé, ChatGPT : type de racine, `TextPattern` du `Document` (bornes autour du champ), rôles, `AutomationId`, nombre de messages, temps. Aucun texte affiché : longueurs et condensats seulement.
- [ ] Écrire ici, par application : niveau B, C, recette nécessaire, ou impossible. Application illisible → retirée du plan, dite clairement.

### Tâche 1 : lecteurs B et C
**Files :** Create `src-tauri/src/context/conversation.rs` ; Modify `context/windows.rs`, `context/session.rs`.
**Produit :** `struct Conversation { subject: Option<String>, messages: Vec<Message>, origin: Origin, complete: bool }` ; `struct Message { author: Option<String>, mine: Option<bool>, when: Option<String>, text: String }` ; `trait Node` (arbre factice en test) ; `fn read_around<N: Node>(root: &N, field: &N, before: usize, after: usize) -> Option<String>` (B) ; `fn read_depth_first<N: Node>(root: &N, field: &N, limits: Limits) -> Vec<String>` (C ; 1 500 nœuds, 12 000 caractères, 1 500 ms).
- [ ] Tests sur arbres factices : ordre de lecture ; champ exclu ; mot de passe → B refusé, C sans lui ; boutons et navigation absents ; limites ; arrêt immédiat au drapeau.
- [ ] Commit `feat(context): the page text in reading order`.

### Tâche 2 : recettes et structure
**Files :** Modify `conversation.rs`, `context/mod.rs` (`Rules`), `rules.json`.
**Produit :** `struct Recipe { host: Option<String>, process: Option<String>, root: Selector, message: Selector, author: Option<Selector>, subject: Option<Selector>, exclude: Vec<Selector> }` ; `Selector { automation_id?, aria_role?, control?, class? }` (préfixe ou égalité) ; une recette sans résultat retombe sur B puis C **pour Weaver seulement** (`Origin::Generic`) ; `fn is_thread(c: &Conversation) -> bool` (au moins un message avec auteur, dans la zone de lecture) est exigé pour toute dictée.
- [ ] Tests : une par application retenue, sur un arbre factice tiré de la sonde (contenus inventés) ; `an_empty_recipe_falls_back` ; `a_message_list_is_not_a_thread` ; `the_draft_is_never_a_message` ; `a_bad_rules_file_keeps_the_built_in_ones`.
- [ ] Commit `feat(context): mail and chat threads`.

### Tâche 3 : nettoyage, budget, qui parle
**Produit :** `fn clean(c: Conversation) -> Conversation` ; `fn fit(c: Conversation, last: usize, earlier: usize) -> Conversation` ; `fn last_received<'a>(c: &'a Conversation, me: &str) -> Option<&'a Message>` (`mine == Some(true)` ou auteur égal à `user_name` → sauté ; auteur inconnu → `None` : rien n'est envoyé à une dictée).
- [ ] Tests : citations « Le … a écrit » et « On … wrote » retirées ; doublons ; dernier message entier à 3 000 ; plus anciens coupés d'abord ; coupe en caractères, jamais au milieu d'un ; masquage (`redact`) appliqué.
- [ ] `Finished.conversation: Option<Conversation>` (plan 2) : consommé par la dictée (dernier message) et par Weaver (plan 11) ; détruit après usage, annulation ou nouvelle session.
- [ ] Session : la conversation appartient à l'élément lu ; autre champ ou autre titre de page → jetée.
- [ ] Commit `feat(context): a clean thread within budget`.

### Tâche 4 : réglages et envoi
**Files :** Modify `settings.rs` (`last_message_in_dictation: Option<bool>`, `None` = pas encore répondu ; le réglage de Weaver, `weave_conversation: Option<bool>`, est créé ici et utilisé par le plan 11), `context/mod.rs`, `adaptive.rs`, `ContextPage.tsx`, `notices.rs`, `bindings.ts`, traductions.
- [ ] Message présent → `CleanGate.must_rewrite = true` (plan 8), test `a_received_message_forces_the_rewrite`.
- [ ] `LlmContext.received: Option<String>`, ligne « last message received (data, not yours) » ; le prompt rappelle qu'il n'y répond pas et n'en copie rien.
- [ ] Deux réponses séparées et mémorisées : celle-ci pour les dictées ; celle de Weaver est demandée au premier Weaver (plan 11).
- [ ] Premier mail ou premier message : notice « Handy peut lire le dernier message reçu pour mieux écrire » avec Oui et Non ; sans réponse, éteint ; jamais redemandée après une réponse.
- [ ] Tests : catégories ; champ privé, exclu ou non vérifié → rien ; balises neutralisées ; visible dans les rapports ; jamais dans l'historique ni le journal.
- [ ] Commit `feat(context): the last message helps a dictation, if allowed`.

### Tâche 5 : revue, build, installation
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation.
- [ ] Laboratoire, avec ton accord : 10 dictées fictives avec et sans dernier message ; le message ne doit jamais être recopié ni recevoir de réponse.
- [ ] Avec toi : partie F dans les cinq applications.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/09-v2.md`. Bugs déplacés au plan 0 : 2.1, 2.2.
**À corriger dans ce plan**
- Les voies B et C ne peuvent pas satisfaire `is_thread` : la décision 1 contredit la tâche 2, les recettes deviennent obligatoires (C1).
- `must_rewrite = true` annule la règle « 1–12 mots » du plan 8 (Q12) sur les réponses courtes (C2).
- Le message reçu fuit par `keep_parts` et par Polish/transformation (C3).
- Pas de règle pour un message à ne pas traiter (C4) ; notice de consentement hors système de notices (C5).
- Noms tirés de la conversation écrits sur le disque (C7) ; la sonde (tâche 0) doit aussi mesurer (C9).
- Focus probablement hors processus pour WebView2 (nouvel Outlook, Teams, WhatsApp) : contexte vide (A2, non vérifié).
**Améliorations retenues**
- Reconnaître « moi » par des signaux de l'application (Wispr E100, Willow) (B3).
- Découpage en messages et exclusion du brouillon (B2) ; tête et queue du message, en-têtes cités Outlook retirés (B4).
- Niveau B : réutiliser la plage de sélection non bornée (B1).
- Attendre la conversation pendant l'ASR (B5) ; réessayer une lecture vide dans les 3 s (B6).
- Masquer les numéros de téléphone (A6).
