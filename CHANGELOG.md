# Notes de version

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
