L’encombrement est étayé par le code : **Avancé expose jusqu’à 21 réglages sous Windows**, et **Post-traitement jusqu’à 48 champs avec les trois transformations par défaut, avant les snippets**. Les sujets « reconnaissance », « amélioration », « vocabulaire » et « confidentialité » sont dispersés. Un défaut fonctionnel est confirmé : **le choix « 30 jours » est rejeté à la sauvegarde, sans retour visible**.

Audit statique de la branche `feat/handy-flow-brand`, sans modification. Les références ont été recherchées avec `rg -n`. Les constats de rendu non mesurés sont explicitement distingués des défauts prouvés.

Conventions utilisées pour raccourcir les tableaux :

| Préfixe | Racine |
|---|---|
| `S/` | [src/components/settings](C:/Users/gavri/Desktop/handy_remastered/Handy/src/components/settings) |
| `R/` | [src-tauri/src](C:/Users/gavri/Desktop/handy_remastered/Handy/src-tauri/src) |
| `O/` | [src/components/onboarding](C:/Users/gavri/Desktop/handy_remastered/Handy/src/components/onboarding) |
| `U/` | [src/components/ui](C:/Users/gavri/Desktop/handy_remastered/Handy/src/components/ui) |
| `FR` | [translation.json](C:/Users/gavri/Desktop/handy_remastered/Handy/src/i18n/locales/fr/translation.json) |

## 1. Inventaire des surfaces

Les nombres représentent des **unités de réglage**, pas chaque option d’un menu ni chaque icône d’aide. Les actions secondaires sont indiquées séparément. Pour les listes variables : `T` = transformations, `S` = snippets, `N` = entrées.

### Navigation et pages de réglages

| Surface | Sections et contrôles rendus | Total et conditions | Preuves |
|---|---|---|---|
| Sidebar | Général, Historique, Modèles, Post-traitement, Apparence, Avancé, Débogage, À propos | **6 à 8 entrées**. Post-traitement si `post_process_enabled` ; Debug si `debug_mode`. Icônes **et textes déjà présents**. | `src/components/Sidebar.tsx:48`, `:68`, `:87`, `:153` |
| Général | Compteur du jour ; raccourci dictée ; activation/annulation ; langues ; audio | **9 réglages Windows courants**, +1 canal si micro multicanal, +1 écriture chinoise. **11 maximum**. Annulation cachée sous Linux. | `S/general/GeneralSettings.tsx:24`, `:30`, `:32`, `:37`, `:40`, `:46` |
| Avancé | Application ; sortie ; transcription ; historique ; expérimental | **15 réglages Windows**, **21 avec expérimental**, +1 lien Apparence. Maximum multiplateforme : 23 intentions de réglage, avec gardes internes. | `S/advanced/AdvancedSettings.tsx:38`, `:55`, `:62`, `:70`, `:78` |
| Apparence | Palette ; thème ; typographie ; densité ; ton, style et position de la pastille | **7 réglages**. Le style écrit deux champs. Quatre styles : pastille, onde, live, masqué. | `S/appearance/AppearanceSettings.tsx:38`, `:47`, `:53`, `:72`, `:94`, `:109`, `:135` |
| Modèles | Mode local/API ; configuration API ; modèle local ou secours ; catalogue | Jusqu’à **9 contrôles de configuration** en mode API, en comptant le modèle local de secours ; catalogue variable. Langue/traduction selon capacités. | `S/models/CloudAsrSettings.tsx:75`, `:117`, `:142`, `:179`, `:188`, `:241` ; `S/models/ModelsSettings.tsx:182`, `:210` |
| Post-traitement | Raccourci ; API ; transformations ; styles/snippets ; contexte ; apprentissage ; prompts | **7 groupes**, **`42 + 2T + 2S` champs maximum** ; soit **`48 + 2S`** avec trois transformations. Certains champs sont conditionnels ou en lecture seule. | `S/post-processing/PostProcessingSettings.tsx:513`, `:521`, `:526`, `:530`, `:535`, `:539`, `:543` ; `R/settings.rs:1365` |
| Historique | Import de fichiers ; dossier ; statistiques ; entrées paginées | **2 actions globales** ; **5 contrôles par entrée**, jusqu’à 3 supplémentaires selon contenu/audio. Pages de 30. | `S/history/HistorySettings.tsx:166`, `:388`, `:393`, `:565`, `:621`, `:661` |
| Debug | Journalisation ; previews ; maintenance ; temporisations ; capture audio ; diagnostic clavier | Windows : **11 réglages + 5 lignes d’outils**, soit **16 lignes interactives**. Maximum macOS portable : 12 + 6. | `S/debug/DebugSettings.tsx:37`, `:44`, `:50`, `:56`, `:61`, `:76` |
| À propos | Identité ; langue ; version ; nouveautés ; source ; dossiers ; remerciements | **2 réglages**, **3 actions**, **2 informations**. | `S/about/AboutSettings.tsx:43`, `:50`, `:53`, `:63`, `:69`, `:70`, `:73` |

### Détail de la densité du post-traitement

| Groupe | Champs | Conditions et actions supplémentaires | Preuves |
|---|---:|---|---|
| Raccourci | 1 | Capture et réinitialisation | `S/post-processing/PostProcessingSettings.tsx:514` |
| API | 7 max. | Fournisseur ; format custom ; URL éditable ; clé/modèle hors Apple ; secours ; délai. Rafraîchissement des modèles, spinner, alerte Apple. | même fichier `:45`, `:61`, `:95`, `:121`, `:143`, `:173`, `:192` ; `S/post-processing/PostProcessTimeout.tsx:12` |
| Transformations | `5 + 2T` | Polish, instruction vocale, trois raccourcis ; nom/instruction par transformation. Enregistrement au blur. | `S/post-processing/TransformSettings.tsx:21`, `:26`, `:31`, `:46`, `:55`, `:65` |
| Styles | 4 | Email, travail, personnel, autre ; cinq tons proposés | `S/post-processing/StyleSettings.tsx:8`, `:9`, `:29` |
| Snippets | `2S` | Déclencheur/texte ; ajouter ; supprimer ; sauvegarde au blur | `S/post-processing/SnippetSettings.tsx:30`, `:38`, `:46`, `:51`, `:64` |
| Contexte | 14 | Nom, lecture, envoi, VS Code, deux exclusions, huit sources ; rapports et actualisation | `S/post-processing/ContextSettings.tsx:79`, `:81`, `:90`, `:99`, `:108`, `:115`, `:121` ; `S/post-processing/ContextReports.tsx:198` |
| Apprentissage et aide ASR | 8 | Apprentissage/son ; six options ASR ; mots/règles, suppression, export, effacement, expressions repliables | `S/post-processing/LearningSettings.tsx:40`, `:49`, `:98`, `:101`, `:107` ; `S/post-processing/AsrContextSettings.tsx:92` |
| Prompts | 3 | Sélection, nom, instructions ; nouveau/créer/annuler/modifier/supprimer. Prompt maison protégé ; dernier prompt non supprimable. | `S/post-processing/PostProcessingSettings.tsx:339`, `:356`, `:373`, `:389`, `:410`, `:420`, `:475` |

### Onboarding

**Six étapes logiques, cinq ou six affichées** : Welcome → Permissions → Model → Demo → Try → Done. Permissions est sautée si déjà accordées. Preuves : `O/onboardingStore.ts:17`, `O/OnboardingFlow.tsx:129`.

| Écran | Inventaire et nombre | Conditions | Preuves |
|---|---|---|---|
| Welcome | Menu langue UI ; recherche langues parlées ; ajouts/retraits variables ; Continuer ; trois choix d’écriture chinoise | Continuer bloqué sans langue ; écriture seulement si chinois ; avertissement si langue mal couverte | `O/screens/Welcome.tsx:31`, `:77`, `:84`, `:107`, `:126`, `:143` |
| Permissions | Une ligne microphone ; une ligne accessibilité macOS ; Retour/Continuer ; Relancer conditionnel | Une demande active à la fois ; progression bloquée avant permissions | `O/screens/Permissions.tsx:66`, `:112`, `:165`, `:183`, `:203`, `:219` |
| Model — recommandations | Jusqu’à trois recommandations, + modèle précédemment choisi ; Voir tous ; Retour ; Télécharger/Continuer | Progression, panne, taille, capacités, notes et vitesse ; changement de modèle pendant téléchargement expliqué | `O/screens/Model.tsx:95`, `:104`, `:198`, `:476`, `:500`, `:546` |
| Model — catalogue | Recherche ; deux filtres ; N sélections ; Retour ; Télécharger/Continuer | Sections selon compatibilité ; Échap et flèches gérés dans la table | `O/screens/Model.tsx:569`, `:622` ; `src/components/shared/modelTable/ModelTable.tsx:122`, `:213`, `:241` |
| Demo | Modifier/annuler raccourci ; Retour ; Essayer ou Réessayer | Action désactivée tant que modèle indisponible ; adaptation mouvement réduit | `O/screens/Demo.tsx:384`, `:494`, `:505`, `:513`, `:555` |
| Try | Zone texte ; raccourci ; mascotte ; Retour ; Passer/Terminer | Micro proposé si silence ; modèle plus rapide si lenteur ; aide Linux ; fin selon exercices réussis | `O/screens/TryIt.tsx:225`, `:361`, `:390`, `:438`, `:490`, `:600`, `:717` |
| Done | **4 réglages** : langues, modèle, raccourci, autostart ; Retour/Terminer | Édition langues/modèle revient à Done | `O/TrayPanel.tsx:111`, `:125`, `:146`, `:185` ; `O/OnboardingFlow.tsx:246` |
| Permissions révoquées | Réutilisation du parcours de permissions | Utilisateur déjà configuré | `O/StandaloneScreens.tsx:14` ; `src/App.tsx:338` |
| Migration | N modèles ; cases conditionnelles ; télécharger/continuer ; autre modèle ; réessayer par panne | Cases si plusieurs modèles et phase de choix ; progression détaillée ; modèles non cochés supprimés en fin de migration | `O/ModelMigration.tsx:233`, `:257`, `:285`, `:326`, `:361`, `:408` |
| Remplacement | Réutilisation de Model | Fin après téléchargement/sélection | `O/StandaloneScreens.tsx:23` ; `src/App.tsx:352` |

### Overlay, footer, notices et tray

| Surface | Inventaire | Conditions / total | Preuves |
|---|---|---|---|
| Overlay compact/direct | Annuler ; fermer notice ; aperçu confirmé/provisoire ; minuterie en direct | Deux présentations ; états enregistrement, chargement, transcription, traitement, collé, incident | `src/overlay/RecordingOverlay.tsx:307`, `:325`, `:342`, `:375`, `:399`, `:432`, `:476`, `:501` |
| Footer | Moteur/modèle → Modèles ; tâche → Historique ; mises à jour ; incidents | **4 points d’entrée possibles** ; tâches/incidents conditionnels ; version informative | `src/components/footer/Footer.tsx:59`, `:69`, `:76`, `:88`, `:91` |
| Notices | Titre, explication/détail, fermer ; action éventuelle ; liste globale | Toast 8 secondes ; pause au survol ; liste fermable par Échap/clic extérieur ; région lecteur d’écran | `src/components/notices/NoticePanel.tsx:13`, `:42`, `:59`, `:80`, `:90`, `:166` |
| Tray au repos | Copier dernière transcription ; décharger modèle ; paramètres ; mises à jour ; quitter ; sous-menu modèles ; langues conditionnelles | **5 commandes + 1 ou 2 sous-menus** ; déchargement actif seulement si modèle chargé | `R/tray.rs:684`, `:691`, `:697`, `:703`, `:709`, `:732`, `:743` |
| Tray en activité | Copier ; annuler ; paramètres ; mises à jour ; quitter | **5 commandes** ; sélection/déchargement remplacés pendant activité | `R/tray.rs:769`, `:780`, `:788`, `:793` |
| Tray complémentaire | Version ; incident ; Secure Input ; langues | Jusqu’à deux entrées d’alerte ; `(Dev)` en compilation debug ; langues absentes si non pertinentes | `R/tray.rs:417`, `:497`, `:506`, `:623` ; `FR:4`, `:5`, `:6`, `:13` |

## 2. Réglages : effet réel et rangement proposé

Cette matrice couvre aussi le livrable de rangement. **Chaque contrôle nommé conserve ses actions secondaires dans la même sous-page** : capture/reset, ajouter/retirer, rafraîchir, tester, confirmer/annuler.

Présentation : **Visible** = écran principal ; **Carte** = résumé de valeur ouvrant une sous-page ; **Expert** = replié ; **Debug** = diagnostic explicite.

### Général, audio et application

| Contrôle / champ | Lecture réelle | Destination proposée |
|---|---|---|
| Raccourci dictée `bindings.transcribe` | `S/general/GeneralSettings.tsx:32` → `R/shortcut/mod.rs:261`, `:316` | **Général · Visible** — action principale |
| Activation `shortcut_activation` | `S/ShortcutActivation.tsx:53` → `R/shortcut/handler.rs:44` | **Général · Visible** — expliquer maintenir/appuyer |
| Annulation `bindings.cancel` | `S/general/GeneralSettings.tsx:37` → `R/shortcut/mod.rs:124` | **Général · Carte Raccourcis** ; garde Linux conservée |
| Langues parlées `spoken_languages` | `S/PreferredLanguages.tsx:215` → `R/language_detection/mod.rs:45`, `:60` | **Général · Carte Langues** — valeur actuelle immédiatement lisible |
| Écriture `chinese_script` | `S/ChineseScript.tsx:52` → `R/managers/transcription.rs:1979` | **Général · même carte**, condition chinoise conservée |
| Micro `selected_microphone` | `S/MicrophoneSelector.tsx:36` → `R/managers/audio.rs:586` | **Général · Carte Microphone** |
| Canal `selected_channel` | `S/ChannelSelector.tsx:46`, `:52` → `R/managers/audio.rs:545` | **Général · Microphone · Expert**, seulement multicanal |
| Couper sortie `mute_while_recording` | `S/MuteWhileRecording.tsx:21` → `R/managers/audio.rs:765` | **Général · Carte Audio** |
| Retour sonore `audio_feedback` | `S/AudioFeedback.tsx:21` → `R/audio_feedback.rs:51`, `:61` | **Général · Carte Audio** |
| Sortie `selected_output_device` | `S/OutputDeviceSelector.tsx:35` → `R/audio_feedback.rs:94` | **Général · Audio · Expert**, condition feedback conservée |
| Volume `audio_feedback_volume` | `S/VolumeSlider.tsx:17` → `R/audio_feedback.rs:93` | **Général · Carte Audio**, désactivé sans feedback |
| Démarrage caché `start_hidden` | `S/StartHidden.tsx:21` → `R/lib.rs:176` | **Général · Carte Démarrage** |
| Autostart `autostart_enabled` | `S/AutostartToggle.tsx:21` → `R/lib.rs:421` | **Général · Carte Démarrage** |
| Tray `show_tray_icon` | `S/ShowTrayIcon.tsx:21` → `R/tray.rs:833`, `R/lib.rs:1223` | **Général · Carte Comportement** — influe sur fermeture |
| Langue UI `app_language` | `S/AppLanguageSelector.tsx:40` → `src/i18n/index.ts:106`, `R/tray.rs:399` | **Général · Carte Langues**, quitter À propos |
| Nouveautés `show_whats_new_on_update` | `S/ShowWhatsNewOnUpdate.tsx:23` → `src/components/whats-new/WhatsNewGate.tsx:15` | **À propos · Carte Mises à jour** |
| Vérifier mises à jour `update_checks_enabled` | `S/UpdateChecksToggle.tsx:23` → `src/components/update-checker/UpdateChecker.tsx:30` | **À propos · Carte Mises à jour**, quitter Debug ; verrou environnement conservé |

### Apparence et sortie de texte

| Contrôle / champ | Lecture réelle | Destination proposée |
|---|---|---|
| Palette `palette` | `S/appearance/AppearanceSettings.tsx:42` → `src/lib/utils/appearance.ts:73` | **Général · Carte Apparence** |
| Thème `theme` | `S/ThemeSelector.tsx:29` → `src/lib/utils/appearance.ts:71` | **Général · Carte Apparence** |
| Titres `heading_style` | `S/appearance/AppearanceSettings.tsx:62` → `src/lib/utils/appearance.ts:74` | **Général · Apparence · Expert** |
| Densité `ui_density` | même fichier `:81` → `src/lib/utils/appearance.ts:75` | **Général · Carte Apparence** |
| Ton pastille `pill_tone` | même fichier `:101` → `src/lib/utils/appearance.ts:76` | **Général · Carte Pastille** |
| Style `overlay_style`, `overlay_show_label` | même fichier `:121`, `:130` → `R/overlay.rs:531`, `src/lib/utils/appearance.ts:77` | **Général · Carte Pastille** — conserver choix masqué |
| Position `overlay_position` | `S/ShowOverlay.tsx:45` → `R/overlay.rs:282` | **Général · même carte** |
| Méthode `paste_method` | `S/PasteMethod.tsx:97` → `R/clipboard.rs:1013`, `:1039` | **Avancé · Carte Insertion du texte** |
| Script `external_script_path` | `S/PasteMethod.tsx:101` → `R/clipboard.rs:1085` | **Avancé · Insertion · Expert**, Linux seulement |
| Outil `typing_tool` | `S/TypingTool.tsx:41`, `:47`, `:75` → `R/clipboard.rs:1039` | **Avancé · Insertion · Expert**, Linux/direct seulement |
| Presse-papiers `clipboard_handling` | `S/ClipboardHandling.tsx:43` → `R/clipboard.rs:1055`, `:1104` | **Avancé · Carte Insertion** |
| Soumission `auto_submit`, `auto_submit_key` | `S/AutoSubmit.tsx:54`, `:58`, `:60` → `R/clipboard.rs:1093` | **Général · Insertion · Expert** — action automatique importante |
| Espace final `append_trailing_space` | `S/AppendTrailingSpace.tsx:21` → `R/clipboard.rs:1018` | **Général · Carte Insertion** |

### Reconnaissance et modèles

| Contrôle / champ | Lecture réelle | Destination proposée |
|---|---|---|
| Mode local/API et fournisseur `cloud_asr.provider_id` | `S/models/CloudAsrSettings.tsx:75`, `:117` → `R/cloud_asr.rs:85`, `R/actions.rs:1188` | **Modèles · Visible** — même valeur pilotée par deux contrôles coordonnés |
| Modèle API `cloud_asr.models[id]` | même fichier `:142` → `R/cloud_asr.rs:85` | **Modèles · Carte Fournisseur** |
| Clé API partagée | même fichier `:38`, `:162` → `R/cloud_asr.rs:92` | **Modèles · Carte Fournisseur** — signaler partage avec Amélioration |
| Délai API `cloud_asr.timeout_secs` | même fichier `:179` → `R/actions.rs:1193` | **Modèles · Fournisseur · Expert** |
| URL custom partagée | même fichier `:204`, `:211` → `R/cloud_asr.rs:177` | **Modèles · Fournisseur · Expert** |
| Modèle local `selected_model` | `S/models/modelEntries.ts:177` → `src/stores/modelStore.ts:141` | **Modèles · Visible**, préciser principal/secours |
| Langue `selected_language` | `S/LanguageSelector.tsx:172` → `R/managers/transcription.rs:1792` | **Modèles · Carte Options du moteur**, selon capacités |
| Traduction `translate_to_english` | `S/TranslateToEnglish.tsx:21` → `R/managers/transcription.rs:1550`, `:2076` | **Modèles · même carte**, selon capacités |
| Déchargement `model_unload_timeout` | `S/ModelUnloadTimeout.tsx:63` → `R/managers/transcription.rs:337`, `:528` | **Modèles · Expert** — gestion mémoire |
| VAD `vad_enabled` | `S/VoiceActivityDetection.tsx:22` → `R/actions.rs:907` | **Avancé · Expert Capture** |
| Découpage `long_form_chunking` | `S/LongFormChunking.tsx:22` → `R/managers/transcription.rs:907`, `:1371` | **Modèles · Expert**, expliciter dépendance au modèle |
| Retrait hésitations `filler_word_removal_enabled` | `S/FillerWordRemoval.tsx:21` → `R/managers/transcription.rs:1961`, `:1997` | **Amélioration · Carte Nettoyage** |
| Accélérateur/GPU | `S/AccelerationSelector.tsx:97`, `:98` → `R/managers/transcription.rs:624` | **Modèles · Expert** — effet réel, pas simple diagnostic |
| Fermeture différée `lazy_stream_close` | `S/LazyStreamClose.tsx:21` → `R/managers/audio.rs:1291`, `:1345` | **Avancé · Expert Capture** |
| AirPods `airpods_mode` | `S/AirPodsMode.tsx:20`, `:27` → `R/managers/audio.rs:546` | **Avancé · Expert Capture**, macOS seulement |
| Backend VAD `vad_backend` | `S/VadBackendSelector.tsx:46` → `R/managers/audio.rs:712` | **Avancé · Expert Capture** |
| Détection `language_detection` | `S/LanguageDetection.tsx:34` → `R/language_detection/mod.rs:47`, `:62` | **Modèles · Expert Langue** ; téléchargement/annulation conservés |
| Recherche, deux filtres, plus/moins, légende | `S/models/ModelsSettings.tsx:60`, `:68`, `:242` ; `src/components/shared/modelTable/ModelTable.tsx:241`, `:279` | **Modèles · Carte Catalogue** |
| Télécharger/sélectionner/annuler/supprimer | `S/models/ModelRow.tsx:101`, `:117`, `:133`, `:151`, `:197` | **Modèles · Catalogue**, confirmation suppression conservée |
| Rescanner ; nettoyer modèles retirés | `S/models/ModelsSettings.tsx:32` ; `S/models/RetiredModelsNotice.tsx:26`, `:35` | **Modèles · Catalogue · Expert** |
| Liens langues/reformulation | `S/models/ModelsSettings.tsx:145`, `:187` | Conserver vers **Général / Amélioration**, intitulés cohérents |

### Amélioration

| Contrôle / champ | Lecture réelle | Destination proposée |
|---|---|---|
| Activation `post_process_enabled` | `S/PostProcessingToggle.tsx:21` → `R/shortcut/mod.rs:1307`, `:1318` ; sidebar `src/components/Sidebar.tsx:68` | **Amélioration · Visible**, sortir du groupe expérimental |
| Raccourci `transcribe_with_post_process` | `S/post-processing/PostProcessingSettings.tsx:514` → `R/actions.rs:1748` | **Général · Carte Raccourcis**, résumé dans Amélioration |
| Fournisseur LLM | même fichier `:45` → `R/actions.rs:454` | **Amélioration · Carte Fournisseur** |
| Format OpenAI/Anthropic | même fichier `:71` → `R/llm_client.rs:212`, `:581` | **Amélioration · Fournisseur · Expert** |
| URL | même fichier `:104` → `R/llm_client.rs:401`, `:645` | **Amélioration · Fournisseur · Expert** |
| Clé | même fichier `:129` → `R/actions.rs:510` | **Amélioration · Carte Fournisseur**, partage explicite |
| Modèle libre/liste ; rafraîchir | même fichier `:156`, `:173` → `R/actions.rs:463` | **Amélioration · Carte Fournisseur** |
| Fournisseur de secours | même fichier `:212` → `R/actions.rs:232`, `:238` | **Amélioration · Fournisseur · Expert** |
| Délai et reset | `S/post-processing/PostProcessTimeout.tsx:14`, `:15` → `R/actions.rs:777` | **Amélioration · Expert Résilience** |
| Polish ; instruction vocale ; trois raccourcis de transformation | `S/post-processing/TransformSettings.tsx:21`, `:26`, `:31` → `R/actions.rs:1755`, `:1763`, `:294` | **Amélioration · Carte Transformations** |
| Nom/instruction de chaque transformation | même fichier `:52`, `:60` → `R/actions.rs:294` | **Amélioration · même sous-page** |
| Tons email/travail/personnel/autre | `S/post-processing/StyleSettings.tsx:29` → `R/context/mod.rs:870` | **Amélioration · Carte Styles** |
| Règles de style apprises ; supprimer | `S/post-processing/LearningSettings.tsx:123`, `:146` → `R/context/mod.rs:900` | **Amélioration · Carte Styles appris** |
| Prompt sélectionné ; nom ; instructions | `S/post-processing/PostProcessingSettings.tsx:229`, `:373`, `:389` → `R/actions.rs:476`, `:485` | **Amélioration · Expert Prompt** |
| Nouveau/créer/annuler/modifier/supprimer prompt | même fichier `:356`, `:406`, `:416`, `:475`, `:483` → `R/shortcut/mod.rs:1488`, `:1497` | **Amélioration · même sous-page**, protections maison/dernier prompt conservées |

### Vocabulaire et contexte

| Contrôle / champ | Lecture réelle | Destination proposée |
|---|---|---|
| Mots personnels `custom_words` ; ajout/retrait | `S/CustomWords.tsx:39`, `:45` → `R/managers/transcription.rs:1983`, `R/context/mod.rs:882` | **Vocabulaire · Visible** — agit au-delà du post-traitement |
| Nom `user_name` | `S/post-processing/ContextSettings.tsx:52`, `:66` → `R/context/mod.rs:873` | **Vocabulaire · Visible** |
| Snippet : déclencheur/texte ; ajouter/supprimer | `S/post-processing/SnippetSettings.tsx:35`, `:43`, `:51`, `:64` → `R/actions.rs:1318` | **Vocabulaire · Carte Expressions enregistrées** |
| Apprentissage `learning_enabled` | `S/post-processing/LearningSettings.tsx:40` → `R/learning.rs:1087`, `:1167` | **Vocabulaire · Visible** |
| Son `learned_sound_enabled` | même fichier `:49` → `R/learning.rs:887` | **Vocabulaire · Expert Retours** |
| Mots appris ; retirer/actualiser/exporter/effacer | même fichier `:64`, `:88`, `:98`, `:101`, `:107` → `R/learning.rs:916`, `:922`, `:946` | **Vocabulaire · Carte Mots appris**, confirmation d’effacement conservée |
| ASR `enabled` | `S/post-processing/AsrContextSettings.tsx:92` → `R/managers/transcription.rs:1168` | **Vocabulaire · Carte Aide à la reconnaissance** |
| ASR `learned_words` | même fichier `:93` → `R/asr_context.rs:99` | **Vocabulaire · même carte** |
| ASR `expressions` ; liste repliable | même fichier `:94`, `:102` → `R/asr_context.rs:107` | **Vocabulaire · même carte** |
| ASR `screen` | même fichier `:112` → `R/asr_context.rs:112` | **Contexte · Visible** — source écran |
| ASR `max_terms` | même fichier `:113` → `R/asr_context.rs:106` | **Contexte · Expert** — budget |
| ASR `capture_wait_ms` | même fichier `:114` → `R/actions.rs:1178` | **Contexte · Expert** — latence |
| Lecture `context_enabled` | `S/post-processing/ContextSettings.tsx:81` → `R/actions.rs:1051` | **Contexte · Visible** |
| Envoi `context_send_enabled` | même fichier `:90` → `R/context/mod.rs:866` | **Contexte · Visible**, distinguer lecture et envoi |
| Exclusions apps/domaines | même fichier `:108`, `:115` → `R/actions.rs:1053`, `:301` | **Contexte · Carte Exclusions** — préciser aussi blocage du collage |
| Lecteur VS Code `vscode_screen_reader` | même fichier `:99` → `R/vscode.rs:159`, `:171` | **Contexte · Expert Windows** — dépend d’`APPDATA` |
| Sources reformulation : écran, mots appris, règles, dictées passées | `S/post-processing/ContextSettings.tsx:121` ; `ContextReports.tsx:198` → `R/context/mod.rs:838`, `:848`, `:851`, `:877` | **Contexte · Carte Sources · Expert** |
| Les quatre mêmes sources pour transformations | `S/post-processing/ContextSettings.tsx:122` → `R/actions.rs:316`, `:1346` | **Contexte · même carte**, groupe distinct |
| Rapports ; actualiser ; détails ASR/LLM | `S/post-processing/ContextReports.tsx:64`, `:85`, `:99`, `:147` → `R/context/mod.rs:776` | **Contexte · Carte Ce qui a été envoyé** ; prompt brut en **Expert** |

### Historique, diagnostic et À propos

| Contrôle / champ | Lecture réelle | Destination proposée |
|---|---|---|
| Limite `history_limit` | `S/HistoryLimit.tsx:24` → `R/managers/history.rs:563` | **Historique · Carte Conservation**, uniquement si `preserve_limit` |
| Rétention `recording_retention_period` | `S/RecordingRetentionPeriod.tsx:23` → `R/managers/history.rs:556` | **Historique · Carte Conservation**, corriger `days30` |
| Import ; dossier | `S/history/HistorySettings.tsx:388`, `:393` ; `S/history/transcribeFiles.ts:45` | **Historique · Visible / Conservation** |
| Lecture/position ; copie ; relancer/annuler ; supprimer ; favori | `S/history/HistorySettings.tsx:621`, `:634`, `:646`, `:653`, `:661` ; `U/AudioPlayer.tsx:322`, `:346` | **Historique · Visible par entrée** |
| Original/réécrit ; plus/moins | `S/history/HistorySettings.tsx:565`, `:585` | **Historique · même entrée**, état accessible à annoncer |
| Expérimental `experimental_enabled` | `S/ExperimentalToggle.tsx:21` → `S/advanced/AdvancedSettings.tsx:78` | **Debug** — retirer son rôle de porte d’entrée vers Amélioration |
| Backend clavier `keyboard_implementation` | `S/debug/KeyboardImplementationSelector.tsx:35` → `R/shortcut/mod.rs:39`, `:145` | **Debug** — choix d’implémentation |
| Niveau logs `log_level` | `S/debug/LogLevelSelector.tsx:33` → `R/commands/mod.rs:67` | **Debug** |
| Sons `sound_theme` ; test | `S/SoundPicker.tsx:57`, `:65` → `R/audio_feedback.rs:34` | **Général · Carte Audio**, quitter Debug |
| Seuil correction `word_correction_threshold` | `S/debug/WordCorrectionThreshold.tsx:18` → `R/managers/transcription.rs:1987` | **Vocabulaire · Expert** |
| Délai avant/après collage | `S/debug/DebugSettings.tsx:62`, `:66` → `R/clipboard.rs:1014`, `:1015` | **Avancé · Insertion · Expert** |
| Collage fiable `reliable_paste` | `S/debug/ReliablePaste.tsx:21`, `:28` → `R/clipboard.rs:1049` | **Avancé · Insertion · Expert**, Windows/macOS |
| Seuil maintien `hold_threshold_ms` | `S/debug/HoldThreshold.tsx:25` → `R/shortcut/handler.rs:45` | **Général · Raccourcis · Expert** |
| Tampon `extra_recording_buffer_ms` | `S/debug/RecordingBuffer.tsx:19` → `R/managers/audio.rs:1259` | **Avancé · Capture · Expert** |
| Fenêtre langue `language_detection_window_secs` | `S/debug/LanguageDetectionWindow.tsx:21` → `R/managers/transcription.rs:1347` | **Modèles · Langue · Expert** |
| Micro permanent `always_on_microphone` | `S/AlwaysOnMicrophone.tsx:21` → `R/managers/audio.rs:526` | **Avancé · Capture · Expert** |
| Micro capot fermé `clamshell_microphone` | `S/ClamshellMicrophoneSelector.tsx:48`, `:58` → `R/managers/audio.rs:574` | **Général · Microphone · Expert**, portable macOS |
| Preview nouveautés/onboarding ; lancer onboarding/migration | `S/debug/DebugSettings.tsx:38`, `:39`, `:44`, `:50` | **Debug**, conserver distinction aperçu/action réelle |
| Diagnostic clavier ; logs live, pause/copier/vider | même fichier `:76`, `:77` ; `S/debug/LiveLogViewer.tsx:200`, `:209`, `:217` | **Debug** |
| Source ; version ; remerciements | `S/about/AboutSettings.tsx:50`, `:63`, `:73` | **À propos · Visible** |
| Dossier application ; dossier logs | même fichier `:69`, `:70` | **À propos · Carte Diagnostic**, logs en détail expert |

**Aucune suppression de fonctionnalité n’est justifiée par un réglage mort démontré.** « Supprimer » s’applique ici aux points d’entrée redondants après déplacement, pas aux capacités utilisateur.

## 3. MORT, CASSÉ, DOUBLON et gardes Windows

| Statut | Verdict factuel | Preuves |
|---|---|---|
| **CASSÉ — 30 jours** | UI propose `days30`. Le parseur accepte cinq autres valeurs, puis retourne une erreur pour celle-ci. Le gestionnaire sait pourtant traiter `Days30`. | `S/RecordingRetentionPeriod.tsx:38` ; `R/commands/history.rs:125`, `:133` ; `R/managers/history.rs:568` |
| **CASSÉ — faux succès de sauvegarde** | Le store applique la valeur optimiste et attend l’updater sans inspecter son `Result`. Une erreur retournée n’entre pas dans le rollback. | `src/stores/settingsStore.ts:412`, `:430`, `:434` ; `src/bindings.ts:1277` |
| **CASSÉ — feedback prompts/export** | Modification/suppression de prompt et export attendent la commande sans lire `status`. Cela ne signifie pas qu’ils échouent toujours ; leurs échecs peuvent rester silencieux. | `S/post-processing/PostProcessingSettings.tsx:279`, `:294` ; `S/post-processing/LearningSettings.tsx:34` ; `src/bindings.ts:320`, `:532`, `:540` |
| **Contextuellement inactif** | `history_limit` est toujours visible mais ne sert au nettoyage que dans `PreserveLimit`. Windows utilise `Days30` par défaut. | `S/advanced/AdvancedSettings.tsx:71` ; `R/managers/history.rs:563` ; `R/settings.rs:959` |
| **MORT** | Aucun champ utilisateur recensé n’est démontré totalement sans consommateur. Les usages conditionnels ne sont pas des champs morts. | Consommateurs détaillés dans les matrices ci-dessus |
| **DOUBLON — valeurs partagées** | Clé API et URL custom sont éditables depuis Modèles et Post-traitement. Le partage est réel et déjà annoncé dans Modèles. | `S/models/CloudAsrSettings.tsx:38`, `:44`, `:162`, `:177`, `:211`, `:235` |
| **DOUBLON — accès secondaires** | Langue de transcription/traduction : paramètres locaux ou API et tray ; langues/modèle/raccourci/autostart aussi dans Done. Même stockage, pas deux configurations indépendantes. | `S/models/ModelSettingsCard.tsx:28`, `:36` ; `S/models/CloudAsrSettings.tsx:241`, `:256` ; `O/TrayPanel.tsx:111`, `:125`, `:146`, `:185` ; `R/tray.rs:417` |
| **Pas deux formulaires API/prompts actifs** | Les anciens fichiers sont des réexports d’une implémentation unique. En revanche, création et édition du prompt répètent leur JSX dans cette implémentation. | `S/PostProcessingSettingsApi/index.tsx:1` ; `S/PostProcessingSettingsPrompts.tsx:1` ; `S/post-processing/PostProcessingSettings.tsx:367`, `:438` |
| **Windows inutile sans garde** | Aucun cas totalement inutile sous Windows n’est établi. AirPods, outil Linux, capot fermé et diagnostic clavier macOS sont déjà protégés. | `S/AirPodsMode.tsx:20` ; `S/TypingTool.tsx:41` ; `R/helpers/clamshell.rs:57` ; `S/debug/KeyboardDiagnostic.tsx:21` |
| **Garde plateforme inverse manquante** | Le réglage VS Code est affiché partout, mais sa logique cherche les paramètres sous `APPDATA`. Les raccourcis supplémentaires sont créés par défaut sous Windows seulement ; leurs lignes restent montées ailleurs et montrent l’état « absent ». | `S/post-processing/ContextSettings.tsx:98` ; `R/vscode.rs:171` ; `R/settings.rs:1299` ; `S/post-processing/TransformSettings.tsx:21` ; `S/ShortcutInput.tsx:73` |

Le mode 30 jours conserve le texte selon l’âge et réduit l’audio aux **20 dernières dictées non favorites** ; les fichiers importés et favoris sont exclus de ce nettoyage. La description générique parle seulement de suppression d’« enregistrements », alors que des entrées de texte sont aussi supprimées. Preuves : `R/managers/history.rs:540`, `:568`, `:591`, `:658` ; `FR:998`, `:999`, `:1006`.

## 4. Problèmes UX concrets

| Problème | Effet utilisateur et constat vérifiable | Preuves |
|---|---|---|
| **Densité sans séparation par tâche** | API, raccourcis, style, vocabulaire, confidentialité, apprentissage et prompts partagent une longue page. Les groupes ne sont pas des sous-pages ni des sections repliables. | `S/post-processing/PostProcessingSettings.tsx:511` |
| **Avancé sert de rangement général** | Démarrage, mots personnels et conservation côtoient VAD et backend clavier. Les réglages d’un même objectif sont répartis. | `S/advanced/AdvancedSettings.tsx:38`, `:62`, `:70`, `:79` |
| **Activation de l’amélioration difficile à trouver** | Son toggle est derrière Expérimental → Avancé. S’il est désactivé, l’entrée sidebar disparaît. Sous Windows, il est activé par défaut : le problème touche surtout la découverte et la réactivation. | `S/advanced/AdvancedSettings.tsx:78`, `:80` ; `src/components/Sidebar.tsx:68` ; `R/settings.rs:979` |
| **Navigation incohérente avec cette disparition** | Modèles conserve un lien vers Post-traitement ; le rendu de la page ne réapplique pas la condition `enabled` de la sidebar. Page accessible par lien mais absente du menu. | `src/App.tsx:65`, `:67`, `:90` ; `S/models/ModelsSettings.tsx:145` |
| **Deux dictées insuffisamment rapprochées** | Général configure la dictée brute ; le raccourci avec amélioration est ailleurs. Par défaut Windows : `Ctrl+Shift+Space` brut, `Ctrl+Space` amélioré. | `S/general/GeneralSettings.tsx:32` ; `S/post-processing/PostProcessingSettings.tsx:514` ; `R/settings.rs:1257`, `:1279` ; `R/actions.rs:1743`, `:1750` |
| **Toggle au sens trop large** | Le libellé promet d’activer l’affinage IA ; le setter enregistre/désenregistre le raccourci dédié. Les actions Polish/instruction existent séparément. Ce n’est pas un interrupteur général de tous les usages LLM. | `FR:1017`, `:1018` ; `R/shortcut/mod.rs:1312` ; `R/actions.rs:1755`, `:1763` |
| **Explications essentielles cachées** | `SettingContainer` utilise le tooltip par défaut. Lecture/envoi du contexte, exclusions et effets du collage nécessitent une consultation supplémentaire. Tooltip après 380 ms à la souris. | `U/SettingContainer.tsx:21`, `:82` ; `U/Tooltip.tsx:24` ; `S/post-processing/ContextSettings.tsx:86`, `:95` |
| **Vocabulaire instable** | Même domaine nommé « Post-traitement », « reformulation », « mise au propre », « affinage ». « Prompt sélectionné » décrit la sélection d’un « modèle ». | `FR:19`, `:709`, `:728`, `:1018`, `:1172`, `:1173` |
| **Anglais et jargon résiduels** | `Use "…"`, `More information`, tooltip Base URL en anglais ; « Polish », « Snippets », « Live », « Streaming », « LLM » en FR. Les noms techniques ne sont pas tous des erreurs de traduction, mais augmentent l’effort de compréhension. | `S/PostProcessingSettingsApi/ModelSelect.tsx:49` ; `U/InfoTooltip.tsx:19` ; `S/PostProcessingSettingsApi/BaseUrlField.tsx:22` ; `FR:312`, `:361`, `:737`, `:876`, `:1161` |
| **Clé réellement absente** | L’annulation de l’overlay demande `common.cancel`, absent des catalogues FR et EN ; `cancel` existe à la racine. Nom accessible incorrect attendu. | `src/overlay/RecordingOverlay.tsx:310` ; `FR:9`, `:1255` |
| **Texte devenu faux** | Apparence indique que « Masquée reste disponible dans Avancé », alors que le choix existe dans Apparence. | `FR:1158` ; `S/appearance/AppearanceSettings.tsx:112`, `:116` |
| **Sauvegardes de comportements différents** | Toggles immédiats ; textes au blur ; prompts avec bouton explicite. Pas de retour de succès commun ; certains échecs seulement console ou `Result` ignoré. | `S/post-processing/TransformSettings.tsx:55` ; `SnippetSettings.tsx:38` ; `PostProcessingSettings.tsx:406` ; `src/stores/settingsStore.ts:435` |
| **Erreurs modèles non restituées** | Le store conserve les erreurs de chargement, rescan, sélection et suppression ; ModelsSettings ne lit pas `error`. Les téléchargements ont bien leurs notifications. | `src/stores/modelStore.ts:102`, `:127`, `:146`, `:230`, `:285` ; `S/models/ModelsSettings.tsx:53` |
| **Échec confondu avec vide dans l’historique** | Chargement, favori, récupération audio et dossier n’ont que `console.error`. Les états vide/chargement existent, mais pas un état d’erreur explicite pour ces parcours. | `S/history/HistorySettings.tsx:191`, `:264`, `:286`, `:324`, `:330`, `:338` |
| **Suppression sans récupération** | Historique supprime directement, avec retrait optimiste, sans confirmation ni action Annuler. Le backend supprime entrée et WAV. | `S/history/HistorySettings.tsx:293`, `:511` ; `R/managers/history.rs:816`, `:824`, `:833` |
| **Actions de l’historique peu visibles** | Copie et autres actions apparaissent au survol/focus ; elles sont donc moins découvrables au repos. Le focus clavier et l’absence de hover sont bien pris en compte. | `S/history/HistorySettings.tsx:619`, `:632` |
| **Statistiques incohérentes pendant la visite** | Les entrées écoutent les mises à jour ; les statistiques ne sont chargées qu’au montage. | `S/history/HistorySettings.tsx:229`, `:412` |
| **Composants de sélection hétérogènes** | Dropdown maison à boutons versus React Select recherchable/créable ; hauteurs et comportements différents. Ce choix peut être justifié pour la recherche, mais les champs voisins n’ont pas une interaction uniforme. | `U/Dropdown.tsx:76`, `:93`, `:131` ; `U/Select.tsx:52`, `:162` |
| **Étiquettes non reliées aux champs** | Titres dans des `h2`, labels de prompts sans `htmlFor` ; plusieurs inputs sans `aria-label`. Le titre visuel ne fournit pas automatiquement le nom accessible. | `U/SettingContainer.tsx:87` ; `S/HistoryLimit.tsx:37` ; `S/post-processing/ContextSettings.tsx:62` ; `PostProcessingSettings.tsx:370`, `:386` |
| **Fermeture overlay inaccessible au clavier** | Les notices utilisent des `div onClick` sans rôle/bouton ni traitement clavier. | `src/overlay/RecordingOverlay.tsx:432`, `:501` |
| **États de bascule incomplets** | Favori annonce `aria-pressed` ; original/réécrit et plus/moins ne fournissent pas l’état équivalent. Les radios onboarding n’implémentent pas les flèches attendues. | `S/history/HistorySettings.tsx:570`, `:587`, `:661` ; `O/screens/Welcome.tsx:143`, `O/screens/Model.tsx:279` |
| **Contraintes de taille à vérifier** | Onboarding fixé à 720×600 dans un parent masquant le débordement ; champs API `min-width:380px`. Risque de coupe au zoom, non constat visuel reproduit. | `O/OnboardingShell.tsx:34`, `:38` ; `S/post-processing/PostProcessingSettings.tsx:115`, `:171` |
| **Version de secours inventée** | Si lecture de version échoue, À propos affiche `0.1.2`, au lieu d’un état indisponible. | `S/about/AboutSettings.tsx:25`, `:26` |

### Ce qui fonctionne déjà

| Point vérifié | Preuves |
|---|---|
| Modèles possède déjà un résumé d’état, des explications de secours et une section catalogue repliable. Ne pas reconstruire ces mécanismes inutilement. | `S/models/ModelsSettings.tsx:155`, `:171`, `:205`, `:211` |
| Chargement, vide, absence de modèle et absence de résultats sont traités dans plusieurs surfaces. Il serait faux de dire « aucun état vide/chargement ». | `S/history/HistorySettings.tsx:330`, `:338` ; `S/models/ModelsSettings.tsx:205`, `:221` ; `src/components/shared/modelTable/ModelTable.tsx:397` |
| Switches, sliders et navigation ont des noms/focus ; les segmented controls gèrent flèches, Home et End. | `U/Switch.tsx:27`, `:35` ; `U/Slider.tsx:57` ; `U/Segmented.tsx:35` ; `src/components/Sidebar.tsx:155` |
| Mode sombre réellement prévu par les tokens et `color-scheme`. **Contrôle existant passé : huit palettes clair/sombre**. Aucun contraste rendu mesuré ici. | `src/styles/theme.css:45`, `:48`, `:51` ; `src/styles/palettes.css:7` ; `scripts/check-themes.ts:90` |
| **902 clés de référence ; 25 langues complètes**, contrôle de cohérence passé. Cela ne détecte pas les chaînes codées en dur ni un appel à une clé absente des deux catalogues. | `scripts/check-translations.ts:124`, `:136` ; exceptions sourcées ci-dessus |
| Système central de notices déjà disponible : les erreurs silencieuses peuvent y être raccordées sans inventer une nouvelle UI. | `src/stores/noticeStore.ts:86`, `:104`, `:142` |

## 5. Diagnostic affiché aux utilisateurs et structure cible

### Contenu technique actuellement exposé

| Contenu | Position actuelle | Traitement recommandé |
|---|---|---|
| Backend clavier, accélérateur/GPU, backend VAD, fermeture différée du flux | Groupe expérimental d’Avancé | Backend clavier → **Debug** ; paramètres de performance → **Expert**. `S/advanced/AdvancedSettings.tsx:81`, `:85`, `:86`, `:88` |
| Nom de modèle, termes ASR, prompt brut, capture tardive, écho rejeté, erreur cloud | Rapports au milieu de Post-traitement | Résumé de confidentialité → **Contexte** ; détails techniques repliés. `S/post-processing/ContextReports.tsx:99`, `:107`, `:127`, `:147`, `:166` |
| Nombre de termes et attente de capture en millisecondes | Apprentissage | **Contexte · Expert**. `S/post-processing/AsrContextSettings.tsx:113`, `:114` |
| Format API, URL, fournisseur de secours | Post-traitement | **Amélioration · Fournisseur · Expert**. `S/post-processing/PostProcessingSettings.tsx:61`, `:95`, `:192` |
| Dossier des logs | À propos, groupe principal | **À propos · Diagnostic**, détail expert. `S/about/AboutSettings.tsx:70` |
| Note d’évaluation des modèles et légende | Catalogue | Conserver près du choix ; ce sont des aides à la décision, pas des logs. `S/models/ModelsSettings.tsx:242` ; `S/models/Legend.tsx:31` |

### En-têtes d’état proposés

Ce sont des **recommandations**, fondées sur des valeurs déjà présentes, pas des fonctionnalités déclarées existantes.

| Onglet cible | Résumé d’état à privilégier | Base existante |
|---|---|---|
| Général | Raccourcis brut/amélioré, micro courant, langues | `S/general/GeneralSettings.tsx:32`, `:40`, `:46` |
| Modèles | Local/API, fournisseur/modèle, secours disponible | Résumé déjà présent : `S/models/ModelsSettings.tsx:155` |
| Amélioration | Disponibilité, fournisseur/modèle, style/prompt actuel | `S/post-processing/PostProcessingSettings.tsx:45`, `:156`, `:229` |
| Vocabulaire | Mots personnels/appris, apprentissage actif | `S/CustomWords.tsx:16` ; `S/post-processing/LearningSettings.tsx:40`, `:58` |
| Contexte | Lecture active, envoi actif, exclusions | `S/post-processing/ContextSettings.tsx:81`, `:90`, `:108` |
| Historique | Conservation réelle, nombre de dictées, import | `S/RecordingRetentionPeriod.tsx:18` ; `S/history/HistorySettings.tsx:388`, `:408` |
| Avancé | Résumé insertion/capture ; experts repliés | `S/advanced/AdvancedSettings.tsx:55`, `:62` |
| À propos | Version réelle, état mises à jour, accès assistance/diagnostic | `S/about/AboutSettings.tsx:50` ; `src/components/footer/Footer.tsx:88` |

La sidebar peut devenir plus compacte, mais **des icônes seules ne résoudront pas la compréhension** : les textes actuels nomment les destinations et la navigation possède déjà `aria-current`. Conserver des noms accessibles et une identification visible de l’onglet actif. Preuve : `src/components/Sidebar.tsx:153`, `:155`, `:167`.

Les contrôles d’onboarding conservent leur place dans le parcours ; leurs réglages durables renvoient aux mêmes destinations que la matrice. Footer et tray restent des raccourcis vers Général, Modèles, Historique et À propos ; ils ne doivent pas devenir des configurations indépendantes. Preuves : `O/TrayPanel.tsx:111`, `:125`, `:146`, `:185` ; `src/components/footer/Footer.tsx:59`, `:69` ; `R/tray.rs:684`.

## 6. Top 10 par impact utilisateur

| Rang | Problème | Impact | Priorité |
|---:|---|---|---|
| 1 | **« 30 jours » rejeté + faux succès UI** | L’utilisateur croit avoir changé une politique de conservation de données. `S/RecordingRetentionPeriod.tsx:38` ; `R/commands/history.rs:133` ; `src/stores/settingsStore.ts:430` | **P1** |
| 2 | **Échecs de sauvegarde/actions non signalés** | Impossible de distinguer réussite et échec pour plusieurs réglages, prompts, exports et opérations modèles. `src/stores/settingsStore.ts:435` ; `S/post-processing/PostProcessingSettings.tsx:279` ; `src/stores/modelStore.ts:146` | **P1** |
| 3 | **Suppression historique immédiate sans récupération** | Perte de texte/audio après un clic. `S/history/HistorySettings.tsx:293` ; `R/managers/history.rs:833` | **P1** |
| 4 | **Lecture/envoi du contexte noyés dans Post-traitement** | Des choix de confidentialité importants sont mélangés aux réglages de style et à des tooltips. `S/post-processing/ContextSettings.tsx:81`, `:90` | **P1** |
| 5 | **Champs sans nom accessible et fermeture overlay non clavier** | Utilisation dégradée avec lecteur d’écran/clavier. `S/HistoryLimit.tsx:37` ; `S/post-processing/PostProcessingSettings.tsx:370` ; `src/overlay/RecordingOverlay.tsx:432` | **P1** |
| 6 | **Post-traitement concentre jusqu’à `48 + 2S` champs** | Parcours long, faible repérage des tâches et réglages essentiels. `S/post-processing/PostProcessingSettings.tsx:511` ; `S/post-processing/TransformSettings.tsx:46` | **P2** |
| 7 | **Raccourcis brut/amélioré séparés ; activation derrière Expérimental** | Difficile de comprendre quelle dictée utiliser et comment réactiver l’amélioration. `S/general/GeneralSettings.tsx:32` ; `S/advanced/AdvancedSettings.tsx:80` | **P2** |
| 8 | **Conservation et limite d’historique mal expliquées** | Limite affichée sans effet dans le mode courant ; intitulé ne couvre pas clairement la suppression du texte. `R/managers/history.rs:563`, `:658` ; `FR:994`, `:999` | **P2** |
| 9 | **Lexique variable et anglais résiduel** | « Modèle », « prompt », « post-traitement », « reformulation » désignent des concepts insuffisamment distingués. `FR:709`, `:728`, `:1018` ; `S/PostProcessingSettingsApi/ModelSelect.tsx:49` | **P2** |
| 10 | **État et feedback incohérents entre pages** | Modèles explique sa configuration ; d’autres écrans montrent surtout des champs, et Historique peut masquer un échec derrière un état vide. `S/models/ModelsSettings.tsx:155` ; `S/history/HistorySettings.tsx:191`, `:338` | **P2** |

Non vérifiés : rendu réel, contrastes pixels, zoom, navigation clavier complète et matériel ; aucun build ni parcours Tauri exécuté.  
Risques confirmés dans le code : conservation non sauvegardée, erreurs silencieuses et suppressions sans récupération.