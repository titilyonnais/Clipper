# Notes de version

## 3.6.0

### Applications ignorées
- Plus besoin de taper le chemin d'un .exe : « Ajouter » ouvre la liste des applications installées, détectées en une fraction de seconde (menu Démarrer, programmes installés, applications ouvertes, historique).
- Suggestions en tête : gestionnaires de mots de passe, authentification à deux facteurs, portefeuilles crypto, banques, accès à distance… Puis les applications dont vous avez déjà copié quelque chose, les plus récentes d'abord.
- Recherche instantanée, y compris par initiales (« vsc ») ou par type (« mots de passe »), au clavier avec ↑ ↓ et Entrée. « Parcourir… » ouvre l'explorateur Windows pour choisir n'importe quel exécutable.
- Chaque application apparaît avec son icône et son vrai nom ; les plus pertinentes sont aussi proposées directement dans Paramètres › Capture.

### Documentation dans l'application
- Nouvel onglet « Guide » dans les paramètres : tout Clipper en quelques sections repliables.
- Les notes de version sont dans À propos, celle de la version installée dépliée.

### Corrections
- Grande fenêtre : copier un élément après avoir fait défiler la liste ne ramène plus en haut ; l'élément reste sous le pointeur et reprend sa place au prochain affichage.
- Collage rapide : la sélection revient sur le premier élément à chaque ouverture. Ctrl+1…9 fonctionne aussi avec le pavé numérique et sur tous les claviers (AZERTY compris) ; avec Maj, en texte brut.
- Après une suppression, la sélection passe à l'élément suivant au lieu de remonter en haut.
- Snippets : changer de snippet avec des modifications non enregistrées demande d'abord quoi en faire.
- Réduire la limite d'éléments demande confirmation quand des éléments vont être supprimés.
- Réglages : deux changements rapprochés ne s'écrasent plus, et un réglage illisible ne remet plus tous les autres à zéro.
- Restauration d'une sauvegarde : le raccourci, Win+V, le démarrage avec Windows et la pause restent ceux de cet ordinateur ; les images de l'historique remplacé sont gardées 30 jours.
- Presse-papiers occupé par une autre application : Clipper insiste un peu au lieu de manquer la copie.
- Collage en série : reprend la main si une autre application lit le presse-papiers en boucle.
- Texte des images : respecte l'option « Masquer les secrets » et s'affiche dès qu'il est reconnu.
- Un secret copié à nouveau après avoir activé la détection est désormais masqué, sans sa mise en forme.
- Annuler une suppression retrouve toujours l'image, même si le nettoyage automatique est passé entre-temps.
- Images enregistrées de façon sûre : une coupure ne laisse plus de fichier incomplet.
- Raccourcis affichés avec les touches de votre clavier (Maj, Win, lettres AZERTY).
- Renommer une collection ou modifier des tags : Échap annule vraiment.
- Intelligence artificielle : Claude Opus 5.5 par défaut, Sonnet 5.5 proposé.

## 3.5.1

- Première version livrée par la mise à jour automatique : elle s'installe depuis Paramètres › À propos, sans fenêtre, et Clipper redémarre tout seul.
- Fenêtres de dialogue : fond sombre uni à la place du flou, qui donnait un aspect compressé.
- Aide des raccourcis du collage rapide : fond opaque, sans flou.
- Fenêtre de mise à jour : contour de sélection du bouton plus discret.

## 3.5.0

### Mises à jour automatiques
- Paramètres › À propos : recherche des nouvelles versions sur GitHub, au démarrage puis toutes les six heures (désactivable). Une pastille discrète apparaît dans la barre latérale quand une version est disponible.
- Installation en un clic : fenêtre noire avec les nouveautés, puis une simple barre de progression. Clipper se ferme, s'installe sans aucune fenêtre et redémarre tout seul.
- Chaque installateur est signé : Clipper refuse tout fichier dont la signature ne correspond pas à sa clé. Win+V, le démarrage avec Windows et l'historique sont conservés.

### Interface
- Nouveau filtre « Sensibles » dans la barre latérale : uniquement les éléments masqués (mots de passe, clés…).
- Filtre par date : le calendrier s'ouvre centré sous les boutons, avec une flèche qui les désigne ; au-dessus s'il manque de place.
- Collage rapide : plus de contour clair ni de flash à la fermeture et après Ctrl+V. La fenêtre redevient opaque comme en 3.2 ; coins arrondis et ombre sont dessinés par Windows.

### Applications d'origine
- Ce qui est copié dans Clipper apparaît sous « Clipper » avec son logo, et non plus « MSEdgeWebView2 ». Idem pour les autres applications à interface web (Teams, Outlook…). Les anciens éléments sont corrigés.
- Icônes d'applications plus nettes (64 px) ; celles qui manquaient (PowerShell…) sont retrouvées en arrière-plan et s'affichent sans redémarrer.

### Dépendances
- Mise à jour de toutes les dépendances (Vite 8, TypeScript 7, React, Tauri, SQLite…).

## 3.4.0

### Interface
- Filtre par date : 7 et 30 derniers jours, 3, 6 et 12 derniers mois, ou deux jours choisis sur un calendrier.
- Collage rapide (Win+V) : plus de flash à l'ouverture. Le panneau est vidé à l'écran avant que la fenêtre se cache, et le rendu reste en sRGB sur les écrans HDR.
- Menu « Copier en… » : plus d'espace vide à gauche des libellés.
- Dates des éléments : « hier » veut dire la veille ; au-delà d'une semaine, la date est affichée (« 20 sept. »).

### Espace disque
- Images recompressées sans aucune perte en arrière-plan (priorité basse), vérifiées pixel par pixel : environ 40 % de place en moins.
- Petites vignettes pour les listes : les grandes captures ne sont plus décodées pour une icône.
- Sauvegardes compressées (environ trois fois plus petites), décompressées seulement pour une restauration ; les anciennes sont converties.
- Moteur d'affichage : cache limité, pas de composants de navigateur téléchargés, rapports de plantage et caches des versions précédentes supprimés.

### Corrections
- Démarrage avec Windows : l'entrée effacée lors d'une mise à jour est recréée au lancement ; un arrêt demandé dans le Gestionnaire des tâches est respecté.
- Mise à jour : Win+V n'est plus rendu à Windows pendant la désinstallation qui précède l'installation d'une nouvelle version.

## 3.3.1

- Collage rapide : plus aucun halo gris autour de la carte. Windows ne dessine plus de cadre, d'ombre ni de coins autour de la fenêtre transparente, et la carte n'a plus d'ombre ; la fenêtre a exactement la taille de la carte.

## 3.3.0

### Interface
- Barre latérale : les icônes restent exactement à leur place quand on la replie ; le bouton de repli est dans la barre de titre, aligné sur elles. « Nouvelle collection » devient une ligne de la liste.
- Filtres de type : des icônes (avec infobulles) remplacent les mots quand la liste est étroite ; le raccourci affiché dans la recherche disparaît au lieu de chevaucher le texte.
- « Aucun élément sélectionné » est centré dans l'aperçu et aligné sur le message de la liste.
- Collage rapide : panneau aux coins arrondis avec une ombre douce, apparition et disparition animées (Échap), lignes qui arrivent à chaque ouverture ; l'animation générique de Windows est désactivée.

### Sécurité
- Ouverture de fichiers : le chemin réel est résolu avant le contrôle (« programme.exe. » ne passe plus), liste d'extensions à risque étendue.
- Import : les chemins réseau ou de périphérique sont refusés dans les listes de fichiers.
- IA : l'adresse est analysée (plus de contournement par « localhost@… ») ; un secret n'est envoyé qu'à un modèle sur cette machine.
- Export : les éléments sensibles ne sont plus exportés ; le fichier est écrit au fur et à mesure.
- Secrets détectés aussi dans les longs textes (fichiers .env…) et dans le texte reconnu sur les images, qui n'est alors pas indexé.
- Les éléments supprimés sont effacés du fichier de la base, pas seulement déréférencés.

### Corrections
- Désinstallation : Win+V est rendu à Windows si Clipper l'avait pris.
- Si Win+V n'est pas encore libre, le raccourci personnalisé reste actif.
- Relance de l'Explorateur : attend la fin du processus avant de le redémarrer.
- Collage : plus de collage dans une fenêtre ouverte il y a longtemps quand le collage rapide est ouvert depuis Clipper ; message clair pour les applications lancées en administrateur.
- Modifier un texte identique à un autre élément conserve épingle, collection et tags.
- Annulation d'une suppression limitée à 15 secondes, y compris si la collection a été supprimée entre-temps.
- Fichiers d'images orphelins supprimés au démarrage ; éléments sans image retirés après une restauration.
- Sauvegardes : la copie ne bloque plus la capture ; les sauvegardes manuelles sont aussi limitées aux sept dernières.
- Le presse-papiers est libéré avant la conversion des images (les autres applications ne sont plus bloquées).
- Deux réglages modifiés rapidement sont tous deux enregistrés.
- Fin d'une série de collages à la fermeture : le dernier élément reste bien dans le presse-papiers.
- Code inutilisé retiré (commandes, fonctions, doublons).

## 3.2.0

### Raccourcis
- Nouvel onglet Paramètres › Raccourcis clavier : chaque action de la grande fenêtre a un raccourci modifiable (copier, texte brut, modifier, épingler, supprimer, snippet, rechercher, nouvelle collection, barre latérale…).
- Les raccourcis marchent partout dans la fenêtre, plus seulement quand la liste a le focus. Ils sont affichés dans les menus et les infobulles.

### Aperçu
- Barre d'actions sans doublon : Copier, Copier en…, Modifier, Épingler et Supprimer sont visibles ; le menu ⋯ ne contient que le reste (collections, snippet, masquer, IA, ignorer l'application).
- Modification sur place : la barre devient Enregistrer / Copier le texte modifié / Annuler. Quitter un texte modifié non enregistré demande confirmation.
- L'application d'origine est indiquée en haut (« Copié depuis Brave ») ; « Ne plus enregistrer depuis… » est dans le menu ⋯, avec confirmation.
- Suppression immédiate avec « Annuler » dans la notification (ou Ctrl+Z).

### Fenêtre
- Barre latérale repliable en icônes (bouton ou Ctrl+B), automatiquement quand la fenêtre est étroite ; les boutons de l'aperçu se compactent quand la place manque.
- Collections créées et renommées directement dans la barre latérale ; bouton + plus grand.
- Menus déroulants : un second clic sur le bouton les referme.
- Fond des fenêtres de dialogue : flou prononcé, qui apparaît progressivement.
- Le message « Aucun élément sélectionné » est centré dans toutes les vues.

## 3.1.0

### Interface
- Plus de liseré blanc en haut des fenêtres : le cadre de Windows 11 prend la couleur du thème, coins arrondis conservés.
- Collage rapide sans bordure, pied de page réduit à l'essentiel ; tous les raccourcis sont dans un panneau (F1).
- Barre latérale et liste redimensionnables à la souris ou au clavier (double-clic pour revenir à la taille d'origine), largeurs mémorisées.
- Animations : apparition du collage rapide, indicateur qui glisse dans les menus et les filtres, arrivée des éléments de la liste, fenêtres et notifications.
- Filtres de l'historique : le calendrier ne déborde plus, il est à côté de la recherche.
- Paramètres : libellés et contrôles alignés verticalement, contrôles de même hauteur partout.

### Corrections
- Les chemins copiés comme texte avec la version 1 (`C:\…`) étaient classés en fichiers et leur aperçu restait noir : ils redeviennent du texte (migration automatique de la base).
- Les aperçus de fichiers affichent le dossier d'origine et une erreur explicite si la lecture échoue.
- La date « il y a 19 h » ne passe plus sur deux lignes dans le collage rapide.

## 3.0.0

Nouvelle interface et nouvelles fonctionnalités. L'historique existant est migré automatiquement.

### Interface
- Refonte complète dans le style de PostShip : noir pur, monochrome, police Onest, composants sobres.
- Thème clair qui suit Windows, densité réglable.
- Nouveau logo, icône de notification qui suit le thème de la barre des tâches et signale la pause.
- Accueil en trois étapes au premier lancement.

### Collage rapide
- Nouveau popup centré (Win+V ou raccourci personnalisé) : recherche, onglets Historique / Snippets / Collections, Ctrl+1…9.
- Collage direct dans l'application précédente, Maj+Entrée pour le texte brut.
- Win+V remplace l'historique de Windows via le réglage officiel de l'Explorateur, sans hook clavier.

### Fonctionnalités
- Snippets avec abréviations et variables.
- File de collage : chaque Ctrl+V colle l'élément suivant.
- Modifier un élément avant de le coller.
- OCR des images (moteur Windows, hors ligne) : le texte des captures est cherchable.
- Conservation de la mise en forme riche (HTML, RTF).
- Détection et masquage des secrets ; ils ne sont ni indexés ni envoyés à une IA en ligne.
- Collections (remplacent les catégories), glisser-déposer, filtre par application avec son icône.
- Mode incognito temporisé, sauvegardes quotidiennes avec restauration.
- Aperçus : couleurs, liens, JSON formaté, nombre de mots et de lignes.

### Corrections et technique
- Base v3 : index de recherche sans contenu (ne peut pas contenir de secret), formats riches en table séparée, favoris fusionnés avec les épinglés.
- Les éléments épinglés ou rangés dans une collection ne sont jamais supprimés automatiquement.
- Plus aucune dépendance au web à l'exécution (polices intégrées).


## 2.0.0

Refonte complète. L'ancienne base est migrée automatiquement au premier lancement.

### Sécurité
- L'interface ne peut plus lire, écrire ou ouvrir des fichiers arbitraires : les actions sur les fichiers passent par l'identifiant de l'élément et le chemin est lu dans la base.
- Ouverture des fichiers via l'API Windows (ShellExecute) au lieu de `cmd /C start` (qui permettait une injection de commande avec certains noms de fichiers). Les programmes et scripts ne sont plus lancés depuis Clipper.
- Clés API déplacées dans le Gestionnaire d'identifiants de Windows ; elles ne transitent plus par l'interface.
- Respect des marqueurs de confidentialité des gestionnaires de mots de passe, liste d'applications ignorées.
- Politique de sécurité du contenu stricte, permissions de la fenêtre réduites au minimum, plus aucune police chargée depuis Internet.
- HTTPS obligatoire pour envoyer une clé API vers un serveur distant.

### Performances et espace disque
- Capture événementielle (`AddClipboardFormatListener`) au lieu d'une lecture du presse-papiers toutes les 500 ms, qui ré-encodait en PNG toute image présente et pouvait gêner les autres applications.
- Images stockées en fichiers PNG plutôt qu'en base64 dans la base et dans l'index de recherche.
- Liste paginée sans le contenu complet des éléments ; aperçu chargé à la demande.
- Index de recherche allégé et non réécrit à chaque copie ; récupération de l'espace disque après suppression.
- Nombre maximal d'éléments par défaut : 5 000 (réglable, 0 = illimité).
- Exécutable et interface allégés (dépendances inutiles supprimées, coloration syntaxique limitée aux langages détectés).

### Corrections
- Plantage possible des actions IA sur un texte accentué.
- Entrée et Échap dans un champ de saisie copiaient l'élément ou masquaient la fenêtre.
- Carte d'activité décalée d'un jour selon le fuseau horaire ; dates relatives affichées en anglais.
- Clipper ne ré-enregistrait plus le démarrage automatique si vous l'aviez désactivé ailleurs dans Windows.
- Double icône possible dans la zone de notification.
- La limite d'éléments ne s'appliquait qu'au texte.

### Publication
- Compilation et publication des installateurs par GitHub Actions (`npm run release -- x.y.z`).
- Installateur NSIS par utilisateur, sans droits administrateur.
