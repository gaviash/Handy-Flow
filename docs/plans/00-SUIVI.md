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
| **0 — Corrections de bugs** | **Fait.** Codé, testé (562 tests Rust réussis), relu par Codex, fusionné dans `main`. |
| 1 à 15 | **À faire.** Aucun n'est codé. Chaque plan a une section **J** (mise à jour du 26/11) à lire avant de coder. |

Prochain : **plan 1 (transcription propre)**. Le plan 5 est à revoir avant de coder (apprentissage circulaire, voir sa section J).

**27/11 : analyse de l'apprentissage (logs du 26–27/11).** Plans 2, 3, 4 et 5 : nouvelle section **K** à lire aussi. Résumé : 0 mot appris en 32 suivis, car tu corriges rarement. Décisions validées :
- validateur `clef-omni` (modèle de décision) ;
- vocabulaire par défaut, paire après 2 corrections vers le même mot ;
- Whisper reçoit le vocabulaire trié, le LLM le contexte plus le vocabulaire ;
- apprentissage sans correction (signaux A et B) ;
- support des modèles de décision à l'ajout d'un modèle.

Le plan 4 passe à 3–3,5 j. Banc d'essai : 40 corrections, script hors dépôt.

**Incident du 27/11** : le banc d'essai a épuisé le quota gratuit Cloudflare du jour (10 000 neurons), donc la reformulation Cloudflare était coupée jusqu'à 01 h.

**Aucune release n'a été faite.** Le code est dans `main` du fork, mais aucune version n'a été publiée ni installée dans `Desktop\Handy Flow` (à ta demande).

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
