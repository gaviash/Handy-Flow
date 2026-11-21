# Handy Flow

Dictée vocale pour Windows : maintenez un raccourci, parlez, relâchez. Le texte arrive, propre et adapté, dans le champ où vous écrivez.

Fork non officiel de [Handy](https://github.com/cjpais/Handy), sans lien avec ses auteurs ni avec Wispr Flow.

## Installation

1. Téléchargez `Handy-Flow-portable-win-x64.zip` depuis la [dernière release](https://github.com/gaviash/Handy-Flow/releases/latest).
2. Décompressez-le où vous voulez, par exemple sur le Bureau.
3. Lancez `Handy Flow.exe`. Un premier écran vous guide : micro, raccourci, choix du modèle.

La version est portable. Vos réglages, votre historique, vos mots appris et vos clés d'API restent dans le dossier `Data` à côté de l'exe. Pour changer de PC, copiez le dossier entier et ressaisissez vos clés d’API. Windows chiffre ces clés pour le compte qui les a enregistrées.

**Mises à jour.** Au démarrage, Handy Flow regarde s'il existe une nouvelle release. Si oui, « Mise à jour disponible » apparaît en bas de la fenêtre. Un clic télécharge la mise à jour, vérifie sa signature, remplace les fichiers et redémarre l'application. Le dossier `Data` n'est jamais touché.

## Utilisation

| Raccourci | Action |
| --- | --- |
| `Ctrl+Maj+Espace` | Dicter : maintenir, parler, relâcher |
| `Ctrl+Espace` | Dicter avec réécriture |
| `Ctrl+Alt+P` | Améliorer le texte sélectionné, sur place |
| `Ctrl+Alt+Espace` | Maintenir et dire une consigne (« rends-le plus formel ») pour transformer la sélection |
| `Échap` | Annuler la dictée en cours |

Tous les raccourcis se changent dans **Général**.

Pendant la dictée, une pastille apparaît en bas de l'écran. En mode **Live**, elle affiche le texte au fur et à mesure que vous parlez. Pendant la réécriture, elle indique le registre choisi, par exemple « Courriel · Formel ». Le style de la pastille se règle dans **Apparence** : Live, Minimal ou Masquée.

## Ce que Handy Flow ajoute à Handy

- **Réécriture.** Les « euh », les hésitations et les autocorrections (« mardi, non, mercredi ») disparaissent. La ponctuation, les nombres et les listes sont mis en forme.
- **Adaptation au contexte.** Le ton suit l'application : courriel, messagerie pro, messagerie perso, document, code, prompt d'IA. Un ton se choisit par catégorie, avec une phrase d'exemple. Les noms visibles à l'écran, comme le destinataire ou l'interlocuteur, sont orthographiés correctement.
- **Apprentissage.** Si vous corrigez un mot juste après l'avoir dicté, Handy Flow le retient et vous le signale. Un mot appris qui n'est ni corrigé ni redicté pendant 30 jours est oublié. La mémoire est chiffrée, consultable, exportable et effaçable.
- **Snippets.** Une expression dite seule, par exemple « signature », insère le texte associé tel quel. Un aperçu montre ce qui sera inséré.
- **Historique.** Chaque dictée garde son audio, avec sa forme d'onde, l'application où elle a été collée et sa langue. « Voir les retouches » barre les mots retirés par la réécriture et surligne les ajouts. Vous pouvez aussi transcrire un fichier audio.
- **Compteur du jour.** La page Général affiche les mots dictés aujourd'hui et une estimation du temps gagné par rapport à la frappe.

## Transcription et réécriture

**Transcription.** Par défaut, la transcription se fait sur votre PC, avec un modèle local à télécharger dans **Modèles** (Parakeet, Whisper, Qwen3-ASR…). Vous pouvez aussi passer par un fournisseur d'API de transcription, par exemple Whisper d'OpenAI. Dans ce cas, l'audio de la dictée lui est envoyé. Le modèle local prend le relais si l'API échoue, et il transcrit toujours les champs de mot de passe et les applications exclues.

**Réécriture.** Elle passe par un modèle de langage (LLM) chez le fournisseur de votre choix : OpenAI, Anthropic, OpenRouter, ou tout service compatible avec l'API d'OpenAI. Un second fournisseur peut prendre le relais si le premier ne répond pas. Il reçoit le texte dicté et un contexte minimal, et vous choisissez quelles parties du contexte lui envoyer dans **Post-traitement**.

Dans **Modèles**, configurez vos fournisseurs et leurs clés d’API, puis associez un modèle à chaque fonction. Les clés restent sur votre PC.

## Confidentialité

Rien n'est envoyé à une API depuis un champ de mot de passe, ni depuis les applications et sites exclus : gestionnaires de mots de passe, banques, etc. La liste se modifie dans **Post-traitement**.

## Compiler depuis les sources

Il faut Rust, Bun et le Vulkan SDK. Le détail est dans [BUILD.md](BUILD.md).

```bash
bun install
bun run tauri build --no-bundle
```

Le build `release` réoptimise tout le programme (LTO) et prend environ 8 minutes. Pour essayer une modification, le profil `fast` (sans LTO, incrémental) est bien plus rapide après un premier build complet :

```bash
bun run tauri build --no-bundle -- --profile fast
```

L'exécutable est alors `src-tauri/target/fast/Handy Flow.exe`. Pour publier une version, voir [docs/release-portable.md](docs/release-portable.md).

## Licence

MIT, comme Handy : voir [LICENSE](LICENSE). Basé sur Handy de CJ Pais, et sur Tauri, transcribe.cpp et Silero VAD.
