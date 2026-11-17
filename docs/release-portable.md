# Publier une mise à jour de Handy Flow

Les copies portables de Handy Flow (0.2.1 et suivantes) regardent au démarrage la dernière release de `gaviash/Handy-Flow`. Elles proposent la mise à jour quand le tag est plus récent que leur version. L'installation n'a lieu que si le ZIP porte une signature faite avec la clé privée du fork.

## La clé

- Clé privée : `%USERPROFILE%\.handy-flow\updater.key`. Elle n'a pas de mot de passe et ne doit jamais être commitée. Garde-en une copie hors du PC : si tu la perds, les copies déjà installées ne pourront plus se mettre à jour toutes seules.
- Clé publique : `PUBKEY` dans `src-tauri/src/self_update.rs`.

## À chaque release

1. Mets la même version, par exemple `0.2.2`, dans `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` et `package.json`. Lance ensuite les tests Rust avec `UPDATE_FIXTURES=1` : `tests/fixtures/app-settings.json` contient la version et se met à jour.
2. Construis le ZIP portable `Handy-Flow-portable-win-x64.zip`. Le ZIP ne doit pas contenir de dossier `Data`. S'il en contient un, la mise à jour l'ignore.
3. Signe le ZIP :

   ```powershell
   bun tauri signer sign -f "$env:USERPROFILE\.handy-flow\updater.key" -p "" Handy-Flow-portable-win-x64.zip
   ```

   La commande crée `Handy-Flow-portable-win-x64.zip.sig`.
4. Publie la release avec le tag `flow-v0.2.2` et joins les deux fichiers :

   ```powershell
   gh release create flow-v0.2.2 -R gaviash/Handy-Flow Handy-Flow-portable-win-x64.zip Handy-Flow-portable-win-x64.zip.sig
   ```

Si la release n'a pas de `.sig` ou si la signature ne correspond pas, l'app refuse la mise à jour et ne touche à rien.

## Ce que fait l'app

L'app télécharge le ZIP et vérifie la signature. Elle décompresse ensuite le ZIP dans `_update\`, à côté de l'exe. Elle remplace les fichiers un par un, met les anciens dans `_old\`, puis redémarre. Au démarrage suivant, elle supprime `_old\` et `_update\`. Si un fichier ne peut pas être remplacé, elle remet les anciens en place. `Data\` n'est jamais modifié. Un fichier supprimé d'une version à l'autre reste dans le dossier.
