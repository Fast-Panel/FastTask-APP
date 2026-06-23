# FastTask — Application de bureau

Application de bureau pour [FastTask](https://task.fastpanel.fr), construite avec [Tauri v2](https://tauri.app).

Charge `https://task.fastpanel.fr` dans une fenêtre native sur Windows, macOS et Linux.

## Téléchargement

Voir les [Releases GitHub](https://github.com/Fast-Panel/FastTask-APP/releases) pour télécharger la dernière version.

| Plateforme | Format |
|---|---|
| Windows | `.msi` (installeur) · `.exe` (portable) |
| macOS | `.dmg` (Intel + Apple Silicon) |
| Linux | `.AppImage` · `.deb` |

> **macOS** : L'app n'est pas signée Apple. Au premier lancement : clic droit → Ouvrir.

## Développement local

**Prérequis** : [Rust](https://rustup.rs), [Node.js 20+](https://nodejs.org), et les [dépendances Tauri](https://tauri.app/start/prerequisites/) pour ta plateforme.

```bash
# Installer le CLI Tauri et générer les icônes
npm install
npm run setup

# Lancer en mode développement
npm run dev

# Compiler
npm run build
```

## Releases

Les builds sont déclenchés automatiquement sur les tags git :

```bash
# Mettre à jour la version dans src-tauri/tauri.conf.json
# puis :
git tag v1.2.0
git push origin v1.2.0
```

GitHub Actions compile ensuite l'app pour Windows, macOS et Linux et publie une Release.

## Stack

- [Tauri v2](https://tauri.app) — shell natif léger (~8 Mo)
- GitHub Actions — build multi-plateforme sans machine dédiée
