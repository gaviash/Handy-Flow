# Handy Flow — Branding, thèmes et polish : plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. For every UI task, also load the `impeccable` skill and read its `reference/craft-floor.md` before editing.

**Goal:** Renommer Handy en « Handy Flow » (nom, processus, icône, fenêtres) et livrer 8 palettes × clair/sombre sélectionnables dans un nouvel écran Apparence, avec le polish et les animations du pilote.

**Architecture:** Les palettes vivent dans un seul fichier CSS (`palettes.css`) qui utilise `light-dark()`. Le mode existant (`data-theme`) ne fait que changer `color-scheme`. Les tokens actuels de `theme.css` deviennent des alias des tokens de palette, ce qui laisse les composants inchangés. Trois nouveaux réglages Rust (`palette`, `heading_style`, `ui_density`, plus `overlay_show_label`) sont émis vers l'overlay comme `theme` l'est déjà. Le renommage passe par la config Tauri et par quelques chaînes Rust ; les chemins de données ne bougent pas.

**Tech Stack:** Tauri 2, Rust, React 18 + Tailwind 4, i18next, scripts de test bun/assert, Playwright (Chrome installé, IPC Tauri simulé).

**Spec:** `docs/superpowers/specs/2026-11-16-handy-flow-brand-design.md` · référence visuelle : `docs/branding/pilote-handy-flow-v2.html`

## Global Constraints

- Branche : `feat/handy-flow-brand`, créée depuis `feat/adaptive-learning-context`. Ne jamais pousser vers `upstream`.
- `identifier` reste `com.pais.handy` ; le marqueur portable `Handy Portable Mode` et le dossier `Data\` restent inchangés.
- Palette par défaut `rose` ; valeur inconnue → `rose`. Mode par défaut `system`.
- Logique Rust pure : `cargo test --release --lib`, lancé dans `src-tauri`.
- Builds d'essai : `bun run tauri build --no-bundle -- --profile fast`. Un seul build par incrément (3 au total) ; ne jamais modifier les sources pendant un build.
- Après chaque build : remplacer `C:\Users\gavri\Desktop\Handy Flow\Handy Flow.exe`, les DLL de `src-tauri/transcribe-libs` et `resources\` ; sauvegarder puis **ne jamais toucher `Data\`**. Ne jamais modifier `Desktop\Handy`.
- Aucun test qui prend le clavier, le focus ou l'audio du PC. Playwright tourne en headless ; les dictées réelles sont confiées à l'utilisateur.
- Tout texte visible passe par i18next (ESLint l'impose). Les nouvelles clés vont dans `en` et `fr`, avec la valeur anglaise dans les 24 autres locales, pour que `bun run check:translations` passe.
- `bindings.ts` se régénère en lançant l'app en debug (`bun run tauri dev`) après tout changement de commande ou de type Rust.
- Mouvement : `cubic-bezier(.22,1,.36,1)`, 150 à 450 ms, et rien sous `prefers-reduced-motion: reduce`.
- Avant chaque commit : `bun run lint`, `bun run format:check`, `cargo clippy`.

## Review Focus

1. **Store existant sans les nouvelles clés** (le `Data\settings_store.json` de l'utilisateur) : il se charge avec les valeurs par défaut sans perdre aucun réglage. Testé dans la Task 5.
2. **Autostart de Handy d'origine** (`Desktop\Handy`, valeur Run `Handy`) : il ne doit jamais être supprimé par le nettoyage. Seule une valeur `Handy` qui pointe vers *notre* exe l'est. Testé dans la Task 3.
3. **Mise à jour automatique** : elle ne doit jamais proposer l'installeur de cjpais/Handy par-dessus Handy Flow. Testé dans la Task 1.
4. **Overlay ouvert pendant un changement de palette ou de libellé** : il se recolore sans redémarrage. Testé dans la Task 12.
5. **`localStorage` indisponible ou valeur corrompue au démarrage** : l'app applique les valeurs par défaut sans planter. Testé dans la Task 7.

---

### Task 0: Préparer la branche

**Files:**
- Create: `src/assets/fonts/Figtree-Variable.woff2`, `InstrumentSerif-Regular.woff2`, `InstrumentSerif-Italic.woff2`, `src/assets/fonts/OFL.txt`

- [ ] **Step 1: Faire mettre de côté le travail en cours sans rapport avec le branding** (`cloud_asr.rs`, plus des modifications dans `Cargo.toml`, `lib.rs`, `llm_client.rs`, `transcription.rs` et `settings.rs`). Demander à l'utilisateur : commit sur sa propre branche, ou `git stash push -u -m cloud-asr-wip`. Ne rien commiter de ce travail sur `feat/handy-flow-brand`.
- [ ] **Step 2: Télécharger les woff2** depuis les releases GitHub officielles des fontes (Figtree : erikdkennedy/figtree ; Instrument Serif : Instrument/instrument-serif). Ajouter le texte de la licence OFL dans `OFL.txt`. Vérifier : `file *.woff2` → « Web Open Font Format (Version 2) ».
- [ ] **Step 3: Commit** `chore: bundle Figtree and Instrument Serif (OFL)`

---

## Incrément 1 : nom, icône, intégration système

### Task 1: Renommer l'app et désactiver l'updater amont

**Files:**
- Modify: `src-tauri/tauri.conf.json` (`productName`, `mainBinaryName`, `plugins.updater.endpoints` → `[]`, `bundle.windows.signCommand` → supprimé)
- Modify: `src-tauri/src/lib.rs:1051` (titre de la fenêtre principale), `src-tauri/src/overlay.rs:442,496`, `src-tauri/src/tray.rs:613-622`, `src-tauri/src/cli.rs:5`, `src-tauri/src/engine_supervisor/worker.rs:477`, `src-tauri/src/engine_supervisor/supervisor.rs:1437`, `src-tauri/src/settings.rs:1723` (`update_checks_forced_disabled`)
- Modify: `index.html`, `src/overlay/index.html` (`<title>`)
- Create: `src-tauri/src/branding.rs` (constantes + tests), déclaré dans `lib.rs`

**Interfaces:**
- Produces: `pub const APP_NAME: &str = "Handy Flow";` et `pub const OVERLAY_TITLE: &str = "Handy Flow Overlay";` dans `branding.rs`, utilisés par lib.rs, overlay.rs et tray.rs.

- [ ] **Step 1: Write the failing tests** dans `branding.rs` `#[cfg(test)]` :

```rust
const CONF: &str = include_str!("../tauri.conf.json");
#[test] fn product_name_is_handy_flow() { let v: serde_json::Value = serde_json::from_str(CONF).unwrap(); assert_eq!(v["productName"], "Handy Flow"); assert_eq!(v["mainBinaryName"], "Handy Flow"); }
#[test] fn identifier_never_changes() { /* v["identifier"] == "com.pais.handy" */ }
#[test] fn upstream_updater_is_unreachable() { /* endpoints is an empty array, and no string in CONF contains "cjpais/Handy/releases" */ }
#[test] fn updater_is_forced_off() { assert!(crate::settings::update_checks_forced_disabled()); }
#[test] fn tray_tooltip_names_handy_flow() { assert!(crate::tray::tray_tooltip().starts_with("Handy Flow v")); }
#[test] fn worker_thread_name_fits_prctl() { assert!("HFlow-worker".len() <= 15); }
```

- [ ] **Step 2: Run** `cargo test --release --lib branding` → FAIL (`productName` vaut "Handy").
- [ ] **Step 3: Implement.** Ajouter `|| true` dans `update_checks_forced_disabled`, avec le commentaire `// Fork: no release channel; never fetch cjpais/Handy over Handy Flow.`. Le titre de fenêtre, l'infobulle et le titre de l'overlay lisent `branding::APP_NAME` et `branding::OVERLAY_TITLE`. Mettre `about = "Handy Flow - Speech to Text"` et `name = "handy-flow"` dans la CLI. Remplacer « restart Handy » par « restart Handy Flow » dans supervisor.rs. Mettre `<title>Handy Flow</title>` dans les deux fichiers HTML.
- [ ] **Step 4: Run** `cargo test --release --lib` → tout PASS. Mettre à jour les tests existants qui attendaient « Handy v ».
- [ ] **Step 5: Vérifier le nom du binaire.** Lancer `bun run tauri build --no-bundle -- --profile fast` **une seule fois, à la fin de la Task 4** (ce build est celui de l'incrément) et noter le nom produit dans `target/fast/`. Si c'est encore `handy.exe`, le déploiement le renomme en `Handy Flow.exe` ; la description de fichier et le nom de produit de l'exe doivent lire « Handy Flow » (`(Get-Item exe).VersionInfo`).
- [ ] **Step 6: Commit** `feat(brand): name the app Handy Flow; never update from upstream Handy`

### Task 2: Renommer dans les traductions + garde-fou

**Files:**
- Create: `scripts/check-branding.ts`
- Modify: `package.json` (script `"check:branding": "bun scripts/check-branding.ts"`), les 26 `src/i18n/locales/*/translation.json`

- [ ] **Step 1: Write the check.** Pour chaque valeur de chaque locale, `/\bHandy\b(?! Flow)/` doit être faux, sauf pour les clés listées dans `ALLOW = ["about.forkOf"]` (crédit au projet d'origine, ajouté dans la Task 13). Le script vérifie aussi que `index.html` et `src/overlay/index.html` contiennent `<title>Handy Flow</title>`. Il sort en code 1 en listant `locale:clé`.
- [ ] **Step 2: Run** `bun run check:branding` → FAIL avec environ 38 occurrences par locale.
- [ ] **Step 3: Remplacer** « Handy » par « Handy Flow » dans les **valeurs** (jamais dans les clés), avec un script ponctuel : remplacer `\bHandy\b(?! Flow)`. Relire ensuite le diff `fr` et `en` à la main pour les accords (« Handy Flow est… »).
- [ ] **Step 4: Run** `bun run check:branding && bun run check:translations` → PASS.
- [ ] **Step 5: Commit** `feat(brand): Handy Flow in every locale, with a check that keeps it`

### Task 3: Autostart Windows sans doublon ni casse

Le plugin d'autostart range la valeur Run sous le nom du produit. Après le renommage, l'ancienne valeur `Handy` qui pointe vers notre exe lancerait l'app deux fois.

**Files:**
- Modify: `src-tauri/src/autostart.rs` (nouveau module `#[cfg(windows)] mod legacy`)

**Interfaces:**
- Produces: `fn legacy_value_points_to(value: &str, current_exe: &Path) -> bool` (pure) et `pub fn remove_legacy_run_value(current_exe: &Path)`, appelée au début de `apply_autostart` sous Windows.

- [ ] **Step 1: Write the failing tests:**

```rust
#[test] fn quoted_path_with_args_matches() { assert!(legacy_value_points_to(r#""C:\Users\u\Desktop\Handy Flow\Handy Flow.exe" --start-hidden"#, Path::new(r"C:\Users\u\Desktop\Handy Flow\Handy Flow.exe"))); }
#[test] fn case_insensitive_match() { /* "c:\\users\\…\\handy flow.exe" matches */ }
#[test] fn original_handy_install_is_left_alone() { assert!(!legacy_value_points_to(r#""C:\Users\u\Desktop\Handy\Handy.exe""#, Path::new(r"C:\Users\u\Desktop\Handy Flow\Handy Flow.exe"))); }
#[test] fn pre_rename_exe_in_same_folder_matches() { /* "…\Handy Flow\handy.exe" matches when current is "…\Handy Flow\Handy Flow.exe": same folder, file stem "handy" */ }
#[test] fn empty_value_is_ignored() { assert!(!legacy_value_points_to("", Path::new(r"C:\a\b.exe"))); }
```

- [ ] **Step 2: Run** `cargo test --release --lib autostart` → FAIL (fonction absente).
- [ ] **Step 3: Implement** avec `winreg` (déjà en dépendance) : ouvrir `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, lire la valeur `Handy` et la supprimer seulement si `legacy_value_points_to` est vrai. Une erreur de registre est journalisée en `warn`, jamais propagée.
- [ ] **Step 4: Run** `cargo test --release --lib` → PASS.
- [ ] **Step 5: Commit** `fix(autostart): drop our own pre-rename Run entry, never the original Handy's`

### Task 4: Icônes Handy Flow

**Files:**
- Create: `src-tauri/icons/source/handy-flow.svg` (tuile, main, point ; couleurs via `{{bg}} {{fg}} {{dot}}`)
- Create: `scripts/render-icons.ts` (Playwright + Chrome installé : rend le SVG à 1024 px pour chaque palette en clair → `src-tauri/icons/source/<palette>.png`, et à 256 px → `src-tauri/resources/icons/palette-<palette>.png`)
- Modify: `src-tauri/icons/*` (régénérés par `bunx tauri icon src-tauri/icons/source/rose.png`), `src-tauri/resources/handy.png` (= palette-rose 256), `src-tauri/tauri.conf.json` (`bundle.resources` inclut `resources/icons/*`)

**Interfaces:**
- Produces: les fichiers `src-tauri/resources/icons/palette-<id>.png`. La fonction `palette_icon_resource(p: Palette) -> &'static str` (`branding.rs`, renvoie `"resources/icons/palette-<id>.png"`) est écrite dans la Task 5, en même temps que `Palette`.

- [ ] **Step 1: Écrire le SVG.** Reprendre le `d` de `HandyHand.tsx`, `rotate(-8)`, la main à 62 % de la tuile, le rayon de tuile à 23 %, le point à 16 % (haut droite, à 15 % des bords). Couleurs Rose : bg `#f28cbb`, fg `#3b1d2c`, dot `#1c1915`.
- [ ] **Step 2: Run** `bun scripts/render-icons.ts` → 8 PNG de 1024 px et 8 de 256 px. Vérifier visuellement les 8 à côté de la section « L'icône suit le thème » du pilote.
- [ ] **Step 3: Run** `bunx tauri icon src-tauri/icons/source/rose.png` → `.ico`, `.icns` et toutes les tailles sont régénérés. Vérifier l'`icon.ico` en 16 et 32 px : la main doit rester lisible.
- [ ] **Step 4: Build incrément 1** (`bun run tauri build --no-bundle -- --profile fast`), puis déployer dans `Desktop\Handy Flow` selon les Global Constraints.
- [ ] **Step 5: Vérifier** sans prendre la main sur le PC : `(Get-Item "...\Handy Flow.exe").VersionInfo.ProductName` → `Handy Flow` ; `Get-Process "Handy Flow"` existe ; `reg query HKCU\...\Run` ne contient plus de `Handy` qui pointe vers ce dossier. Confier à l'utilisateur le contrôle visuel : nom dans le Gestionnaire des tâches, icône de la barre des tâches, infobulle du tray, historique et modèles toujours présents.
- [ ] **Step 6: Commit** `feat(brand): Handy Flow icon set, one per palette`

---

## Incrément 2 : système de thèmes

### Task 5: Réglages d'apparence (Rust)

**Files:**
- Modify: `src-tauri/src/settings.rs` (enums, champs, defaults, tests), `src-tauri/src/shortcut/mod.rs` (commandes, sur le modèle de `change_theme_setting` l. 652), `src-tauri/src/lib.rs` (enregistrer les commandes), `src/bindings.ts` (régénéré)

**Interfaces:**
- Produces (Rust, serde `snake_case`) :
  - `enum Palette { Rose, Sauge, Prune, Encre, Ocean, Ambre, Lime, Contraste }`, avec `Palette::ALL: [Palette; 8]`, `fn id(self) -> &'static str` et `fn from_setting(&str) -> Palette` (inconnu → Rose)
  - `enum HeadingStyle { Editorial, Modern }`, `enum UiDensity { Comfortable, Compact }`
  - Champs d'`AppSettings` : `palette`, `heading_style`, `ui_density`, `overlay_show_label: bool` (défaut `true`), tous avec `#[serde(default = …)]`
  - Commandes : `change_palette_setting(palette: String)`, `change_heading_style_setting(style: String)`, `change_ui_density_setting(density: String)`, `change_overlay_show_label_setting(enabled: bool)`. Chacune écrit le réglage puis émet `"appearance-changed"` avec l'objet `Appearance { theme, palette, heading_style, ui_density, overlay_show_label }`. `change_theme_setting` émet aussi `appearance-changed` (en plus de `theme-changed`, gardé pour compatibilité).
  - Commande `get_palette_icon_path` : inutile, l'icône est posée côté Rust (Task 14).

- [ ] **Step 1: Write the failing tests** (module de tests de `settings.rs`, même style que les tests `sound_theme` existants, l. 1926-2042) :

```rust
#[test] fn legacy_store_without_appearance_keys_gets_defaults() { /* the existing legacy-store fixture → palette Rose, Editorial, Comfortable, overlay_show_label true, and sound_theme etc. unchanged */ }
#[test] fn unknown_palette_is_salvaged_to_rose() { /* map.insert("palette", json!("neon")) → salvage → Rose, other fields kept */ }
#[test] fn palette_from_setting_round_trips() { for p in Palette::ALL { assert_eq!(Palette::from_setting(p.id()), p); } assert_eq!(Palette::from_setting("x"), Palette::Rose); }
#[test] fn every_palette_has_a_css_block_and_an_icon() {
    let css = include_str!("../../src/styles/palettes.css");
    for p in Palette::ALL { assert!(css.contains(&format!("[data-palette=\"{}\"]", p.id()))); assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(crate::branding::palette_icon_resource(p)).exists()); }
    assert_eq!(css.matches("[data-palette=\"").count(), Palette::ALL.len());
}
```

- [ ] **Step 2: Run** `cargo test --release --lib settings` → FAIL. Le dernier test ne compilera qu'après la Task 6 ; le marquer `#[ignore]` jusque-là, puis retirer l'`ignore` à la fin de la Task 6.
- [ ] **Step 3: Implement** les enums, les champs, les defaults, les commandes et l'émission d'événement, puis régénérer `bindings.ts` avec `bun run tauri dev`.
- [ ] **Step 4: Run** `cargo test --release --lib` → PASS.
- [ ] **Step 5: Commit** `feat(appearance): palette, heading style, density and overlay label settings`

### Task 6: Palettes CSS et refonte de `theme.css`

**Files:**
- Create: `src/styles/palettes.css` (8 blocs × 13 tokens, valeurs de la spec §2, plus le bloc des dérivés `[data-palette]` copié du pilote)
- Modify: `src/styles/theme.css` (les paires `--light-*`/`--dark-*` de couleur d'interface deviennent des alias), `src/App.css` (importer `palettes.css` avant `theme.css` ; enregistrer les nouveaux tokens dans `@theme inline`)
- Create: `scripts/check-themes.ts` ; Modify: `package.json` (`"check:themes"`)

**Interfaces:**
- Produces les tokens CSS `--bg --ink --accent --on-accent --accent-text --side --side-ink --pill --pill-ink --pill-accent --icon-bg --icon-fg --icon-dot` et les dérivés `--surface --raised --ink-2 --ink-3 --line --line-strong --accent-wash --side-ink-2 --side-active --shadow-c`.
- Alias à garder pour ne pas toucher les composants : `--color-text: var(--ink)`, `--color-background: var(--bg)`, `--color-logo-primary: var(--accent)`, `--color-background-ui: var(--accent)`, `--color-ink-2: var(--ink-2)`, `--color-ink-3: var(--ink-3)`, `--color-line: var(--line)`, `--color-line-strong: var(--line-strong)`, `--color-wash: var(--surface)`, `--color-plum: var(--accent-text)`. `warning`, `error` et `moss` (avec leurs `-wash`) passent en `light-dark()` et gardent leurs valeurs actuelles. Les tokens `demo-*`, `mascot` et `wordmark` de l'onboarding restent, recâblés sur `--accent` et `--ink`.

- [ ] **Step 1: Write the check** `scripts/check-themes.ts`. Il parse chaque bloc `[data-palette="x"]`, résout `var(--ink)` et `var(--bg)` dans le même bloc, puis sépare `light-dark(a, b)`. Il vérifie : les 13 tokens présents ; les seuils de contraste de la spec §2 dans les deux modes (`ink-2` est calculé par mélange oklab de `ink` dans `bg`, à 72 % ou au `--ink-2-k` du bloc, puis comparé à `raised`) ; et l'égalité entre la liste des ids et `PALETTE_IDS` de `src/lib/themes.ts` (Task 7). Sortie : tableau des échecs, code 1.
- [ ] **Step 2: Run** `bun run check:themes` → FAIL (le fichier n'existe pas).
- [ ] **Step 3: Écrire `palettes.css`** et refaire `theme.css` : supprimer les blocs de couleur `@media (prefers-color-scheme: dark)` et `:root[data-theme]`, devenus inutiles puisque `color-scheme` suffit. Poser `data-palette="rose"` par défaut sur `<html>` dans les deux `index.html`.
- [ ] **Step 4: Run** `bun run check:themes` → PASS. Si une palette échoue, ajuster sa valeur dans la spec **et** dans le CSS, puis le noter dans le commit.
- [ ] **Step 5: Retirer l'`#[ignore]`** de la Task 5 et lancer `cargo test --release --lib` → PASS.
- [ ] **Step 6: Commit** `feat(themes): eight palettes as light-dark tokens; old tokens become aliases`

### Task 7: Appliquer l'apparence (frontend + overlay)

**Files:**
- Create: `src/lib/themes.ts` ; Modify: `src/lib/utils/theme.ts` (renommé `appearance.ts`, en mettant à jour tous les imports), `src/overlay/main.tsx`, `src/main.tsx` (application au démarrage)
- Test: `src/lib/utils/appearance.test.ts` (bun assert, selon la convention de `portableInstaller.test.ts`)

**Interfaces:**
- Produces :
  - `src/lib/themes.ts` : `export const PALETTE_IDS = ["rose","sauge","prune","encre","ocean","ambre","lime","contraste"] as const; export type PaletteId = typeof PALETTE_IDS[number];`
  - `export type Appearance = { theme: Theme; palette: PaletteId; heading_style: "editorial"|"modern"; ui_density: "comfortable"|"compact"; overlay_show_label: boolean }`
  - `applyAppearance(a: Appearance, root?: HTMLElement): void` pose `data-theme` (absent si `system`), `data-palette`, `data-headings`, `data-density` et `data-overlay-label`, puis écrit `localStorage["handy.appearance"]`
  - `getStoredAppearance(): Appearance` : valeurs par défaut si la valeur est absente, invalide ou si le stockage lève une erreur ; migre l'ancienne clé `handy.theme`
  - `withThemeTransition(fn: () => void): void` : `document.startViewTransition` si disponible et sans réduction du mouvement, sinon `fn()`
  - L'overlay et la fenêtre principale écoutent tous deux `"appearance-changed"`.

- [ ] **Step 1: Write the failing tests:**

```ts
// applies every attribute; system mode removes data-theme
applyAppearance({theme:"system",palette:"sauge",heading_style:"modern",ui_density:"compact",overlay_show_label:false}, root);
assert.equal(root.dataset.palette,"sauge"); assert.equal(root.dataset.theme, undefined); assert.equal(root.dataset.headings,"modern"); assert.equal(root.dataset.density,"compact"); assert.equal(root.dataset.overlayLabel,"false");
// corrupt storage → defaults
storage.setItem("handy.appearance","{oops"); assert.deepEqual(getStoredAppearance(), DEFAULT_APPEARANCE);
// unknown palette → rose, other fields kept
// storage that throws on access → defaults, no throw
// legacy key handy.theme="dark" → theme dark, palette rose
```

Utiliser un faux `root` (un objet avec `dataset`) et un faux `localStorage` injecté par `globalThis`.

- [ ] **Step 2: Run** `bun src/lib/utils/appearance.test.ts` → FAIL.
- [ ] **Step 3: Implement**, puis brancher `main.tsx` et `overlay/main.tsx` (appliquer la valeur stockée avant le rendu, puis se synchroniser depuis `getAppSettings`).
- [ ] **Step 4: Run** le test → PASS ; `bun run build` → OK.
- [ ] **Step 5: Commit** `feat(appearance): apply palette, mode, headings and density everywhere`

### Task 8: Harnais d'intégration Playwright (IPC Tauri simulé)

**Files:**
- Modify: `playwright.config.ts` (`use.channel: "chrome"`, `reporter: "list"`, `expect.toHaveScreenshot.maxDiffPixelRatio: 0.01`)
- Create: `tests/fixtures/tauri-mock.ts`, `tests/fixtures/app-settings.json`, `tests/branding.spec.ts`
- Modify: `src-tauri/src/settings.rs` (test de fixture)

**Interfaces:**
- Produces : `export async function mockTauri(page, opts?: { settings?: Partial<AppSettings>; handlers?: Record<string, (args) => unknown> }): Promise<{ calls: Array<{cmd: string; args: unknown}>; emit(event: string, payload: unknown): Promise<void> }>`. Un `addInitScript` y installe `window.__TAURI_INTERNALS__` (`invoke`, `transformCallback`, `metadata.currentWindow/currentWebview.label = "main"`) et l'IPC des événements du plugin `event`. Une commande inconnue renvoie `null` et va dans `calls`. Par défaut, les commandes de modèle déclarent un modèle téléchargé et actif, pour sauter l'onboarding.

- [ ] **Step 1: Fixture des réglages.** Dans `settings.rs`, ajouter le test `default_settings_fixture_is_current`. Il compare `serde_json::to_value(AppSettings::default())` à `../tests/fixtures/app-settings.json` et réécrit le fichier si `UPDATE_FIXTURES=1`. Lancer une fois avec la variable pour créer le fichier, puis sans → PASS.
- [ ] **Step 2: Write `branding.spec.ts`:**

```ts
test("window title and wordmark say Handy Flow", ...) // expect(page).toHaveTitle("Handy Flow"); sidebar contains text "Handy" + "Flow"
test("no visible text says Handy alone", ...) // visit each sidebar section; for every text node: !/\bHandy\b(?! ?Flow)/
```

- [ ] **Step 3: Run** `bunx playwright test tests/branding.spec.ts` → le premier test PASS (titre de la Task 1). Le second peut échouer sur le wordmark tant que la Task 9 n'est pas faite ; c'est attendu, le noter.
- [ ] **Step 4: Commit** `test: Playwright harness with a mocked Tauri IPC`

### Task 9: Polices, wordmark, barre latérale

**Files:**
- Modify: `src/App.css` (`@font-face` × 3 ; `--font-ui: Figtree, …` ; `--font-display: var(--display)`, où `--display` vaut Instrument Serif, ou Figtree sous `[data-headings="modern"]`, comme dans le pilote)
- Create: `src/components/icons/HandyFlowWordmark.tsx` (`<span class="wm"><b>Handy</b><i>Flow</i></span>`, `aria-label="Handy Flow"`, taille en prop)
- Modify: `src/components/Sidebar.tsx` (wordmark, section `appearance` avec l'icône lucide `Brush` après `postProcessing`, séparateur, carte d'état du modèle en bas), `src/components/settings/index.ts` et le routeur de sections (`AppearanceSettings`), `src/i18n/locales/*/translation.json` (`sidebar.appearance`)
- Modify: `src/components/onboarding/screens/Welcome.tsx`, `FileDropOverlay.tsx` (remplacer `HandyTextLogo` par le wordmark) ; supprimer `HandyTextLogo.tsx` s'il n'a plus d'usage

- [ ] **Step 1: Étendre `branding.spec.ts`** : la barre latérale montre 8 entrées dans l'ordre Général, Historique, Modèles, Post-traitement, Apparence, Avancé, (Débogage si activé), À propos ; l'état « Prêt à dicter » est visible.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement**, selon la fenêtre du pilote : barre latérale de 212 px sur `--side`, élément actif sur `--side-active`, hover en 150 ms.
- [ ] **Step 4: Run** `bunx playwright test tests/branding.spec.ts` → PASS, puis `bun run check:translations` → PASS.
- [ ] **Step 5: Commit** `feat(ui): Handy Flow wordmark, bundled fonts, sidebar with Appearance`

### Task 10: Primitives d'interface au niveau du pilote

**Files:**
- Modify: `src/components/ui/ToggleSwitch.tsx`, `SettingsGroup.tsx`, `SettingContainer.tsx`, `Dropdown.tsx`, `Select.tsx`, `Button.tsx`, `Badge.tsx`, `Radio.tsx`
- Create: `src/components/ui/Segmented.tsx` (`{ options: {value, label}[]; value; onChange; ariaLabel }`, rôle `radiogroup`, flèches gauche/droite)
- Modify: `src/App.css` (sélection, `:focus-visible` 2 px `accent-text`, barres de défilement sur `--line-strong`, `@media (prefers-reduced-motion)` global)

- [ ] **Step 1: Ajouter `tests/ui.spec.ts`** : un interrupteur coché a pour fond `--accent` (couleur calculée = l'accent de la palette active) ; `Segmented` se pilote au clavier (Tab, puis flèche droite → `aria-checked` se déplace) ; avec `emulateMedia({ reducedMotion: "reduce" })`, la `transition-duration` calculée du bouton d'interrupteur vaut `0s`.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement**, valeurs du pilote : groupes de rangées sur `--raised` avec un trait de 1 px et un rayon de 14 px ; padding vertical des rangées `var(--row-y)` (14 px, ou 9 px sous `[data-density="compact"]`) ; interrupteur 40×24 dont le bouton prend `--on-accent` à l'état coché ; titres de groupe à 12 px en majuscules, tracking .06em, `--ink-3`. Une seule élévation par élément : un trait **ou** une ombre.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(ui): settings primitives restyled on palette tokens`

### Task 11: Écran Apparence

**Files:**
- Create: `src/components/settings/appearance/AppearanceSettings.tsx`, `PalettePicker.tsx`, `PalettePreview.tsx`
- Modify: `src/components/settings/about/AboutSettings.tsx` (retirer `ThemeSelector`), `src/components/settings/ThemeSelector.tsx` (devient un `Segmented` dans Apparence), les locales (`settings.appearance.*` : titre, description, `palettes.<id>.name` et `.tagline` repris du pilote, headings, density, overlay)
- Test: `tests/appearance.spec.ts`

**Interfaces:**
- Consumes : `applyAppearance` et `withThemeTransition` (Task 7), `Segmented` (Task 10), les commandes de la Task 5 via `useSettings().updateSetting`.

- [ ] **Step 1: Write `tests/appearance.spec.ts`:**

```ts
test("each palette applies and persists", ...) // for id of PALETTE_IDS: click card → html[data-palette=id]; calls has {cmd:"change_palette_setting", args:{palette:id}}; card aria-checked=true
test("mode, headings, density apply", ...)     // Sombre → html[data-theme=dark]; Moderne → data-headings=modern; Compact → data-density=compact
test("reload keeps the choice", ...)            // pick ocean, reload → data-palette=ocean before settings resolve
test("picker is keyboard operable", ...)        // focus first card, ArrowRight ×2, Enter → prune
test("overlay label choice maps to settings", ...) // Onde → calls change_overlay_style_setting minimal + change_overlay_show_label_setting false
test("visual baseline", ...)                    // for 8 palettes × light/dark: toHaveScreenshot(`general-${id}-${mode}.png`)
```

- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement**, selon le pilote (`#look`). La grille passe de 4 à 2 colonnes sous 900 px. Chaque carte porte `data-palette={id}`, donc son aperçu se rend dans sa propre palette. La carte active a un contour `accent-text` et une coche ; soulèvement de 2 px au survol. Chaque changement passe par `withThemeTransition`.
- [ ] **Step 4: Run** `bunx playwright test tests/appearance.spec.ts --update-snapshots` une fois, puis **examiner les 16 captures** (avec impeccable : contraste, débordements). Relancer sans `--update-snapshots` → PASS.
- [ ] **Step 5: Commit** `feat(appearance): Appearance page with live palette picker`

---

## Incrément 3 : overlay, pages, finitions

### Task 12: Overlay de dictée

**Files:**
- Modify: `src/overlay/RecordingOverlay.css` (`--s-*` recâblés : `--s-surface: var(--pill)`, `--s-accent: var(--pill-accent)`, texte `--pill-ink` ; `--s-font: var(--font-ui)`), `src/overlay/RecordingOverlay.tsx` (le libellé est masqué si `data-overlay-label="false"`, en gardant un `aria-label` ; « Transcription… » respire en opacité, 1,4 s ; pulsation du point d'enregistrement, 1,6 s)
- Test: `tests/overlay.spec.ts` (charge `/src/overlay/index.html` avec `mockTauri`)
- **Ne pas toucher** : `--ov-*` (les dimensions natives, synchronisées avec `overlay.rs`) ni le pilotage des barres par le niveau audio.

- [ ] **Step 1: Write the tests:**

```ts
test("recolors live on appearance-changed", ...) // emit("appearance-changed", {...,palette:"lime"}) → html[data-palette=lime]; computed pill bg = #141611
test("label hidden in Onde", ...)                // overlay_show_label false → label not visible, pill has aria-label
test("states render per palette", ...)           // emit the existing recording/transcribing events; screenshot overlay-<state>-<palette>.png for rose, sauge, contraste
```

Pour les noms et le format des événements, lire `RecordingOverlay.tsx`.

- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** → PASS ; examiner les captures.
- [ ] **Step 5: Commit** `feat(overlay): palette-driven pill, optional label, gentle motion`

### Task 13: Pages, finitions ciblées

Garder la structure et le comportement de chaque page ; seuls les points ci-dessous changent.

**Files:**
- Modify: `src/components/settings/general/GeneralSettings.tsx` (le raccourci en tête, en carte « Raccourci de dictée » avec les touches en `kbd`), `history/HistorySettings.tsx` (actions de ligne en fondu au survol **et** au `:focus-within` ; heure en chiffres tabulaires), `models/ModelRow.tsx` (pastille « Actif » sur `--accent`/`--on-accent`), `about/AboutSettings.tsx` (wordmark + icône 56 px + ligne `about.forkOf` : « Basé sur Handy, licence MIT »), titres de page en `font-display`
- Modify: les locales (`about.forkOf`)

- [ ] **Step 1: Étendre `branding.spec.ts`** : la page À propos contient le wordmark et le crédit `about.forkOf` (seule mention autorisée de « Handy » seul).
- [ ] **Step 2: Étendre `appearance.spec.ts`** : sur la ligne d'historique, les actions ont l'opacité 0 au repos et 1 après `focus()` sur le bouton copier.
- [ ] **Step 3: Run** → FAIL → implémenter → PASS.
- [ ] **Step 4: Commit** `feat(ui): page-level polish (shortcut card, history actions, about)`

### Task 14: Icône de fenêtre qui suit la palette

**Files:**
- Modify: `src-tauri/src/shortcut/mod.rs` (`change_palette_setting` appelle `apply_window_icon`), `src-tauri/src/lib.rs` (appliquer au démarrage), `src-tauri/src/tray.rs:234` (la variante `Colored` utilise `palette_icon_resource(settings.palette)`)

**Interfaces:**
- Produces : `fn apply_window_icon(app: &AppHandle, p: Palette)`. Elle charge la ressource avec `tauri::image::Image::from_path` et fait `set_icon` sur la fenêtre `main` ; un échec est journalisé en `warn`.

- [ ] **Step 1: Test** (Rust) : `tray_colored_icon_follows_palette`, c'est-à-dire que la fonction qui choisit le chemin de l'icône du tray renvoie `palette_icon_resource(p)` pour `AppTheme::Colored` à l'état Idle.
- [ ] **Step 2: Run** → FAIL → implémenter → `cargo test --release --lib` PASS.
- [ ] **Step 3: Commit** `feat(appearance): window and colored tray icon follow the palette`

### Task 15: Vérification finale et livraison

- [ ] **Step 1: Tout lancer**, sortie verte attendue : `cargo test --release --lib` · `bun run check:branding` · `bun run check:themes` · `bun run check:translations` · `bun src/lib/utils/appearance.test.ts` · `bunx playwright test` · `bun run lint` · `bun run format:check` · `cargo clippy`.
- [ ] **Step 2: Lancer le détecteur impeccable** : `impeccable detect --json src/` ; corriger les vrais défauts ; justifier les faux positifs dans le handoff.
- [ ] **Step 3: Build incrément 3**, puis déploiement dans `Desktop\Handy Flow` (sauvegarde de `Data\` et de l'ancien exe d'abord).
- [ ] **Step 4: Écrire `docs/superpowers/handoff/2026-11-16-handy-flow-brand.md`**. Y mettre : ce qui a été livré, les résultats des tests, les écarts avec la spec, et la **checklist manuelle pour l'utilisateur** :
  1. Gestionnaire des tâches : « Handy Flow ».
  2. Démarrage Windows : une seule entrée, Handy d'origine intact.
  3. Changer de palette fenêtre ouverte, puis dicter : l'overlay a la bonne couleur.
  4. Clair, sombre et Système suivent Windows.
  5. Historique, modèles et clés API toujours là.
  6. Icône du tray lisible sur barre claire et sombre.
- [ ] **Step 5: Commit** `docs: Handy Flow brand handoff and manual checklist`
