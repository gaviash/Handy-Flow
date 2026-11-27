# Liste des plans, version 3 (15 plans rédigés le 2026-11-25)

Source des idées : `../PLAN_LOTS.md` (diagnostic T, D, M, S, C ; lots K, A ; idées W de Willow ; H de tes données) et les idées V de Wispr (plus bas).
Ordre d'implémentation. Les 15 plans sont rédigés et relus par Terra ; aucun n'est codé. Estimations : développement, hors essais réels.
**Weaver** = nom provisoire de la fonction de rédaction vocale.

**Mise à jour du 26/11** : passe 2 Willow + Wispr (`willow/00-PASSE-2.md`). Chaque plan a une section **J** (corrections du plan, améliorations retenues, questions). Plan 5 à revoir avant de coder. Le plan 0 passe avant le plan 1.

| N° | Plan | Idées regroupées | Dépend de | Estimation |
|---|---|---|---|---|
| 0 | **Corrections de bugs** (`00-CORRECTIONS-BUGS.md`) — **fait le 27/11, voir `00-SUIVI.md`** | 24 corrections en 5 lots : données, confidentialité, pannes muettes, mots, réactivité | — | 20–22 h |
| 1 | Transcription propre (plan rédigé) | K1, T1 à T6 | — | 3–4 h |
| 2 | Capture dès l'appui et icône (plan rédigé) | K5, A1, C1 (boutons), C3 à C5, C7, C8, W1 à W3, W40, V9 | 1 | 2 j |
| 3 | Suivi des corrections (plan rédigé) | K0 (reste), K2, D1 à D8, W4, W5, H7, V10 | 2 | 1,5–2 j |
| 4 | Mots et dictionnaire (plan rédigé ; section K du 27/11 : clef-omni, vocabulaire hybride, apprentissage sans correction, modèles de décision) | K3, M1 à M5, W7, W8, W33, H8, V8 | 3 | 3–3,5 j |
| 5 | Style appris et apprentissage depuis l'historique (plan rédigé) | K4, S1 à S6, W6, H5, H4, W23 | 3, 4 | 2 j |
| 6 | Préférences et À propos de moi (plan rédigé) | A4, W9, W10, W39, V21 (H3 abandonné) | 5 | 1 j |
| 7 | Retouche vocale (plan rédigé) | H1, V6 | 3, 5 | 1 j |
| 8 | Vitesse et quota Groq (plan rédigé) | A7 réduite, W24, W18 (durée), H6 (W27, V17, V18 abandonnés) | 2, 4, 6 | 1,5 j |
| 9 | Lecture des conversations (plan rédigé) | A2 (conversations), C1 (contenu), C2, W11 (V7 reporté) | 2, 8 | 2 j |
| 10 | Contexte de développement (plan rédigé) | A2 (éditeurs), C6, W12, H2, V19 (V11 abandonné) | 2, 8 | 1,5 j |
| 11 | Weaver : l'essentiel (plan rédigé) | A3, W13 (brouillon, Tab, « déjà collé »), W28 à W31 | 6, 8, 9 | 3 j |
| 12 | Weaver : fil et panneau complet (plan rédigé) | W13 (propositions, résumé), W32 | 11 | 1,5 j |
| 13 | Audio et micro (plan rédigé) | W15, W16, W34, W38, W26 (essai micro), V1, V2 | 8 | 1–1,5 j |
| 14 | Gestes, insertion et reprise (plan rédigé) | A5, W14, W25, W41, W42, V3 (W17, W35 et W18 second essai écartés) | 8, 13 | 1 j |
| 15 | Confort et interface (plan rédigé) | A6, W20, W22, W26 (recherche), W36 (W19, W21, W37, V12 écartés) | 11 | 1,5 j |

## Idées V : Wispr, absentes de la version 1

Lu en entier : `UX_BEHAVIORS`, `PERFORMANCE_UX`, `ARCHITECTURE`, `HANDOFF`. Lu en grande partie : `IMPLEMENTATION_SPEC`, `RESEARCH_ADDENDUM`. Non lus : registre de preuves, tests, `WISPR_RESEARCH`, `OPEN_QUESTIONS`, `GOLDEN`, `CODEX_REVIEW`.

| # | Idée | Source |
|---|---|---|
| V1 | Alerte « aucun son » pendant la dictée, à 5 s puis 15 s | PERFORMANCE §2 |
| V2 | Fin de parole : jusqu'à 300 ms après le relâchement, coupée sur silence (Handy : 120 ms fixes) | PERFORMANCE §2 |
| V3 | Surveiller le raccourci global et le réarmer si Windows le désactive | PERFORMANCE §4 |
| V6 | Polish sans sélection (réduit : dernière dictée seulement, avant/après dans l'historique) | UX §7 |
| V7 | Ton déduit des messages déjà envoyés dans la conversation | UX §6 |
| V8 | Mots favoris envoyés en premier ; noms de l'écran repris dans le texte final appris | SPEC §9.3, 9.4 |
| V9 | Suivre le changement de focus par événement | SPEC §3.1 |
| V10 | Corrections suivies par événement, avec catégorie de touche (envoi, effacement, annulation), jamais les caractères | SPEC §8 |
| V11 | Terminal et IA : peu ou pas de reformulation, termes techniques intacts | SPEC §6 |
| V12 | Listes collées en vrai format quand l'application l'accepte | SPEC §6, §7 |
| V17 | Deux fournisseurs en course, le premier valide gagne (coûte du quota : option) | PERFORMANCE §1 |
| V18 | Longues dictées découpées (28 s), fin rejouée si la connexion coupe | PERFORMANCE §1 |
| V19 | Reconnaître l'agent dans le terminal (Claude Code, Codex) | ARCHITECTURE §3 |
| V21 | Exemples de tes textes fournis par toi | ARCHITECTURE §5 |

Numéros absents (V4, V5, V13 à V16, V20) : déjà dans Handy, déjà dans une idée W, de niche, ou **refusés par toi le 15/11** : bloc-notes de secours, vérification après collage, pas de collage si la fenêtre a changé. W30 ne concerne que Weaver.

## Changements depuis la version 1
- Plan 3 trop large : le bouton « apprendre de ça » et le signalement passent en 5, après les mots et le style qu'ils alimentent.
- Plan 5 coupé : style appris (5), préférences (6).
- Plan 9 coupé en trois : retouche (7, avancée), Weaver essentiel (11), Weaver complet (12).
- Plan 10 coupé : audio (13), gestes et reprise (14).
- C1 était seulement en 7 alors que les boutons polluent déjà la dictée : boutons en 2, contenu en 9. C2 va en 9, où le texte sert.
- W24 (identifiant et échéance par dictée) avancé en 8 : l'envoi pendant la parole en a besoin.
- H3 (mode consigne pour IA) passe en 6 : c'est une préférence. Le plan 10 ne dépend plus du 9.
- H5 reste après le tri des anciens textes faux (D3, D4, traités en 3).
