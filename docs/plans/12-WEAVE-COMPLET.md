# Plan 12 : Weave, fil et panneau complet (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : W13 (propositions, résumé), W32. Dépend du plan 11. Durée : 1,5 j. Rien n'est codé.
**Goal :** des fils par conversation, retouchables. **Architecture :** un magasin chiffré et le panneau du plan 11 agrandi. **Tech Stack :** Rust, DPAPI, React.

## A. But

Retrouver et retoucher ses brouillons, conversation par conversation.

## B. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Un fil par conversation : application, hôte, sujet ou titre normalisé. Jamais un seul fil pour tout |
| 2 | Gardé 30 jours, chiffré (DPAPI), 50 fils au plus, effaçable un par un ou en totalité ; interrupteur « Garder les fils Weave », allumé |
| 3 | Contenu : consignes, brouillons, « collé à HH:MM ». Jamais la conversation lue |
| 4 | Panneau agrandi à la demande : 12 derniers échanges, « Plus anciens » par 20, séparateur après 2 h d'écart, défilement qui suit le bas |
| 5 | Propositions : jusqu'à 3 choix cliquables quand le modèle hésite (`<choix>`), jamais collés directement |
| 6 | Actions rapides : plus court, plus formel, en anglais (les transformations existantes) |
| 7 | Au-delà de 8 000 jetons estimés : les échanges anciens sont résumés, l'ancien résumé reste en cas d'échec |
| 8 | Retouche à la voix seulement (nouvel appui sur Ctrl+Alt+W) ; le panneau ne prend jamais le focus |
| 9 | Redimensionnable, position retenue ; sélection citée repliable |
| 10 | Réessayer, arrêter, copier à chaque étape |

## C. Ta réponse (25/11)

- **Q20 : voix seulement.**

## D. Acceptation

| Cas | Attendu |
|---|---|
| deux mails différents | deux fils |
| même mail rouvert le lendemain | fil retrouvé |
| « Tout effacer » | plus aucun fichier de fil |
| résumé impossible | fil complet gardé, notice |
| réponse interrompue | texte partiel visible, réessai possible |
| 200 échanges | panneau fluide |

## E. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/weave-fil`, créée depuis la branche du plan 11.

## F. Review Focus

1. Deux mails de même objet, destinataires différents : fils séparés si l'hôte donne un identifiant, sinon un seul, signalé. → tâche 1.
2. Horloge reculée : rien n'est effacé trop tôt. → tâche 1.
3. Fichier abîmé : mis de côté, les autres fils restent. → tâche 1.
4. Choix cliqué pendant une rédaction : ignoré. → tâche 3.
5. Application exclue après coup : ses fils sont effacés. → tâche 1.

## G. Tâches

### Tâche 1 : magasin
**Files :** Create `src-tauri/src/weave/threads.rs`.
**Produit :** `pub fn thread_key(process: &str, host: Option<&str>, thread_id: Option<&str>, subject: Option<&str>, title: &str) -> String` (condensé ; titre sans compteur « (3) » ni nom d'application) ; `struct Thread { key, process, host: Option<String>, label, turns: Vec<Turn>, summary: Option<String>, updated: i64 }` (`thread_id` : identifiant de page lu par la recette du plan 9 quand il existe, sinon `None` et objet + titre) ; `Turn { ask, draft, pasted_at: Option<i64>, at: i64 }` ; `load`, `append`, `delete`, `clear`, `prune(now)` (30 jours, 50 fils) ; un fichier chiffré par fil dans `Data/weave`.
- [ ] Tests : `the_same_thread_next_day` ; `two_subjects_two_threads` ; `a_counter_in_the_title_is_ignored` ; `a_broken_file_is_set_aside` ; `clear_leaves_no_file` ; `excluding_an_app_deletes_its_threads` ; `nothing_read_on_screen_is_stored` ; `off_stores_nothing`.
- [ ] Commit `feat(weave): threads per conversation, encrypted`.

### Tâche 2 : résumé
**Files :** Modify `weave/engine.rs`, `weave/prompt.rs`.
**Produit :** `pub fn needs_summary(t: &Thread) -> bool` (caractères / 4 > 8 000) ; garde les 6 derniers tours ; résumé demandé après la réponse, jamais avant.
- [ ] Tests : échec → ancien résumé gardé ; résumé d'un fil changé entre-temps ignoré.
- [ ] Commit `feat(weave): long threads are summed up`.

### Tâche 3 : panneau agrandi
**Files :** Modify `src/weave/`, `weave/reply.rs` (`<choix>`), `lib.rs`, traductions.
- [ ] 12 derniers tours, « Plus anciens », séparateur après 2 h, actions rapides, 3 choix au plus, Réessayer, Arrêter, Copier, redimensionnement (480 × 360 à 960 × 800, borné à l'écran), taille retenue.
- [ ] Tests : `parse` avec choix incomplets ; Playwright : 200 tours, clavier de l'application cible jamais capté sauf Tab et Échap sur un brouillon prêt, 200 % de zoom.
- [ ] Commit `feat(weave): the full panel`.

### Tâche 4 : réglages, revue, build, installation
- [ ] Page Weave : « Garder les fils », liste, effacer un, tout effacer.
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation ; partie D avec toi.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/12-v2.md`. Bugs déplacés au plan 0 : 1.1, 2.1, 3.3.
**À corriger dans ce plan**
- Marqueur « résumé jusqu'au tour N » : sans lui, le résumé se relance à chaque tour (B1, Willow).
- Budget incohérent : 8 000 jetons ici contre ~4 000 au plan 11 (C1).
- `thread_id` vient d'une recette du plan 9 qui ne le produit pas (C2) ; clé par titre fusionne des conversations (C3).
- Le collage reprendrait `auto_submit`, l'espace final et `Direct` (A3, plan 11).
- `redact` fuit dans le brouillon (A5) ; aucun délai global sans flux (A7).
**Améliorations retenues**
- Échéance en deux temps, silence + total (B2) ; Réessayer = même contexte figé (B3).
- Tour persisté seulement à la fin ; un partiel n'est jamais « brouillon précédent » (B4).
- Choix actifs seulement sur le dernier tour (B6) ; suivi du bas à 24 px, bouton retour en bas, sélection citée non répétée (B7, passe 1).
- Lignes chiffrées une à une (JSONL) (B8) ; contrôles de caractères sur les données lues (B9).
