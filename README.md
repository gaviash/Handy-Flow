# Handy Flow

Dictée vocale pour Windows : maintenez un raccourci, parlez, relâchez. Le texte arrive, propre et adapté, dans le champ où vous écrivez.

Fork non officiel de [Handy](https://github.com/cjpais/Handy), sans lien avec ses auteurs ni avec Wispr Flow.

## Ce que ça ajoute à Handy

- **Reformulation automatique.** Les « euh », les hésitations et les autocorrections (« mardi, non, mercredi ») disparaissent. La ponctuation, les nombres et les listes sont mis en forme.
- **Adaptation au contexte.** Le ton suit l'application (email formel, messagerie décontractée). Les noms affichés à l'écran (destinataire, interlocuteur) sont orthographiés correctement.
- **Apprentissage.** Quand vous corrigez un mot juste après la dictée, Handy Flow le retient. La mémoire est chiffrée, consultable et exportable.
- **Polish et instructions vocales.** Sélectionnez un texte :
  - `Ctrl+Alt+P` l'améliore sur place ;
  - `Ctrl+Alt+Espace` maintenu, puis une consigne dite (« rends-le plus formel »), le transforme.
- **Snippets, styles et transformations enregistrées**, chacune avec son raccourci.
- **Confidentialité.** Rien n'est envoyé depuis un champ de mot de passe ni depuis les applications et sites exclus (gestionnaires de mots de passe, banques…).

Tout se règle dans l'application.

## Fonctionnement

- La transcription reste locale (Parakeet).
- La reformulation passe par un fournisseur LLM au choix, par défaut [Groq](https://console.groq.com). Il reçoit le texte dicté et un contexte minimal.
- Un fournisseur de secours ([Cloudflare Workers AI](https://developers.cloudflare.com/workers-ai/)) prend le relais si Groq atteint son quota ou ne répond pas.

Il vous faut vos propres clés d'API, à saisir dans **Post-traitement**.

## Installation

Aucune version compilée n'est publiée pour l'instant. Pour compiler depuis les sources (Rust, Bun, Vulkan SDK), suivez [BUILD.md](BUILD.md) :

```bash
bun install
bun run tauri build --no-bundle
```

Le build `release` réoptimise tout le programme (LTO) et prend environ 8 minutes. Pour essayer une modification, utilisez le profil `fast`, sans LTO et incrémental, donc nettement plus rapide après un premier build complet :

```bash
bun run tauri build --no-bundle -- --profile fast
```

L'exécutable est alors dans `src-tauri/target/fast/handy.exe`. Gardez `release` pour une version à distribuer.

## Licence

MIT, comme Handy : voir [LICENSE](LICENSE). Basé sur Handy de CJ Pais, et sur Tauri, transcribe.cpp et Silero VAD.
