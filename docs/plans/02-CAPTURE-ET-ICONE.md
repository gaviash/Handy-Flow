# Plan 2 : capture dès l'appui et icône (2026-11-24)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Idées : K5, A1, C1 (boutons), C3 à C5, C7, C8, W1 à W3, W40, V9. Dépend du plan 1. Durée : 2 j. Rien n'est codé.
**Goal :** tout lire pendant la parole et montrer l'application visée. **Architecture :** une session ouverte à l'appui, deux fils UIA, un contrôle rapide au relâchement. **Tech Stack :** Rust, crate `windows` 0.61, React.

## A. But

1. Le micro démarre sans rien attendre.
2. L'icône de l'application visée apparaît tout de suite dans la pastille.
3. Le contexte est lu pendant que tu parles. Au relâchement, plus d'attente (aujourd'hui 336 à 342 ms dans Gmail).

## B. Aujourd'hui (code lu)

- `TranscribeAction::stop` lance `context::capture`. L'appel Groq attend `private_rx`, envoyé **après** toute la lecture.
- Un seul fil UIA (`WORKER`) : 700 ms au total, 300 ms pour la page, 600 nœuds, boutons compris.
- La destination est le champ qui a le focus **au relâchement** (décision D3, gardée). Aucune garde de destination (ta décision du 15/11, gardée).
- Connexion Groq ouverte au relâchement, fermée après 30 s d'inactivité.

## C. Conception

```
appui        micro (inchangé, en premier) ─────────────────────────────►
             fenêtre + processus (Win32) → icône dans la pastille
             fil A (rapide) : mot de passe, site, exclusions, champ
             fil B (page)   : noms et termes, 3 s de calcul max, interruptible
             réseau         : connexion ouverte vers l'ASR et la reformulation
parole       toutes les 250 ms : fenêtre ou champ changé ? → lecture relancée, icône mise à jour
relâchement  fil A : champ actuel, mot de passe, curseur (250 ms max)
             → l'audio part dès ce contrôle, sans attendre la page
             même champ  → noms et termes déjà lus
             autre champ → anciens noms jetés, relecture comme aujourd'hui
```

| # | Décision (mienne, corrigeable) | Pourquoi |
|---|---|---|
| 1 | Deux fils UIA : contrôle rapide et lecture de page | une lecture de 3 s ne doit pas bloquer le contrôle du relâchement |
| 2 | Changement de focus par sondage à 250 ms, pendant la dictée seulement (V9) | plus simple qu'un abonnement, même effet visible |
| 3 | Rien de ce qui est lu n'est envoyé avant le contrôle du relâchement ; dictée annulée → tout est effacé | confidentialité |
| 4 | Contrôle du mot de passe impossible → ni contexte **ni dictée** envoyés à la reformulation (C8, W40). L'historique reste : c'est le seul moyen de récupérer le texte (décision du 15/11) | aujourd'hui, seul l'ASR est protégé |
| 5 | Boutons, menus, barres d'outils, onglets et zones de navigation ignorés, par type de contrôle et `LandmarkType` (aucun libellé traduit) | C1 |
| 6 | Champ lu à 1 500 caractères de chaque côté | C7 |
| 7 | Champ illisible → second essai à 300 ms, pendant la parole | C5 |
| 8 | Connexion gardée 90 s au lieu de 30 s | dictées longues |
| 9 | Icône : `WM_GETICON` (50 ms max), puis classe de fenêtre, puis fichier exe ; gardée par exe ; réglage pour la masquer, affichée par défaut | W3 |
| 10 | Polish et transformations par touche : lecture inchangée | pas de phase d'écoute |

La limite de 250 ms du relâchement est un maximum : sa durée réelle sera mesurée.

## D. Découpage du code

- `context/session.rs` (nouveau) : `Session::start`, `Session::finish() -> Finished` (privé ou non vérifié, contexte, identité du champ), `Session::cancel()`.
- `context/windows.rs` : fils A et B, filtre de page, 1 500 caractères, `watch_destination()` pour le plan 3.
- `context/icon.rs` (nouveau, Windows) : `icon_for(hwnd, exe) -> Option<AppIcon { width, height, rgba_base64 }>`, dessinée sur un `canvas`, sans dépendance ajoutée (`base64` est déjà là).
- `actions.rs` : `start` ouvre la session après `try_start_recording` ; `stop` appelle `finish`.
- `llm_client.rs` : `prewarm(provider)`, 90 s.
- `overlay.rs`, `RecordingOverlay.tsx` et `.css`, `AppearanceSettings.tsx`, `settings.rs`, traductions.

## E. Tes réponses (24/11)

- **Q1 : icône prise sur le site.** Téléchargée directement chez le site visité, jamais par un service tiers.
- **Q2 : à la place du point**, petit point coloré dans le coin pour le micro. La pastille garde sa taille.

Règles de Q1 (mes choix) :
1. Téléchargée **après** le contrôle du relâchement, seulement si le champ n'est ni privé, ni exclu, ni non vérifié. La première dictée sur un site montre donc l'icône du navigateur ; les suivantes, celle du site. Échec : icône du navigateur.
2. `https://<hôte>/favicon.ico` ; sinon le premier `<link rel="icon">` de la page d'accueil. Sans cookie ni identifiant ; même hôte ou sous-domaine ; 3 redirections, 3 s, 100 Ko, type `image/*`.
3. Jamais pour un domaine exclu, une adresse IP, `localhost`, un nom sans point, ni si la lecture du contexte est coupée.
4. Gardée 30 jours dans `Data/site-icons`, échec retenu 1 jour, 200 icônes au plus. Effacée avec un bouton dans la page Contexte.
5. Affichée par une balise `img` (un SVG n'y exécute aucun script).

## F. Acceptation

| Cas | Attendu |
|---|---|
| Gmail, dictée de 5 s | attente avant l'appel vocal mesurée et inférieure à 50 ms (objectif) ; aucun bouton dans les termes |
| Fenêtre changée pendant la parole | icône changée en moins de 500 ms ; contexte de la fenêtre d'arrivée seulement |
| Champ mot de passe, même atteint à la fin | rien d'envoyé, ni historique ni audio |
| Application figée | dictée normale, sans contexte |
| Dictée annulée | aucune donnée gardée |
| Démarrage du micro | pas plus lent qu'avant (`start-path pre-recording steps`) |

Le filtre des zones de navigation demande une sonde en lecture seule sur Gmail, page ouverte par toi.

## G. Règles communes

Mêmes règles communes que le plan 1 (partie D) : branche locale, jamais poussée sans accord ; journal sans texte dicté ; un seul build à la fin, profil `fast` ; aucun test qui prend le clavier, le focus ou le micro ; ligne `Claude-Session` en fin de commit.
Test Rust : `cd Handy/src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` ; frontend : `bun run build && bun run lint && bun run check:translations`. Attendu partout : 0 échec.
Chaque tâche suit : test qui échoue → code → tests → commit.
Branche `feat/capture-session`, créée depuis la branche du plan 1.

## H. Review Focus

1. Focus déplacé entre l'appui et le relâchement : jamais le contexte de l'ancienne fenêtre. → tâche 3.
2. Champ mot de passe atteint pendant la parole : lecture de page arrêtée, noms effacés. → tâche 3.
3. Relâchement avant la fin de la première lecture : pas d'attente de la page. → tâche 2.
4. Deux dictées rapprochées : la session de la première n'alimente pas la seconde. → tâche 3.
5. Site dont l'icône est énorme, en HTML ou redirigée ailleurs : refusée. → tâche 6.

## I. Tâches

### Tâche 1 : deux fils UIA et filtre de page
**Files :** Modify `src-tauri/src/context/windows.rs`, `context/mod.rs`.
**Produit :** `enum Lane { Quick, Page }` ; `fn run_on<T>(lane: Lane, timeout: Duration, f: impl FnOnce(&IUIAutomation) -> T + Send + 'static) -> Option<T>` ; `fn skipped_zone(control: i32, landmark: i32) -> bool` ; `FIELD_CHARS = 1_500`.
- [ ] Tests : `a_stuck_page_read_does_not_delay_the_quick_lane` (Page bloquée 1 500 ms, Quick répond en moins de 100 ms) ; `chrome_of_the_page_is_skipped` (Button, Menu, MenuBar, MenuItem, ToolBar, Tab, TabItem et landmark navigation → vrai ; Text, Hyperlink, ListItem sans landmark → faux).
- [ ] Le fil bloqué de chaque voie est remplacé séparément (`a_stuck_reader_is_replaced` gardé pour les deux).
- [ ] Commit `feat(context): a quick lane and a page lane for UI Automation`.

### Tâche 2 : la session
**Files :** Create `src-tauri/src/context/session.rs` ; Modify `context/mod.rs`, `context/windows.rs`.
**Consomme :** `run_on`, `Lane`. **Produit :**
```rust
pub struct Target { pub hwnd: isize, pub pid: u32, pub process: String }
pub trait Reader: Send + Sync + 'static {           // Windows en vrai, factice en test
    fn target(&self) -> Target;
    fn quick(&self, t: &Target) -> Option<Quick>;   // mot de passe, hôte, exclusions, champ, identité de l'élément
    fn page(&self, t: &Target, stop: &AtomicBool) -> Vec<String>;
}
pub struct Quick { pub capture: Capture, pub element: Vec<i32> } // element = UIA runtime id
pub struct Session;                                  // identifiant croissant
impl Session {
    pub fn start(reader: Arc<dyn Reader>, on_target: impl Fn(&Target) + Send + 'static) -> Self;
    pub fn finish(self) -> Finished;
    pub fn cancel(self);
}
pub struct Finished { pub id: u64, pub private: bool, pub capture: Capture, pub element: Option<Vec<i32>> }
```
`element` est l'identité du champ contrôlé au relâchement ; le plan 3 la reçoit jusqu'à `observe`.
- [ ] Tests avec lecteur factice : `release_does_not_wait_for_the_page` (page 3 s, `finish` < 300 ms, termes vides) ; `same_field_reuses_the_page_terms` ; `another_field_at_release_drops_the_old_terms` ; `an_unreadable_field_is_read_again_after_300_ms` ; `an_unchecked_password_is_private`.
- [ ] `quick` limité à 250 ms ; `page` à 3 s et au drapeau `stop`, levé par `finish`, `cancel` ou un changement de cible.
- [ ] Commit `feat(context): a capture session from press to release`.

### Tâche 3 : branchement sur la dictée
**Files :** Modify `actions.rs` (`start`, `stop:1018-1067`, annulation), `context/mod.rs:431` (`llm_context`), `transcription_coordinator.rs` si l'annulation y passe.
**Consomme :** `Session`.
- [ ] Tests : `llm_context_needs_a_checked_password` (Windows : `password_checked = false` → `None`) ; `an_old_session_is_ignored` ; `a_password_reached_while_speaking_clears_the_names` ; `an_unchecked_field_is_never_rewritten` (pas d'appel LLM, texte brut collé, historique gardé).
- [ ] `start` ouvre la session **après** `try_start_recording`. `private_tx` part dès le retour de `finish`. Échap, échec du micro et changement de session appellent `cancel`. Sans session (Linux, macOS, contexte coupé) : chemin actuel.
- [ ] Journal : `release check {ms}, page {prête|tardive|jetée}`.
- [ ] Commit `feat(dictation): context is read while speaking`.

### Tâche 4 : connexion ouverte à l'appui
**Files :** Modify `llm_client.rs`, `actions.rs`.
**Produit :** `pub fn prewarm(base_url: &str)` (requête `HEAD` sans clé, erreurs ignorées, une fois par hôte et par dictée) ; `pool_idle_timeout` 90 s.
- [ ] Test `prewarm_reuses_the_pool` : serveur local qui compte les connexions, `prewarm` puis une requête → 1 connexion.
- [ ] Commit `perf(net): the connection opens at key press`.

### Tâche 5 : icône de l'application
**Files :** Create `context/icon.rs` ; Modify `overlay.rs`, `actions.rs`, `settings.rs` (`overlay_app_icon: bool`, défaut vrai), `RecordingOverlay.tsx`, `RecordingOverlay.css`, `AppearanceSettings.tsx`, `bindings.ts`, traductions, `Cargo.toml` (`Win32_UI_Shell`).
**Produit :** `pub struct AppIcon { pub width: u32, pub height: u32, pub rgba_base64: String }` ; `pub fn bgra_to_rgba(p: &mut [u8])` ; événement `dictation-target` `{ name, icon: AppIcon | null, image: string | null }` vers `recording_overlay`.
- [ ] Tests Rust : `bgra_becomes_rgba` ; `an_icon_without_alpha_is_opaque`. Playwright (simulé) : l'icône remplace le point, 18 × 18 px, `aria-hidden` ; sans icône, le point reste ; réglage coupé → point.
- [ ] Le nom accessible de la pastille devient « Enregistrement dans <application> ».
- [ ] Commit `feat(overlay): the target app's icon`.

### Tâche 6 : icône du site
**Files :** Create `src-tauri/src/site_icon.rs` ; Modify `actions.rs`, `lib.rs`, `ContextPage.tsx`, `bindings.ts`, traductions.
**Produit :** `pub fn allowed_host(host: &str, excluded: &[String]) -> bool` ; `pub fn icon_link(html: &str, host: &str) -> Option<String>` ; `pub async fn site_icon(host: &str, dir: &Path) -> Option<String>` (URL `data:`) ; commande `clear_site_icons`.
- [ ] Tests : `only_public_https_hosts` (refuse `localhost`, `10.0.0.1`, `intranet`, domaine exclu ; accepte `mail.google.com`) ; `icon_link_stays_on_the_site` ; serveur local : 101 Ko refusé, `text/html` refusé, échec mis en attente 1 jour, deuxième appel sans réseau ; aucune requête pour une dictée annulée ou privée.
- [ ] Commit `feat(overlay): the site's own icon`.

### Tâche 7 : revue, build, acceptation
- [ ] Revue Terra, corrections, tests.
- [ ] Build `fast`, sauvegarde, installation (étapes 5.2, 5.3 et 5.5 du plan 1, sans réparation d'historique).
- [ ] Avec toi : sonde Gmail en lecture seule (termes sans bouton), puis les cas de la partie F.


## J. Mise à jour du 26/11 (Willow, Wispr — passe 2)
Détail : `willow/02-v2.md`. Bugs du code actuel déplacés au plan 0 : 2.2, 2.3.
**À corriger dans ce plan**
- Voie rapide « tout ou rien » : le verdict mot de passe est rendu seul et d'abord, indépendant des lectures lentes (A2, C1).
- Décision 4 (non vérifié = privé) : ajouter `excluded` et une notice (C2).
- Q1 icône du site « même hôte » : refuserait les icônes Gmail/Docs/Slack (CDN) ; accepter l'icône déclarée par la page (C3).
- Voie de lecture dédiée au suivi (`Lane::Watch`) : un délai UIA remplace le worker et perd l'élément observé (A5 ; plan 3).
- Lecture du champ : 500 caractères après suffisent (Wispr 500/500), pas 1 500 (C6).
- Pastille : garder un signal net « micro pas prêt » (C7).
**Améliorations retenues**
- Page lue pendant la parole : un résultat tardif n'est pas jeté, la reformulation l'utilise (B1).
- Une seule lecture en vol, arrêt après 3 échecs (B3) ; Chrome froid : relancer la page (B4).
- Message quand la reformulation est sautée ou qu'il n'y a pas de champ (A4).
- Ne rien lire ni apprendre quand le champ est dans Handy (passe 1).
- Icône des applications empaquetées (MSIX) (B5).
**Non vérifié** : focus hors processus (WebView2/Outlook) (A7), connexion tiède (A8).

## K. Mise à jour du 27/11 (logs du 26–27/11)
**Filtre des termes lus à l'écran (validé).** Les termes envoyés à Whisper et au LLM sont surtout du bruit : libellés d'interface (« Outils », « Moyen », « Idées », « Réinitialisation », « Connecté », « Hors », « Click »), jours et mois (« Thu », « Nov »), fuseaux (« Europe/Paris »), identifiants opaques (`1sb1cJS…`, `req_011…`, `UCqPVd…`), titres coupés par « … », libellés de sélecteur de modèle (« GPT-6.1 Terra Élevé »). Ils viennent de textes et de liens, pas de boutons : la décision 5 (type de contrôle) ne suffit pas.
- Filtre dans `extract_terms` (`context/mod.rs:615`), **avant** comptage et classement : mot courant isolé (`lexicon::is_common`, jamais appelé ici), jours et mois abrégés, fuseaux horaires, texte qui finit par « … », identifiant opaque (longue suite de lettres et chiffres mêlés, sans voyelles régulières).
- Un test par exemple réel ci-dessus ; un vrai nom propre (« Cartoon Shorts », « Hyperframe ») doit passer.
- Durée : + 2 h.
