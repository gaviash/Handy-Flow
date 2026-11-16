# Handy Flow — branding, thèmes et polish (spec)

Référence visuelle validée par l'utilisateur : `docs/branding/pilote-handy-flow-v2.html` (ouvrir dans Chrome). En cas de doute sur un rendu, le pilote fait foi.

## 1. Nom et identité

| Élément | Avant | Après |
|---|---|---|
| Nom produit (`productName`, titre fenêtre, infobulle tray, `<title>`) | Handy | **Handy Flow** |
| Exécutable / processus Windows | `handy.exe` | **`Handy Flow.exe`** (`mainBinaryName: "Handy Flow"`), description de fichier « Handy Flow » |
| CLI (`cli.rs` `name`) | `handy` | `handy-flow` |
| Nom de thread worker Linux (`prctl`, ≤ 15 octets) | `Handy-worker` | `HFlow-worker` |
| Titre de la fenêtre overlay | Recording | `Handy Flow Overlay` |
| `identifier` | `com.pais.handy` | **inchangé** (sinon les réglages, l'historique et les modèles changent de dossier) |
| Marqueur portable `Handy Portable Mode`, dossier `Data\` | | **inchangés** |
| Chemins Linux `/usr/lib/Handy` | | inchangés (hors périmètre) |
| Updater | releases de `cjpais/Handy` | **désactivé** dans le fork : aucune mise à jour ne doit installer Handy d'origine par-dessus Handy Flow |

Wordmark : « Handy » en Figtree 800 (tracking -0.035em), suivi de « Flow » en Instrument Serif italique, à 1,18 × la taille, couleur `--accent-text`. Composant texte, pas de SVG tracé.

Icône : la main de `HandyHand.tsx` (inclinée de -8°, 62 % de la tuile), sur une tuile squircle (rayon 23 %), avec un point d'écoute (16 %, en haut à droite). L'icône d'installation et l'exe utilisent la palette Rose. À l'exécution, l'icône de la fenêtre suit la palette choisie. Les icônes monochromes de la zone de notification restent celles d'aujourd'hui, sauf la variante colorée (Linux), qui suit la palette.

## 2. Thèmes

Thème = **palette** × **mode**. Le mode reste le réglage `theme` existant (`system | light | dark`), qui pose déjà `data-theme` et `color-scheme`. Les palettes utilisent `light-dark()`, donc le mode ne fait que changer `color-scheme`.

Chaque palette définit exactement 13 tokens. Valeurs en `clair | sombre` ; une seule valeur s'applique aux deux modes :

| Palette | bg | ink | accent | on-accent | accent-text | side | side-ink | pill | pill-ink | pill-accent | icon-bg | icon-fg | icon-dot |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| rose | #faf7ef \| #18141a | #1c1915 \| #f6f1ea | #f28cbb \| #f6a5c8 | #3b1d2c \| #2a1320 | #b13c72 \| #f6a5c8 | #f3eee2 \| #141016 | =ink | #1c1915 \| #2c2530 | #faf7ef | #f28cbb \| #f6a5c8 | #f28cbb \| #2a2330 | #3b1d2c \| #f6a5c8 | #1c1915 \| #c9b6f2 |
| sauge | #f5f3e8 \| #0f1916 | #171a16 \| #eef0e6 | #12443d \| #8fd3bd | #f5f3e8 \| #0c2420 | #12443d \| #8fd3bd | #12443d \| #0b2420 | #f2f1e6 | #12443d \| #173a33 | #f5f3e8 | #f39ac2 | #12443d | #f5f3e8 | #f39ac2 |
| prune | #f7f3fa \| #141117 | #1d1622 \| #f4eef6 | #6b3fa0 \| #c9b6f2 | #ffffff \| #1d1426 | #6b3fa0 \| #d4c4f6 | #eee7f4 \| #1a151f | =ink | #2a1f33 \| #2b2333 | #f7f3fa | #f6a5c8 | #2a1f33 \| #2b2333 | #f6a5c8 | #c9b6f2 |
| encre | #fafaf8 \| #111111 | #111111 \| #f2f2ef | #111111 \| #f2f2ef | #fafaf8 \| #111111 | =ink | #f1f1ee \| #161616 | =ink | #111111 \| #222222 | #fafaf8 | #ffffff | #111111 \| #f2f2ef | #fafaf8 \| #111111 | #f28cbb |
| ocean | #f4f7fb \| #0c1420 | #0f1b2a \| #e9f0f8 | #2459c7 \| #7aa7ff | #ffffff \| #0b1a33 | #2459c7 \| #9cbcff | #0f2a4a \| #0a192b | #eaf1fb | #0f2a4a \| #14283f | #eaf1fb | #6ee7d8 | #0f2a4a | #eaf1fb | #6ee7d8 |
| ambre | #fbf6ee \| #1a140f | #22180f \| #f7eee3 | #c2532a \| #f0956a | #ffffff \| #2a130a | #a8461f \| #f4a985 | #f4eadb \| #201812 | =ink | #2a1d14 \| #2e231a | #fbf6ee | #f0956a | #c8582c \| #2e231a | #fff7ee \| #f0956a | #22180f \| #f7d38a |
| lime | #f6f7f0 \| #0b0c0a | #141611 \| #eef2e4 | #c6ff3d | #141611 | #4a7300 \| #c6ff3d | #141611 \| #0b0c0a | #eef2e4 | #141611 | #eef2e4 | #c6ff3d | #141611 | #c6ff3d | #ffffff |
| contraste | #ffffff \| #000000 | #000000 \| #ffffff | #0040d6 \| #ffd84a | #ffffff \| #000000 | #0040d6 \| #ffd84a | =bg | =ink | #000000 | #ffffff | #ffd84a | #000000 \| #ffd84a | #ffd84a \| #000000 | #ffffff \| #000000 |

Contraste renforce aussi les traits : `--line-k: 40%`, `--line-strong-k: 70%`, `--ink-2-k: 88%`.

Tokens dérivés (calculés une seule fois pour toutes les palettes, formules du pilote) : `surface`, `raised`, `ink-2`, `ink-3`, `line`, `line-strong`, `accent-wash`, `side-ink-2`, `side-active`, `shadow-c`. Les couleurs de statut (`warning`, `error`, `moss`, et leurs `-wash`) restent indépendantes de la palette.

Seuils de contraste WCAG, à respecter dans les deux modes : ink/bg ≥ 7 ; accent-text/bg ≥ 4.5 ; on-accent/accent ≥ 4.5 ; pill-ink/pill ≥ 4.5 ; side-ink/side ≥ 4.5 ; pill-accent/pill ≥ 3 ; ink-2/raised ≥ 4.5.

Palette par défaut : **rose**. Une valeur inconnue en stockage retombe sur rose.

## 3. Écran Apparence (nouvelle section de la barre latérale)

Il se place entre Post-traitement et Avancé, précédé d'un séparateur, avec une icône pinceau.
- **Thème** : grille de 8 cartes (4 colonnes, 2 sous 900 px). Chaque carte porte un mini-aperçu rendu dans sa propre palette. La carte active a un contour `accent-text` et une coche. Navigation au clavier par les flèches.
- **Mode** : Système / Clair / Sombre (le réglage `theme` existant, retiré de À propos).
- **Titres** : Éditorial (Instrument Serif 400) / Moderne (Figtree 700, -0.03em).
- **Densité** : Confortable (padding vertical des rangées 14 px) / Compact (9 px).
- **Pastille de dictée** : Pastille / Onde / Live. Pastille = `overlay_style: minimal` avec libellé ; Onde = `minimal` sans libellé ; Live = `live`. « Masquée » (`none`) reste disponible, dans Avancé.

Chaque changement s'applique immédiatement à la fenêtre principale **et** à l'overlay, puis est persisté.

## 4. Polish et mouvement (fin, jamais bruyant)

- Changement de palette ou de mode : fondu enchaîné View Transition de 450 ms, `cubic-bezier(.22,1,.36,1)`. Sans support de View Transition, le changement est instantané.
- Changement de section : `--animate-rise` existant (260 ms).
- Interrupteurs : glissement du bouton en 250 ms ease-glide ; hover de la navigation en 150 ms.
- Historique : les actions (copier, réécouter, garder) apparaissent au survol et au focus clavier.
- Cartes de palette : soulèvement de 2 px au survol.
- Overlay : pulsation douce du point d'enregistrement (1,6 s) ; libellé « Transcription… » qui respire en opacité, sans dégradé ; les barres restent pilotées par le **niveau audio réel**.
- `prefers-reduced-motion: reduce` coupe toutes les animations et transitions.
- Éléments natifs thémés : sélection de texte, anneau de focus (2 px `accent-text`), barres de défilement.
- Polices embarquées en woff2 (OFL), aucune requête réseau : Figtree (variable, 400–800), Instrument Serif (regular + italic).

## 5. Hors périmètre

macOS et Linux doivent compiler et garder leur comportement, mais ne sont pas validés visuellement. Pas de nouveau thème des sons. Pas de refonte de l'onboarding au-delà du changement de nom et de l'héritage des tokens.
