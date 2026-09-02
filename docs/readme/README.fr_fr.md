<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — un lanceur Minecraft moderne et multiplateforme, affichant quatre thèmes Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>Un lanceur Minecraft petit, rapide et léger</strong><br>
  Gestion multi-instances · Market de mods et packs de ressources · Multijoueur cross-LAN · Support multi-thèmes</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md"><strong>Français</strong></a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Qu'est-ce que c'est

Conic Launcher est un lanceur Minecraft moderne pour bureau : la logique principale est implémentée en Rust, l'interface est construite avec Tauri 2 + Vue 3. Il se caractérise par une petite taille d'installation, un démarrage rapide et une faible consommation de ressources, contribuant même à réduire les émissions de carbone.

De la création d'instances et l'installation de chargeurs à la recherche de mods et l'invitation d'amis pour jouer ensemble, en passant par le lancement du jeu et le suivi du temps de jeu — l'ensemble du workflow peut être complété dans une seule application.

> Le projet est actuellement en phase alpha précoce. Les fonctionnalités et l'interface évoluent rapidement — n'hésitez pas à l'essayer et à signaler des problèmes.

## Fonctionnalités

### Gestion multi-instances

Parcourez les instances groupées par chargeur de mods, avec tri, recherche et favoris. Chaque instance possède ses propres paramètres, un arrière-plan personnalisé, le suivi du temps de jeu et l'horodatage de la dernière exécution.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Support de tous les chargeurs majeurs

Choisissez votre version de Minecraft et votre version de chargeur — le lanceur s'occupe du reste. Vanilla, Forge, NeoForge, Fabric et Quilt sont tous supportés, avec une progression d'installation en temps réel.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Market de mods et packs de ressources

Recherche intégrée Modrinth et CurseForge : parcourez les détails des projets et les dépendances, lisez les résumés traduits et installez des mods ou des packs de ressources dans vos instances en un clic. Vous pouvez également gérer les mods installés localement directement.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Multijoueur cross-LAN Conic Nexus

Pas d'outils supplémentaires nécessaires : créez un groupe, partagez le code d'invitation avec vos amis et rejoignez le même monde ensemble. Le lanceur détecte votre environnement réseau (type NAT) et fournit des suggestions d'amélioration.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Plusieurs méthodes de connexion

Prend en charge la connexion au compte Microsoft (flux code appareil), la connexion hors ligne et tout serveur d'authentification externe Authlib-Injector / Yggdrasil. Les comptes offrent aperçu 3D des skins, affichage des capes et téléchargement de skins.

<img width="840" height="533" alt="Capture d'écran 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Expérience de lancement sans tracas

Analyse automatique du Java sur votre système, ou téléchargement automatique d'un runtime approprié. La mémoire est allouée automatiquement (avec protection de la limite Java 32 bits). Le processus de lancement est entièrement visualisé : téléchargement des fichiers de jeu, installation des chargeurs, résolution des dépendances et attente du démarrage du processus.

Les utilisateurs avancés ne sont pas oubliés — le ramasse-ordures JVM, les arguments JVM, les arguments de jeu, le classpath, la commande wrapper et les commandes avant/après le lancement sont tous personnalisables.

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Sauvegardes et données de jeu

Parcourez les cartes de mondes des sauvegardes, gérez les packs de données et packs de ressources, et consultez les captures d'écran du jeu — le tout dans le lanceur.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Interface et personnalisation

Quatre thèmes Catppuccin (Mocha · Macchiato · Frappé · Latte), plus des variantes à contraste élevé, avec passage automatique clair/sombre selon les paramètres de votre système. Arrière-plans de lanceur personnalisés supportés. Lecteur de musique intégré avec lecture de musique locale et visualisation spectrale.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### Et plus encore

- Mises à jour automatiques intégrées et notifications de mise à jour des versions de jeu
- Téléchargements multi-thread avec limites de connexions et débit ajustables ; support des serveurs miroirs et proxy système
- Recherche globale (`/`) et palette de commandes (`;`)
- Suivi du temps de jeu et calendrier d'activité

## Téléchargement

Téléchargez l'installateur pour votre plateforme sur [GitHub Releases](https://github.com/conic-apps/launcher/releases) :

| Plateforme | Architecture          | Formats                       |
| ---------- | --------------------- | ----------------------------- |
| Windows    | x64 · arm64           | MSI · installateur NSIS · exe portable |
| macOS      | Apple Silicon · Intel | DMG                           |
| Linux      | x64 · arm64           | deb · rpm · AppImage          |

L'application intègre des mises à jour automatiques — aucune mise à niveau manuelle n'est nécessaire après l'installation.

## Compilation depuis les sources

Assurez-vous d'avoir [Rust 1.88+](https://rustup.rs/), [Node.js 24](https://nodejs.org/) et [pnpm](https://pnpm.io/) installés, et suivez les [prérequis Tauri](https://tauri.app/start/prerequisites/) pour votre plateforme.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # Développement et débogage
pnpm tauri build    # Build de production
```

## Contribuer

Les Issues et Pull Requests (ciblant la branche `dev`) sont les bienvenus. Avant de soumettre, veuillez exécuter :

```bash
pnpm check
```

## Architecture

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="Diagramme d'architecture : le frontend Vue 3 communique avec le noyau Rust via Tauri IPC, avec des modules crates séparés par domaine en dessous">
</div>

Le frontend (Vue 3 + Pinia) communique avec les fonctionnalités Rust via Tauri IPC. Le `core` gère l'assemblage de l'application Tauri, tandis que les fonctionnalités spécifiques sont réparties en modules `crates/*` du workspace, organisés par domaine, qui évoluent indépendamment et se combinent selon les besoins.

## Licence

Ce projet est distribué sous [GPL-3.0](../../LICENSE) avec des conditions supplémentaires selon la section 7 du GPLv3 :

1. Lors de la distribution de versions modifiées, vous devez modifier raisonnablement le nom du logiciel ou le numéro de version pour le distinguer de l'original (conformément à [GPL-3.0 §7(c)](../../LICENSE)) ; vous devez remplacer toutes les références au nom du projet dans le code source.
2. Vous ne devez pas supprimer les avis de copyright affichés dans le logiciel (conformément à [GPL-3.0 §7(b)](../../LICENSE)).
3. Si quelqu'un entre dans des conditions contractuelles avec les destinataires et assume la responsabilité, le concédant et les auteurs ne sont pas tenus solidairement responsables (conformément à [GPL-3.0 §7(b)](../../LICENSE)).

## Remerciements

- [Catppuccin](https://catppuccin.com) — Palette de thèmes pastel magnifique
- [BMCLAPI](https://bmclapi2.bangbang93.com/) et [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) par bangbang93 — Accélération des téléchargements et amorçage de l'installation Forge
- [MCIM](https://mod.mcimirror.top) — Résumés traduits CurseForge et cache miroir
- Tous les joueurs qui ont fourni des suggestions et des retours

## Avertissement

Conic Launcher n'est pas un produit Minecraft officiel et n'est ni approuvé ni affilié à Mojang Studios. « Minecraft » est une marque déposée de Mojang AB. Toute utilisation de la marque Minecraft dans ce projet est conforme aux <a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Directives de marque et d'actifs</a> de Mojang Studios.
