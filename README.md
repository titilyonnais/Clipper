# Clipper

Gestionnaire de presse-papiers pour Windows 10/11 : historique local, recherche instantanée, catégories et tags, aperçu du code avec coloration syntaxique, actions IA facultatives (Ollama en local, Claude ou OpenAI).

Construit avec Tauri 2 (Rust + WebView2), React et SQLite.

## Fonctionnalités

- Capture du texte, du code, des liens, des images et des fichiers copiés dans l'Explorateur.
- Recherche plein texte (insensible aux accents), filtres par type, date, taille, catégorie, tags.
- Épingler, favoris, catégories, tags ; « Copier en… » (minuscules, JSON, URL, Base64…).
- Raccourci global configurable (par défaut `Ctrl+Shift+V`), icône dans la zone de notification.
- Rétention automatique (durée et nombre maximal d'éléments), export / import JSON.
- IA : résumer, expliquer, reformuler, corriger, traduire, classer automatiquement.

## Confidentialité et sécurité

- Tout est stocké localement dans `%APPDATA%\com.clipper.app\` (base SQLite + dossier `images`).
- Aucune télémétrie, aucune connexion réseau, sauf si vous lancez une action IA (vers Ollama sur votre machine, ou vers le fournisseur choisi).
- Les données marquées confidentielles par les gestionnaires de mots de passe (KeePass, Bitwarden, 1Password…) ne sont jamais enregistrées, et vous pouvez ignorer n'importe quelle application.
- Les clés API sont conservées dans le Gestionnaire d'identifiants de Windows, jamais dans la base ni dans l'interface.
- L'interface n'a accès ni au disque ni au réseau : elle ne peut agir que sur les éléments de l'historique, via des commandes Rust validées. Les programmes et scripts copiés ne sont jamais lancés depuis Clipper.

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
src/                     Interface React (TypeScript, Tailwind)
  App.tsx                État, pagination, raccourcis clavier
  components/            Liste, aperçu, paramètres, filtres…
  lib/api.ts             Appels typés vers le backend
src-tauri/src/
  lib.rs                 Démarrage, zone de notification, raccourci global, nettoyage horaire
  clipboard.rs           Écoute du presse-papiers (AddClipboardFormatListener) et écriture
  db.rs                  SQLite + FTS5, migrations, rétention
  commands.rs            Commandes exposées à l'interface
  ai.rs                  Ollama / OpenAI / Claude
  secrets.rs             Clés API dans le Gestionnaire d'identifiants Windows
```

La capture est événementielle : Clipper ne fait rien tant qu'une autre application ne modifie pas le presse-papiers, et ne l'ouvre que le temps de copier les données.

## Signature du code

Un exécutable non signé déclenche SmartScreen, et les antivirus accordent plus facilement leur confiance à un éditeur identifié. Options, de la plus simple à la plus complète :

1. **Azure Trusted Signing** (une dizaine de dollars par mois) ou **SignPath Foundation** (gratuit pour les projets open source) : signature depuis GitHub Actions sans manipuler de certificat. Tauri appelle l'outil via `bundle.windows.signCommand` dans `src-tauri/tauri.conf.json`.
2. **Certificat OV/EV classique** : renseignez `bundle.windows.certificateThumbprint`.

Si un antivirus signale malgré tout un faux positif, soumettez le fichier à [Microsoft](https://www.microsoft.com/wdsi/filesubmission) (et à l'éditeur concerné) : l'analyse prend généralement un à trois jours.

## Licence

MIT
