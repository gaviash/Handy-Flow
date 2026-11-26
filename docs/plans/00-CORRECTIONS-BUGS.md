# Plan 0 : corrections de bugs (2026-11-26)

Source : passe 2 approfondie (`willow/00-PASSE-2.md`, détail et preuves dans `willow/NN-v2.md`, référencés ici « 04·A3 » = `willow/04-v2.md` §A3).
À faire **avant le plan 1** : les plans 4, 5, 6, 11, 12 reposent sur `learned.bin` et sur la neutralisation du contexte.
Seuls les bugs du **code actuel** sont ici. Les défauts qui n'existent qu'à travers un plan restent dans la section J de ce plan.

## A. But
Supprimer les pertes de données, les fuites de confidentialité et les pannes muettes avant d'empiler de nouvelles fonctions.

## B. Règle de travail
1. **Vérifier avant de corriger.** Tout vient d'une lecture de code, rien n'a été exécuté. Pour chaque bug : relire le code cité, écrire le test qui échoue (`cargo test`), puis corriger. Si le bug n'est pas confirmé, le noter « non reproduit » et passer.
2. Un lot = un build. Remplacer Handy Flow après build (Data\ intact).
3. Pas de dictée simulée ni de clavier/audio pilotés : les cas audio/collage se testent par tests unitaires ou par toi.

## C. Lots

### Lot 1 — Pertes de données (≈ 2 h)
| # | Bug | Réf. | Correction | Effort |
|---|---|---|---|---|
| 1.1 | `learned.bin` illisible (DPAPI, autre compte, format) → `Memory::default()` puis écrasé au prochain `save` : mots, règles, préférences perdus | 04·A3, 05·A4, 06·A1, 12·A2 | Fichier présent mais illisible : le renommer `learned.bad` (horodaté) avant tout `save`, journaliser, notice une fois | 0,5 h |
| 1.2 | `save()` avale les échecs d'écriture | 06·A4 | Propager l'erreur, journaliser, notice | 0,5 h |
| 1.3 | Annulation Échap : WAV laissés sur disque ; deux sorties « instruction » perdent la consigne | 14·A4 | Supprimer le WAV à l'annulation ; garder la consigne | 1 h |

### Lot 2 — Confidentialité et injection (≈ 5 h)
| # | Bug | Réf. | Correction | Effort |
|---|---|---|---|---|
| 2.1 | `unframe` contournable par imbrication (`</con</context>text>` → `</context>`) | 09·A1, 11·A2, 12·A1, 06·A9 | Neutraliser en boucle jusqu'à point fixe (ou échapper `<`), test avec imbrications | 0,5 h |
| 2.2 | Hôte du site introuvable (nom de barre d'adresse localisé, recherche non bornée) → contrôle mot de passe et domaines exclus sautés | 02·A1, 09·A3 | Échec de lecture de l'hôte = « non vérifié » (privé), jamais « autorisé » ; borne de temps ; lecture par le `Document` en second | 1,5 h |
| 2.3 | Champ exclu ou non vérifié : le texte part quand même au LLM, sans message | 02·A3, 02·A4 | Pas de reformulation cloud dans ces cas, notice discrète | 1 h |
| 2.4 | Dictée faite dans une app exclue enregistrée et renvoyée comme « dictée passée » ; dictées passées non masquées | 05·A5, 06·A6 | Marquer l'entrée privée, l'exclure de `past_dictations` ; appliquer `redact` aux dictées passées | 1,5 h |
| 2.5 | Garde mot de passe / exclu de la retouche ignore « lecture non faite » | 07·A6 | Même règle que 2.2 | 0,5 h |

### Lot 3 — Pannes muettes et texte perdu (≈ 6 h)
| # | Bug | Réf. | Correction | Effort |
|---|---|---|---|---|
| 3.1 | Micro sans son / bloqué : le recorder le détecte (10 s / 5 s) mais seul un drapeau est levé → dictée vide sans message | 13·A1 | Remonter le drapeau en notice « Micro muet » | 1 h |
| 3.2 | Échap peut laisser le son système coupé (`cancel_recording` ne rétablit pas le volume) | 13·A2 | Rétablir le volume sur tous les chemins de sortie | 0,5 h |
| 3.3 | `PasteOutcome::Blocked` traité comme succès (Transform) ; résultat d'une instruction absent de l'historique si le collage échoue | 12·A4, 07·A2 | Blocked = échec ; garder le résultat à l'historique | 1 h |
| 3.4 | Texte non lu par la cible : l'ancien presse-papiers est restauré, le texte est perdu ; échec signalé après 8 s, après un « collé » ; notice admin écrasée ; message `PasteFailed` ne dit pas où est le texte | 14·A1, A2, A3, A7 | Laisser le texte au presse-papiers en cas d'échec, dire « texte dans le presse-papiers » ; ne pas afficher « collé » avant l'accusé ; garder la notice admin | 2 h |
| 3.5 | Repli Ctrl+C « rien n'est sélectionné » : interrompt une commande ou duplique la ligne dans le terminal VS Code ; `copy_selection` détruit les formats non texte | 07·A1, 07·A12 | Pas de Ctrl+C dans les terminaux ; sauvegarder/restaurer tous les formats | 1,5 h |
| 3.6 | Résultat vide après vraie parole : disparition silencieuse | 01·A3 | Notice « rien reconnu » | 0,5 h |

### Lot 4 — Exactitude des mots (≈ 5 h)
| # | Bug | Réf. | Correction | Effort |
|---|---|---|---|---|
| 4.1 | Mot appris corrigé plusieurs fois jamais appliqué sans reformulation (`memory_guard` « known » contre `count+used>=2`) ; les tests contournent le garde | 04·A1 | Aligner les deux conditions ; test sans contournement | 1 h |
| 4.2 | « Annuler » sur un mot nouveau ne retient pas la paire | 04·A2 | Mémoriser le refus | 0,5 h |
| 4.3 | Termes de 3 lettres en minuscules (npm, sql) rejetés ; apostrophes supprimées à la saisie | 04·A6, A7 | Accepter, conserver | 0,5 h |
| 4.4 | Remplacement Soundex aveugle (Steven → Stefan) ; correction locale aveugle aux accents ; dédoublonnage ASCII seulement | 04·A8, 01·A7, A11 | Exiger une distance d'édition max en plus de Soundex ; comparer en NFC avec accents | 1,5 h |
| 4.5 | `collapse_stutters` écrase « non non non » ; les fillers mangent le point final | 01·A8, A9 | Liste d'exceptions ; garder la ponctuation | 1 h |
| 4.6 | Une dictée de 3 mots qui n'est qu'un terme du vocabulaire/nom de branche est vidée par l'anti-écho | 01·A4, 10·A4 | L'anti-écho exige l'étiquette du prompt ou une longueur minimale | 0,5 h |

### Lot 5 — Réactivité et confort (≈ 3 h)
| # | Bug | Réf. | Correction | Effort |
|---|---|---|---|---|
| 5.1 | Budget de reformulation fixe : 3 dictées longues du 26/11 collées brutes (10 échecs / 267) | 08·A1 | Budget proportionnel à la longueur | 1 h |
| 5.2 | « Secours » de reformulation identique au principal dans ta config | 08·A2 | Avertir dans les réglages ; choisir un secours différent (ton choix) | 0,25 h |
| 5.3 | Échap ne coupe pas l'appel du modèle d'une instruction ; file bloquée jusqu'à 20 s | 07·A3 | Annulation propagée à l'appel | 1 h |
| 5.4 | Notice « mots appris » remplace la pastille pendant une dictée longue | 15·A1 | Différer la notice après la session | 0,5 h |

**Non inclus volontairement** : maintien de touche 100 ms (08·A6, 14·A6) et modificateurs tenus (14·A5) → plan 14 ; style circulaire (05·A3) → plan 5 ; suivi des corrections (03·A1–A6) → plan 3.

## D. Acceptation
- Chaque bug confirmé a un test qui échouait avant et passe après.
- `learned.bin` corrompu à la main : fichier `.bad` créé, rien d'écrasé.
- Chaîne `</con</context>text>` : aucune balise fermante ne survit.
- Hôte illisible : dictée traitée comme privée (pas de LLM).
- Build, installation dans Handy Flow, Data\ intact.

## E. Tâches
1. Lot 1 (vérifier, tester, corriger), build.
2. Lot 2, build.
3. Lot 3, build.
4. Lot 4, build.
5. Lot 5, build, installation.

Total estimé : 20 à 22 h de développement, hors essais réels.
