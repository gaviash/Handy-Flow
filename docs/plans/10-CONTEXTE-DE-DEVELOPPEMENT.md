# Plan 10 : contexte de développement (2026-11-25)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : A2 (éditeurs), C6, W12 (sans `@`), H2 (noms et branches), V19. Dépend des plans 2 et 8. Durée : 1,5 j. Rien n'est codé.
**Goal :** les noms du projet ouvert écrits juste. **Architecture :** un index en mémoire, une recherche par le son. **Tech Stack :** Rust, git en ligne de commande.

## A. But

Dans VS Code et le terminal (195 de tes 254 dictées au 24/11), Handy écrit juste les fichiers, les branches et les termes du projet ouvert.

## B. Constats

- Termes envoyés depuis VS Code : des libellés et des titres de conversations (« GPT-6.1 Terra Élevé », « Préparer Capacitor », « Peux-tu », « Ouvre »). Depuis le terminal : la barre d'état de l'agent (« GPT-6.2-Sol », « PM », « Localiser »).
- Le titre de VS Code donne le fichier et le nom du projet, pas son chemin.
- Le terminal affiche souvent le chemin (`PS C:\…>`).
- VS Code est réglé en mode lecteur d'écran par Handy, ce qui rend le champ lisible.

## C. Conception

| # | Décision (mienne, corrigeable) |
|---|---|
| 1 | Projet : chemin lu dans l'état de VS Code (`storage.json`, dossiers ouverts, rapproché du titre) ou dans l'invite du terminal ; doute → aucun projet |
| 2 | Index selon Q17 ; `git ls-files` si c'est un dépôt, sinon parcours borné ; `.gitignore` respecté, même pour un fichier déjà suivi (`git ls-files -ci --exclude-standard` retiré de la liste) ; jamais `.env`, clés, `node_modules`, `target`, `dist` |
| 3 | 5 000 noms au plus, refait quand `HEAD` ou l'index git change, sinon au bout de 10 min ; en mémoire seulement |
| 4 | Envoyés à Whisper : 25 noms au plus, choisis par proximité (fichier ouvert, même dossier, récents, déjà dictés). À la reformulation : ceux dont le son ressemble à la dictée |
| 5 | Libellés d'interface de VS Code et du terminal écartés : seul le texte du champ, le titre et le projet comptent |
| 6 | Agent reconnu dans le terminal (Claude Code, Codex) par le nom du processus enfant : sert à ignorer son invite et sa barre d'état |
| 7 | Pas d'étiquette `@fichier` automatique : le nom est écrit, tu ajoutes `@` toi-même |
| 8 | Application exclue, ou dossier hors de ton profil utilisateur **une fois les liens et jonctions résolus** : aucun index ; un lien vers l'extérieur n'est jamais suivi |

## E. Ta réponse (25/11)

- **Q17 : noms de fichiers et branches seulement.** Jamais le contenu d'un fichier, aucun identifiant de code lu sur le disque.

## F. Acceptation

| Cas | Attendu |
|---|---|
| « ouvre le fichier asr context » | `asr_context.rs` |
| « sur la branche fix lesson retries » | `fix/lesson-retries` |
| deux projets ouverts | celui de la fenêtre active |
| dossier de 200 000 fichiers | dictée sans retard, index partiel |
| `.env` dans le projet | jamais lu ni envoyé |
| terminal avec Codex | aucun terme de sa barre d'état |

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/contexte-dev`, créée depuis la branche du plan 9.

## H. Review Focus

1. Deux fenêtres VS Code au même nom de dossier : aucun projet plutôt que le mauvais. → tâche 1.
2. Lecteur réseau ou disque lent : index abandonné à 2 s. → tâche 2.
3. Nom de fichier contenant un secret ressemblant (`id_rsa`, `*.pem`, `.env*`) : exclu. → tâche 2.
4. Mot courant égal à un nom de fichier (« test », « index ») : pas forcé. → tâche 3.
5. Dépôt sans commit : branche absente, index quand même. → tâche 2.

## I. Tâches

### Tâche 1 : quel projet
**Files :** Create `src-tauri/src/project.rs`.
**Produit :** `pub fn vscode_root(title: &str, storage_json: &str) -> Option<PathBuf>` ; `pub fn terminal_root(tail: &str) -> Option<PathBuf>` (`PS C:\…>`, `C:\…>`, chemin Git Bash) ; `pub fn allowed_root(p: &Path, home: &Path) -> bool`.
- [ ] Tests sur des exemples fictifs : fenêtre simple, deux homonymes → `None`, `storage.json` abîmé → `None`, invite absente → `None`, dossier système refusé.
- [ ] Commit `feat(dev): which project is open`.

### Tâche 2 : index
**Produit :** `struct Index { files: Vec<String>, branches: Vec<String>, partial: bool }` ; `pub fn index(root: &Path, limits: Limits) -> Index` (`git ls-files -co --exclude-standard`, `git branch --format`, sans fenêtre, 2 s, 5 000 noms ; sinon parcours de profondeur 6).
- [ ] Tests dans un dossier temporaire : fichier suivi puis ignoré, jonction vers l'extérieur refusée, `.gitignore` respecté, secrets exclus, dépôt sans commit, dossier sans git, limites, dossier supprimé.
- [ ] Chaque index porte sa racine et l'identifiant de session ; un résultat d'une autre racine ou d'une session dépassée est jeté (test `a_late_index_of_another_project_is_dropped`). Refait si `.git/HEAD` ou `.git/index` change ; un seul en cours ; lancé par la session à l'appui, jamais attendu.
- [ ] Commit `feat(dev): file and branch names`.

### Tâche 3 : choix et envoi
**Files :** Modify `context/mod.rs`, `context/session.rs`, `asr_context.rs`, `adaptive.rs`, `actions.rs`.
**Produit :** `pub fn spoken_form(name: &str) -> String` (`asr_context.rs` → « asr context » ; `fix/lesson-retries` → « fix lesson retries ») ; `pub fn pick(ix: &Index, recent: &[String], open: Option<&str>, max: usize) -> Vec<String>` (ASR, 25) ; `pub fn heard(ix: &Index, transcription: &str) -> Vec<String>` (reformulation ; distance ≤ 0,34, deux mots ou plus, ou un mot absent du lexique).
- [ ] Tests : « ouvre asr context » → `asr_context.rs` ; « la branche fix lesson retries » ; « lance le test » sans forcer `test.rs` ; deux candidats proches → les deux proposés au modèle, rien de forcé sans lui.
- [ ] Un nom entendu (`heard` non vide) → `CleanGate.must_rewrite = true` (plan 8).
- [ ] Seulement dans les catégories IA et code et les terminaux ; jamais en champ privé ou exclu ; réglage `project_names` (défaut vrai).
- [ ] Commit `feat(dev): project names reach the recognizer and the rewrite`.

### Tâche 4 : bruit de VS Code et du terminal
**Files :** Modify `context/windows.rs`, `context/mod.rs` (`extract_terms`).
**Produit :** `pub fn terminal_agent(children: &[String]) -> Option<Agent>` (`claude`, `codex`) ; `pub fn strip_agent_ui(text: &str, agent: Agent) -> String`.
- [ ] Tests : barre d'état Codex et invite Claude Code (exemples inventés) retirées ; la dictée reste ; libellés isolés de VS Code (bouton, onglet) absents des termes.
- [ ] Commit `fix(context): editor and agent chrome are not vocabulary`.

### Tâche 5 : revue, build, installation
- [ ] Revue Terra, tests, build `fast`, sauvegarde, installation ; partie F avec toi dans deux projets.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/10-v2.md`. Bug déplacé au plan 0 : 4.6.
**À corriger dans ce plan**
- Tâche 4 mal fondée : sur ce PC, `claude.exe` et `codex.exe` sont enfants de `Code.exe`, pas d'un terminal (A5, C1).
- Le bruit du terminal vient de `extract_terms`, qui applique des règles de prose : corriger l'extraction (A9).
- Titre de VS Code : parfois un titre de conversation ajouté comme terme (A1) ; projet et fichier évincés en premier des 20 termes (A2).
- Prompt Whisper : 25 noms de fichiers évincent les mots appris ; fixer un budget (A3, C5).
- `git ls-files` sans `-z` : noms accentués échappés (A6) ; `storage.json` a `openedWindows` vide (A8, C2).
- Noms du projet en clair dans `history.db` (A11) ; index de 5 000 noms à revoir (C3) ; critères d'acceptation manquants (C6).
**Améliorations retenues**
- Comparer des clés collées (1 à 4 mots) plutôt que générer des formes parlées : plus simple (B1).
- Racine du projet depuis le répertoire courant de l'agent (B2, non vérifié).
- Voir ce qui a été utilisé (B5).
**Questions pour toi** : onglets ouverts de VS Code comme signal (B3, passe 1), contre la décision 5 ? Remplacement local de secours pour un nom exact (B4) ?
