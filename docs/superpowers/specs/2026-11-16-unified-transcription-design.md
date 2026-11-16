# Modèles : transcription et secours local

Proposition du 2026-11-16, à valider avant implémentation.

## Intention et périmètre

La page Modèles rassemble uniquement la transcription : explication, choix entre API et ordinateur, configuration API, modèle local et secours. La reformulation conserve sa page actuelle, ses fournisseurs, ses modèles et son secours. Général conserve les raccourcis, les langues parlées et l'audio.

## Approche retenue dans la conversation

Deux approches ont été proposées : centraliser transcription et reformulation, ou centraliser seulement la transcription. L'utilisateur a choisi la seconde pour garder chaque fonction dans une page distincte. Réutiliser les composants, le catalogue et les réglages existants ; aucune nouvelle dépendance ni modification du routage Rust prévue.

## Page Modèles proposée

1. Titre « Modèles de transcription », explication courte « Transforme votre voix en texte. La mise au propre se règle dans Reformulation. » et lien vers cette page.
2. Résumé de la configuration : mode principal, fournisseur et modèle ; modèle local principal ou de secours. Distinguer configuration complète et fonctionnement réellement vérifié : ne pas annoncer une API opérationnelle sur la seule présence d'une clé.
3. Choix explicite « Sur cet ordinateur » / « Par API ». Le mode local utilise le modèle sélectionné ; le mode API propose fournisseur, modèle et clé. Réglages secondaires repliés : délai avant secours et URL du serveur personnalisé.
4. Bloc local toujours identifiable : « Modèle de transcription » en mode local, « Modèle local de secours » en mode API. Nom sélectionné et accès au catalogue existant pour choisir, télécharger ou supprimer. Les boutons de sélection précisent leur effet selon le mode ; sélectionner un secours ne désactive pas l'API.
5. Explication visible du secours : utilisé si l'API échoue, ne renvoie pas de texte exploitable ou dépasse le délai ; les gardes de confidentialité existantes peuvent aussi imposer le local. Si aucun modèle local n'est disponible, proposer son téléchargement et signaler l'absence de secours.

L'absence de clé ou de modèle API indique une configuration incomplète et explique le recours local actuel. Un serveur personnalisé peut fonctionner sans clé. Une suppression ou un téléchargement interrompu actualise le résumé, sans badge « prêt » trompeur.

## Cohérence des autres écrans

- Retirer de Général le formulaire API et déplacer les options propres au modèle dans Modèles, en distinguant leurs capacités locales/API. Les langues parlées restent dans Général.
- Remplacer le sélecteur local du pied de page par un résumé du mode configuré et un lien vers Modèles. Conserver les informations de téléchargement et le statut des tâches, sans présenter le secours comme moteur principal.
- Garder la reformulation dans sa page actuelle. Aucun de ses réglages n'est déplacé vers Modèles.
- Les clés et l'URL personnalisée sont actuellement partagées entre usages : le mentionner près des champs concernés, sans dupliquer ni migrer les secrets. Changer un modèle de transcription ne change pas le modèle de reformulation.
- Présenter l'organisation et les fonctions déjà existantes ; pas de refonte de l'onboarding ni d'extension du traitement API à d'autres usages dans cette tâche.

## Contraintes et validation

Conserver les réglages existants, les protections de confidentialité, le catalogue, les téléchargements et le verrouillage pendant les tâches. Garder les composants et le style visuel du projet. Traduire les nouveaux libellés (français/anglais, clés présentes dans les autres langues). Navigation clavier, états de focus et erreurs lisibles.

Vérifier les modes local/API, configuration incomplète, serveur sans clé, secours absent/supprimé et changement du secours sans désactivation de l'API. Contrôler visuellement la page et son défilement dans la fenêtre de l'application. Aucun test qui prend le clavier, le focus ou le micro de l'utilisateur sans demande explicite.
