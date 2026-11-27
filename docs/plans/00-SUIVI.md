# Suivi des plans (mis à jour le 2026-11-27)

## Règles pour tous les plans (à lire avant de coder)

1. **Aucune régression.** Les dictées actuelles sont bonnes ; l'apprentissage et le contexte marchent plutôt bien. Chaque plan doit **améliorer** sans rien dégrader. Avant de changer un comportement de dictée, de contexte ou d'apprentissage :
   - les tests existants restent verts ;
   - un test fige le comportement actuel s'il n'y en a pas ;
   - on compare avant et après sur des cas réels (historique, labo de prompt) ;
   - en cas de doute, l'ancien comportement reste.
2. **Méthode** : Superpowers (TDD, vérifier avant d'affirmer), Ponytail (le plus petit changement complet), Impeccable pour toute interface.
3. **Codex** pour l'aide et les revues : `gpt-6.2-sol`, raisonnement moyen, lecture seule. Chaque demande lui donne le contexte du plan et lui rappelle Ponytail, Superpowers et la règle 1. Jamais les données personnelles de `Desktop\Handy Flow\Data`.

## Où on en est

| Plan | État |
|---|---|
| **0 — Corrections de bugs** | **Fait.** Codé, testé (562 tests Rust réussis), relu par Codex, fusionné dans `main`. Publié dans la 0.3.0. |
| **1 — Transcription propre** | **Fait, sauf ton essai réel** (tâche 6). Codé, 577 tests Rust, relu par Codex, fusionné dans `main`, publié en **0.3.0** et installé le 27/11. |
| 2 à 15 | **À faire.** Chaque plan a une section **J** (26/11), et les plans 2 à 5 une section **K** (27/11), à lire avant de coder. |

Prochain : **d'abord réparer les tests Rust** (section « Tests Rust qui ne démarrent plus », plus bas, environ 20 min), puis **plan 2 (capture dès l'appui et icône)**, après ton essai du plan 1 (voir plus bas). Le plan 5 est à revoir avant de coder (apprentissage circulaire, voir sa section J).

**27/11 : analyse de l'apprentissage (logs du 26–27/11).** Plans 2, 3, 4 et 5 : nouvelle section **K** à lire aussi. Résumé : 0 mot appris en 32 suivis, car tu corriges rarement. Décisions validées :
- validateur `clef-omni` (modèle de décision) ;
- vocabulaire par défaut, paire après 2 corrections vers le même mot ;
- Whisper reçoit le vocabulaire trié, le LLM le contexte plus le vocabulaire ;
- apprentissage sans correction (signaux A et B) ;
- support des modèles de décision à l'ajout d'un modèle.

Le plan 4 passe à 3–3,5 j. Banc d'essai : 40 corrections, script hors dépôt.

**Incident du 27/11** : le banc d'essai a épuisé le quota gratuit Cloudflare du jour (10 000 neurons), donc la reformulation Cloudflare était coupée jusqu'à 01 h.

**Release 0.3.0 (27/11)** : plans 0 et 1. Tag `flow-v0.3.0` sur gaviash/Handy-Flow, ZIP portable signé avec `%USERPROFILE%\.handy-flow\updater.key` (procédure : `docs/release-portable.md`). Installée dans `Desktop\Handy Flow` le 27/11 à 16 h 54 ; sauvegarde complète avant installation : `backups/handy-flow-20261127-165412/`.

## Plan 1 : ce qui a été fait (27/11)

Branche `feat/plan-1-transcription` (poussée), fusionnée dans `main` (`6eee6be3`). Commits : prompt vocabulaire seul, retrait de la copie en tête, second appel sans prompt, interface, corrections de revue, version 0.3.0.

- **Prompt Whisper = vocabulaire seul** (`asr_context.rs`) : `Vocabulaire : a, b.` (`Vocabulary:` en anglais), ton nom (`user_name`) en tête, plafond estimé de 200 jetons en plus des 600 caractères. Plus d'expressions fréquentes, d'application ni de texte avant le curseur (code, réglage, commande `get_frequent_expressions` et lignes du rapport supprimés).
- **Copie en tête retirée** (`strip_echo`) : une copie étiquetée « Vocabulaire » de 3 mots ou plus, ou le prompt entier, est retirée. La coupe s'arrête à la fin de la phrase copiée. Sans l'étiquette, rien n'est retiré.
- **Prompt entièrement recopié** (`cloud_asr::without_echo`) : le même audio est redemandé une fois sans prompt. Un seul budget de temps pour les deux appels, compté depuis le premier appel. Échap coupe les deux.
- **Mots personnalisés** (`gave_all`) : leur correction locale n'est sautée que si **tous** sont arrivés au modèle.
- **Historique réparé** : n° 301, 306 et 313 remplacés par les résultats de l'essai du 24/11 ; n° 286 et 291 supprimés. Les 25 autres lignes polluées par « « » n'ont pas de version propre : laissées telles quelles.
- **Contrôle « aucune régression »** : le prompt exact sur tes 20 enregistrements du 24/11 donne 0 début pollué (10 sur 20 avec l'ancien), 964 mots contre 963 pour « vocabulaire seul », et chaque texte est identique à au moins 90 %.

**Décisions prises en codant (à connaître)**
- Prises de moins de 500 ms **non** ignorées (J A2) : l'historique contient de vraies dictées d'un mot (« Codex. », « Bonjour. »).
- Balises `[BLANK_AUDIO]` non traitées (J B7) : aucune dans 352 dictées.
- Tri du vocabulaire (J B4) reporté au plan 4 (N4) : l'écran y passera en dernier.
- Modèles locaux qui acceptent un prompt de contexte : ils perdent aussi l'application et le texte précédent. Ton modèle local (Parakeet) n'en prend pas. À rétablir pour ce seul cas après un essai A/B, si un tel modèle sert un jour.
- Non corrigé, mineur : `strip_echo` copie le texte même sans écho (coût négligeable).

**À faire par toi (tâche 6)**
1. Réglages → Contexte, rapport de la dernière dictée : le prompt commence par « Vocabulaire : » et finit par un point.
2. Une vingtaine de dictées normales. Attendu : aucun début pollué, aucune phrase perdue. L'agent suivant contrôle dans l'historique.
3. Ensuite : cocher K1 dans `PLAN_LOTS.md`.

**Retour arrière** : fermer Handy Flow, recopier l'exe, les DLL et `resources\` depuis `backups/handy-flow-20261127-165412/`. `history.db` seulement en cas de besoin, car les dictées faites depuis seraient perdues.

## Tests Rust qui ne démarrent plus (à corriger en premier)

**Contexte.** Le 27/11, en préparant la release 0.3.0 (plans 0 et 1), la version est passée de 0.2.9 à 0.3.0 dans `Cargo.toml`, `tauri.conf.json` et `package.json`. Ce changement relance `build.rs` et recompile le binaire de test, qui ne démarre plus depuis. Le code du plan 1 avait été testé juste avant, en 0.2.9 : 577 tests réussis. La release a été publiée parce que seule la version avait changé et que l'exe 0.3.0 démarre (`--help` répond avec le code 0). **Pourquoi c'est bloquant** : sans tests qui tournent, impossible de travailler en TDD ni de vérifier la règle « aucune régression » sur le plan 2. Ça ne touche ni l'utilisateur ni l'app installée.

**Symptôme.** Depuis le passage en 0.3.0, `cargo test --release --lib` compile, puis le binaire de test s'arrête au chargement : `STATUS_ENTRYPOINT_NOT_FOUND` (0xC0000139), avant tout test. Le code n'est pas en cause : les mêmes 577 tests passaient en 0.2.9, et l'app 0.3.0 démarre normalement.

**Diagnostic (vérifié le 27/11).**
1. Le binaire de test importe `TaskDialogIndirect` de `comctl32.dll` (via Tauri). Vérifié avec `dumpbin /imports`.
2. Cette fonction n'existe que dans `comctl32` **version 6**. Windows ne charge la version 6 que si l'exe a un manifeste qui la demande (« Microsoft.Windows.Common-Controls » 6.0.0.0). Sinon il charge la version 5, d'où l'arrêt.
3. Le binaire de test **n'a aucun manifeste** (`mt.exe` : « ne contenait pas de section ressource »). L'app en a un (Common-Controls présent).
4. Pourquoi : `tauri-build` 2.6.3 → `tauri-winres` 0.3.5 → `embed-resource` 3.0.6 `compile()` joint `resource.lib` (manifeste compris) avec `cargo:rustc-link-arg-bins` (`embed-resource-3.0.6/src/lib.rs:444`), donc **aux exécutables de l'app seulement, jamais aux tests**.
5. Non expliqué, et sans effet sur la correction : pourquoi le binaire de test 0.2.9 démarrait. Il a été effacé par un `cargo clean -p handy` resté à moitié (fichier verrouillé). Ne pas chercher davantage.

**Pistes déjà écartées, ne pas refaire.** Runtime MSVC (`HANDY_VC_REDIST_DIRS`), `STATIC_VCRUNTIME=true`, DLL de runtime copiées à côté du binaire de test : aucune ne change rien.

**Correction à faire.** Dans `src-tauri/build.rs`, joindre aux seuls binaires de test un manifeste qui demande Common-Controls v6, sans toucher à celui de l'app. Uniquement pour une cible Windows MSVC (tester `CARGO_CFG_TARGET_OS` et `CARGO_CFG_TARGET_ENV`, pas `cfg!(windows)`, qui décrit la machine de build) :

```rust
println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
println!("cargo:rustc-link-arg-tests=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
```

- **À vérifier d'abord** : `rustc-link-arg-tests` doit bien s'appliquer au binaire de tests unitaires de la bibliothèque (`cargo test --lib`), et pas seulement aux tests du dossier `tests/`. Contrôle : `mt.exe -inputresource:"<binaire de test>;#1" -out:m.xml`, puis chercher « Common-Controls » dans `m.xml`.
- **Si ce n'est pas le cas** : ne pas utiliser `cargo:rustc-link-arg` (sans suffixe), qui s'appliquerait aussi à l'app et ferait un manifeste en double avec `resource.lib`. Solution de repli : `cargo:rustc-link-arg-tests=/DELAYLOAD:comctl32.dll` avec `delayimp.lib`, pour que la DLL ne soit chargée qu'au premier appel. Les tests n'appellent jamais de boîte de dialogue.
- Après `build.rs` modifié, `cargo` relance le script de build tout seul. Sinon, `touch build.rs`.

**Vérification attendue avant de commit.**
1. `cd src-tauri && PATH="$PWD/transcribe-libs:$PATH" cargo test --release --lib` : 577 réussis, 0 échec, en 0.3.0.
2. L'app garde **un seul** manifeste : `bun run tauri build --no-bundle -- --profile fast`, puis `mt.exe` sur `target/fast/Handy Flow.exe` (Common-Controls présent, pas d'erreur de ressource en double), et `"Handy Flow.exe" --help` répond avec le code de sortie 0.
3. Commit `fix(build): embed the Common-Controls manifest in test binaries`. Pas besoin de release : rien ne change pour l'utilisateur.

**Outils.**
- `dumpbin` : `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\dumpbin.exe`.
- `mt.exe` : `C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\mt.exe`.
- Binaire de test : `src-tauri\target\release\build\handy\<empreinte>\out\handy_app_lib-<empreinte>.exe` (chemin affiché par `cargo test` sur la ligne « Running unittests »).

**Pour une release (sans rapport avec ce problème).** Définir `HANDY_VC_REDIST_DIRS` sur les dossiers redist de VS Build Tools 14.44 (`...\VC\Redist\MSVC\14.44.35112\x64\Microsoft.VC143.CRT` et `...\Microsoft.VC143.OpenMP`, séparés par `;`). Sans lui, `build.rs` recrée `transcribe-libs` sans les DLL `msvcp140`/`vcruntime140`/`vcomp140`, que le ZIP portable doit contenir.

**Connu, sans effet** : au démarrage, 9 erreurs « Hotkey cannot be empty » pour les transformations sans raccourci. Code inchangé, déjà présent avant.

## Plan 0 : ce qui a été corrigé

Branche `fix/plan-0-bugs` (commit `7f40d6d1`), fusionnée dans `main` (commit `cfd57a6d`), poussée sur `origin` (gaviash/Handy-Flow) le 27/11. Détail et preuves : `willow/NN-v2.md`.

**Lot 1 — Pertes de données**
- `learned.bin` illisible : mis de côté en `learned.bad-…` au lieu d'être écrasé, avec une notice.
- Échec d'enregistrement de la mémoire : notice au lieu du silence.
- Annulation (Échap) : le WAV est supprimé ; une consigne échouée reste dans l'historique.

**Lot 2 — Confidentialité**
- Balises de contexte imbriquées (`</con</context>text>`) neutralisées ; le code dicté (`a < b`, `->`) n'est plus abîmé.
- Dictées faites dans une app exclue : marquées privées, plus jamais renvoyées comme « dictées passées » ; dictées passées masquées (e-mails…).
- **Ton choix (26/11)** : un site non reconnu ou une vérification mot de passe sans réponse **n'empêche plus** la reformulation. Seuls un champ mot de passe détecté ou une app/un site exclu la bloquent.

**Lot 3 — Pannes muettes et texte perdu**
- Micro qui lâche pendant la dictée : notice « Le micro ne répond plus » (aussi si seul le début a été collé).
- Parole d'au moins ~1,25 s sans texte : notice « Rien n'a été reconnu ».
- Échap rétablit le volume coupé pendant la dictée.
- Collage non confirmé : le texte reste dans le presse-papiers et la notice le dit ; notice « app admin » plus écrasée ; collage bloqué en transformation signalé.
- Champ mot de passe : jamais laissé au presse-papiers (restauration réessayée, sinon presse-papiers vidé).
- Polish/transformations : plus de Ctrl+C quand rien n'est sélectionné (plus de commande interrompue dans le terminal, plus de ligne dupliquée).

**Lot 4 — Exactitude des mots**
- Un mot corrigé plusieurs fois est appliqué même sans reformulation.
- « Annuler » retire seulement la forme annoncée (le mot redevient en attente) et elle n'est plus réapprise.
- `npm`, `git`, `sql` peuvent être appris ; apostrophes gardées (O'Brien).
- Remplacement flou : un vrai mot (« Steven ») n'est plus changé en mot personnalisé (« Stefan »).
- « non non non », « très très très bien » gardés ; « C'est prêt euh. » garde son point.
- Une dictée qui n'est qu'un nom de branche ou quelques mots du vocabulaire n'est plus vidée (relance sans prompt).

**Lot 5 — Réactivité**
- Délai de reformulation selon la longueur (7 s + 60 ms/mot, ton réglage reste le minimum, plafond 60 s).
- Secours identique au principal : un second essai au lieu du texte brut.
- Échap pendant une instruction vocale libère tout de suite la file.
- « Mots appris » n'écrase plus la pastille d'une dictée en cours.

## Reporté (décisions prises pendant le plan 0)

| Reporté | Où | Risque en attendant |
|---|---|---|
| « Collé » affiché seulement après confirmation du collage | Plan 14 | « Collé » peut précéder un échec (le texte est alors au presse-papiers). |
| Résultat d'une instruction vocale dans l'historique (avant/après) | Plans 5 et 7 | Retrouvable seulement via le presse-papiers. |
| Sauvegarde de tous les formats du presse-papiers lors du repli Ctrl+C | Plan 7 | HTML/fichiers copiés perdus dans Google Docs lors d'une transformation. |
| Échap pour Polish et les touches de transformation | Plan 7 | Un Polish lent ne s'annule pas. |
| Accents dans le remplacement flou (Bérénice) | Plan 4 | Les noms accentués ne sont pas corrigés par ce chemin. |
| File de notices (course pastille / « mots appris ») | Plan 15 | Très rare : la notice peut passer devant une dictée qui démarre au même instant. |
| Choisir un secours de reformulation différent du principal | **Toi, dans les réglages** | Si Cloudflare est indisponible, le second essai échoue aussi. |

## À vérifier par toi (rien n'a été testé en vrai)

Rien n'a été lancé sur ta machine (ni micro, ni collage, ni modèle). Après la prochaine installation :
1. Quelques dictées dans Gmail et Chrome : la reformulation doit marcher.
2. Une dictée longue (≥ 45 s) : elle doit être reformulée.
3. Échap pendant une dictée avec « couper le son » activé : le son doit revenir.

## Fichiers utiles

- `00-LISTE-DES-PLANS.md` : la liste et l'ordre des plans.
- `00-CORRECTIONS-BUGS.md` : le plan 0.
- `willow/00-PASSE-2.md` : synthèse des trouvailles Willow/Wispr ; `willow/NN-v2.md` : détail par plan.
