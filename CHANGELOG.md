# Notes de version

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
