<img src="public/clipper.svg" width="56" alt="">

# Clipper

Gestionnaire de presse-papiers pour Windows 10/11 : historique local, recherche instantanée, catégories et tags, aperçu du code avec coloration syntaxique, actions IA facultatives (Ollama en local, Claude ou OpenAI).

Construit avec Tauri 2 (Rust + WebView2), React et SQLite.

## Fonctionnalités

- **Collage rapide** : Win+V (ou le raccourci de votre choix) ouvre une palette centrée sur l'écran. Entrée colle directement dans l'application où vous étiez, Maj+Entrée en texte brut, Ctrl+1…9 colle l'un des neuf premiers éléments.
- **Historique complet** : texte, code (coloration syntaxique), liens, images et fichiers copiés dans l'Explorateur, avec la mise en forme d'origine (Word, Excel, web).
- **Recherche instantanée**, y compris dans le texte des captures d'écran grâce à l'OCR intégré à Windows (hors ligne).
- **Snippets** : textes permanents avec abréviation (`;sig`) et variables `{date}`, `{heure}`, `{jour}`, `{presse-papiers}`.
- **File de collage** : sélectionnez plusieurs éléments, chaque Ctrl+V colle le suivant.
- **Modifier avant de coller**, épingler, ranger dans des **collections** (par glisser-déposer), tags, filtre par application d'origine.
- **Secrets masqués** : mots de passe, clés d'API, jetons et cartes bancaires sont détectés, masqués et exclus de la recherche.
- **Mode incognito** (5 min, 1 h ou jusqu'à réactivation), applications ignorées, sauvegardes quotidiennes, export / import.
- **IA facultative** : résumer, expliquer, reformuler, corriger, traduire, classer, avec Ollama en local, Claude ou OpenAI.
- Thème noir ou clair (suit Windows), icône de notification qui suit le thème de la barre des tâches.

## Installer

Téléchargez `Clipper_x.y.z_x64-setup.exe` depuis la page [Releases](https://github.com/titilyonnais/Clipper/releases). L'installation se fait dans votre profil, sans droits administrateur.

Tant que l'exécutable n'est pas signé avec un certificat de signature de code, Windows SmartScreen peut afficher « Windows a protégé votre ordinateur » : cliquez sur *Informations complémentaires* puis *Exécuter quand même*. Voir [Signature](#signature-du-code) pour supprimer cet avertissement.

## Publier une nouvelle version

La compilation se fait sur les serveurs de GitHub : rien de lourd à installer ni à compiler sur votre PC.

```bash
npm run release -- 2.0.1
```

Le script (qui utilise la [CLI GitHub](https://cli.github.com), `gh`) change le numéro de version sur une branche `release/v2.0.1` et ouvre une pull request qui se fusionne toute seule dès que la CI est verte. L'arrivée de la nouvelle version sur `main` déclenche [`release.yml`](.github/workflows/release.yml), qui compile l'installateur `.exe` (NSIS) et le `.msi`, puis publie la release avec leurs empreintes SHA-256 (environ 10 minutes la première fois, nettement moins ensuite grâce au cache).

La branche `main` est protégée : toute modification passe par une pull request vérifiée par [`ci.yml`](.github/workflows/ci.yml) (typage, build, Clippy, tests). Dependabot propose chaque semaine les mises à jour des dépendances.

## Développer en local (facultatif)

Prérequis : [Node.js 22+](https://nodejs.org), [Rust](https://rustup.rs) et les *Build Tools* Visual Studio (charge « Développement Desktop en C++ »).

```bash
npm install
```

```bash
npm run app
```

`npm run app` lance l'application avec rechargement à chaud de l'interface. La première compilation Rust prend quelques minutes ; les suivantes sont incrémentales. `npm run installer` produit les installateurs localement dans `src-tauri/target/release/bundle/`.

Tests du backend :

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Architecture

```
src/                     Interface React (TypeScript, Tailwind v4)
  main/                  Grande fenêtre : historique, snippets, paramètres, accueil
  popup/                 Collage rapide
  clip/                  Liste, aperçu, éditeur, actions, IA (partagés)
  ui/                    Composants (boutons, champs, menus, dialogues)
src-tauri/src/
  clipboard/             Capture événementielle, écriture, file de collage (rendu différé)
  db/                    SQLite + FTS5, migrations, collections, snippets
  commands/              Commandes exposées à l'interface
  paste.rs               Collage direct dans l'application précédente
  hotkey.rs              Prise en charge de Win+V
  ocr.rs                 Windows.Media.Ocr
  sensitive.rs           Détection des secrets
  tray.rs, window.rs     Zone de notification, fenêtres
```

La conception détaillée est dans [`docs/specs/2026-09-28-clipper-3-design.md`](docs/specs/2026-09-28-clipper-3-design.md).

## Signature du code

Un exécutable non signé déclenche SmartScreen, et les antivirus accordent plus facilement leur confiance à un éditeur identifié. Options, de la plus simple à la plus complète :

1. **Azure Trusted Signing** (une dizaine de dollars par mois) ou **SignPath Foundation** (gratuit pour les projets open source) : signature depuis GitHub Actions sans manipuler de certificat. Tauri appelle l'outil via `bundle.windows.signCommand` dans `src-tauri/tauri.conf.json`.
2. **Certificat OV/EV classique** : renseignez `bundle.windows.certificateThumbprint`.

Si un antivirus signale malgré tout un faux positif, soumettez le fichier à [Microsoft](https://www.microsoft.com/wdsi/filesubmission) (et à l'éditeur concerné) : l'analyse prend généralement un à trois jours.

## Licence

MIT
