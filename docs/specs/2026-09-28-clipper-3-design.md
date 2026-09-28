# Clipper 3.0 — conception

Statut : validée par titilyonnais le 2026-09-28 (« je valide tout, en totale autonomie »), implémentée en 3.0.0.

## Objectif

Une refonte complète, livrée en une version : Clipper devient un outil rapide, très bien intégré à Windows 10/11, au style de PostShip (noir pur, monochrome, sobre), sans bug connu. Public : l'auteur d'abord, publiable proprement.

## Décisions

| Sujet | Décision |
|---|---|
| Ouverture | Win+V (réglage officiel `DisabledHotkeys` de l'Explorateur, jamais de hook clavier) ou raccourci personnalisé |
| Popup | Fenêtre centrée sur l'écran actif, style palette : recherche, onglets Historique / Snippets / Collections, épinglés en tête, Ctrl+1…9, aperçu à droite |
| Choix d'un élément | Colle directement dans l'application précédente (focus restauré + Ctrl+V simulé) ; Maj+Entrée colle en texte brut |
| Grande fenêtre | Gestion complète ; visible dans la barre des tâches et Alt+Tab quand elle est ouverte |
| Style | Jetons PostShip : noir `#000`, surfaces `#0f0f0f`/`#1a1a1a`/`#242424`, bordures `#2a2a2a`/`#333`, texte `#ededed`/`#a1a1a1`/`#868686` ; thème clair qui suit Windows ; monochrome pur (aucune couleur d'accent) ; Onest (graisse max 500) + JetBrains Mono ; rayons 7 px (contrôles), 6 px (cartes), 8 px (dialogues) ; aucun dégradé, verre, lueur ni `hover:scale` |
| Logo | Carré arrondi contenant trois lignes d'historique décroissantes (piste D), SVG en `currentColor` |
| Icône de notification | Monochrome, suit le thème de la barre des tâches, variante barrée en pause/incognito |
| Langue | Français seulement |
| IA | Conservée (Ollama, Claude, OpenAI) dans un menu « IA » discret |
| Secrets | Détectés, masqués à l'affichage et exclus de la recherche, jamais effacés automatiquement |
| Historique existant | Migré entièrement |

## Fonctionnalités

- **Collage direct** et **texte brut** (Maj+Entrée ; réglage « toujours en texte brut »).
- **Snippets** : textes permanents avec titre, abréviation facultative (ex. `;sig`, reconnue dans la recherche du popup) et variables `{date}`, `{heure}`, `{jour}`, `{presse-papiers}`.
- **File de collage** : sélection multiple puis « Coller en série » ; chaque Ctrl+V dans n'importe quelle application colle l'élément suivant. Implémentation par rendu différé du presse-papiers (`WM_RENDERFORMAT`), sans surveiller le clavier.
- **Modifier avant de coller** : édition du texte, puis coller, enregistrer ou coller sans enregistrer.
- **OCR** des images avec le moteur OCR de Windows (hors ligne) ; le texte reconnu est cherchable. Traitement en arrière-plan, y compris des images existantes.
- **Mise en forme riche** : les formats HTML et RTF sont conservés avec le texte et recollés tels quels (le texte brut reste disponible).
- **Aperçus intelligents** : pastille de couleur (`#hex`, `rgb()`, `hsl()`), domaine des liens, JSON formaté, informations de fichiers, dimensions d'images.
- **Collections** : groupes nommés (remplacent les catégories ; migration automatique), glisser-déposer d'un élément vers une collection. Les éléments d'une collection ne sont jamais supprimés automatiquement. Les tags sont conservés.
- **Épinglés** : restent en tête et ne sont jamais supprimés automatiquement. Les « favoris » de la v2 deviennent des épinglés (concept redondant).
- **Filtre par application d'origine**, avec l'icône de l'exécutable.
- **Glisser-déposer** : d'un élément vers une collection ; du texte vers une autre application depuis la poignée de l'aperçu.
- **Mode incognito** : pause 5 min, 1 h ou jusqu'à réactivation (popup, fenêtre, zone de notification).
- **Sauvegardes automatiques** quotidiennes de la base (`VACUUM INTO`), 7 conservées, restauration depuis les paramètres.
- **Accueil** au premier lancement : Win+V, collage direct, démarrage avec Windows.

Hors périmètre 3.0 : synchronisation entre PC, verrouillage Windows Hello, expansion d'abréviations dans toutes les applications (exigerait un hook clavier).

## Architecture

- **Deux fenêtres Tauri, un seul bundle React** : `main` (grande fenêtre) et `popup` (préchargée, cachée, sans bordure, toujours au premier plan, masquée à la perte de focus). Le composant racine choisit l'interface selon l'étiquette de la fenêtre.
- **Frontend** : React 19, Tailwind v4 (jetons dans `@theme`), polices locales (sous-ensembles latin), Lucide, highlight.js limité aux langages détectés. Petite bibliothèque de composants maison (`Button`, `IconButton`, `Input`, `Kbd`, `Badge`, `Menu`, `Dialog`, `Switch`, `Segmented`, `Tabs`).
- **Backend Rust**, modules :
  - `clipboard` : capture événementielle (texte, HTML, RTF, fichiers, images), écriture, rendu différé pour la file de collage.
  - `paste` : mémorise la fenêtre active à l'ouverture du popup, restaure le focus et envoie Ctrl+V (`SendInput`).
  - `hotkey` : Win+V via `DisabledHotkeys` + relance propre de l'Explorateur (message « Quitter l'Explorateur » à `Shell_TrayWnd`, puis relance).
  - `ocr` : WinRT `Windows.Media.Ocr`, file d'attente en arrière-plan.
  - `secrets` : détection (préfixes de clés connues, JWT, clés privées PEM, cartes bancaires avec Luhn, jetons à forte entropie) + stockage des clés API (Gestionnaire d'identifiants).
  - `appicons` : icône de l'exécutable d'origine en PNG, mise en cache.
  - `backup`, `db` (schéma v3), `ai`, `commands`, `tray`, `windows` (placement du popup).
- **Schéma v3** : `clips` gagne `sensitive`, `ocr_text`, `collection_id`, `has_rich`, `app_path` ; tables `collections`, `snippets`, `clip_formats` (données HTML/RTF, suppression en cascade). L'index FTS couvre `content` (vide pour les secrets), `ocr_text` et `tags`.

## Qualité

- Tests Rust : migration v2 → v3 (y compris sur une copie de la vraie base), détection de secrets, variables de snippets, file de collage (logique d'état), requêtes.
- Clippy sans avertissement, `tsc` strict, build Vite.
- Vérification manuelle de l'app compilée : capture, popup, collage direct, Win+V, OCR, thèmes, et inspection visuelle par captures d'écran.
