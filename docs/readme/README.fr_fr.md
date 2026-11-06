<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — un lanceur Minecraft moderne et multiplateforme, affichant quatre thèmes Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>Un lanceur Minecraft petit, rapide et léger</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md"><strong>Français</strong></a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Qu'est-ce que c'est

Conic Launcher est un lanceur Minecraft de bureau petit, rapide et léger, implémenté en Rust, avec une
interface construite sur [Slint](https://slint.dev). Installation légère, démarrage rapide, faible
consommation de ressources — il contribue même à réduire les émissions de CO₂.

> Le projet est actuellement en début de phase Alpha ; les fonctionnalités et l'interface évoluent
> rapidement. N'hésitez pas à l'essayer et à signaler les problèmes.

## Fonctionnalités

### Gestion multi-instances

Parcourez les instances regroupées par chargeur de mods, avec tri, recherche et favoris ; chaque
instance peut avoir ses propres réglages spécifiques.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Prise en charge de tous les chargeurs majeurs

Prise en charge complète de Forge, NeoForge, Fabric et Quilt.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Marché de mods et de packs de ressources

Recherche Modrinth et CurseForge intégrée : consultez les détails des projets et leurs dépendances
dans le lanceur, installez un mod ou un pack de ressources dans une instance en un clic et gérez
directement les mods installés localement.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Multijoueur cross-LAN Conic Nexus

Aucun outil supplémentaire n'est nécessaire : créez un groupe, envoyez le code d'invitation à un ami
et rejoignez le même monde ensemble. Le lanceur détecte votre environnement réseau (type de NAT) et
suggère des améliorations.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Plusieurs méthodes de connexion

Prend en charge la connexion Microsoft (flux par code d'appareil), la connexion hors ligne et tout
serveur d'authentification externe Authlib-Injector / Yggdrasil.

<img width="840" height="533" alt="Capture d'écran 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Expérience de lancement sans tracas

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Sauvegardes et données de jeu

Parcourez les cartes des mondes sauvegardés, gérez les datapacks et les packs de ressources et
consultez les captures d'écran du jeu, directement dans le lanceur.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Interface et personnalisation

Quatre thèmes Catppuccin (Mocha · Macchiato · Frappé · Latte), ainsi que des variantes à fort
contraste ; le fond du lanceur et la musique de fond sont personnalisables.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## Téléchargement

Téléchargez l'installateur correspondant à votre plateforme depuis
[GitHub Releases](https://github.com/conic-apps/launcher/releases) ou le
[site officiel](https://conicmc.app) :

| Plateforme | Architecture          | Formats              |
| ---------- | --------------------- | -------------------- |
| Windows    | x64 · arm64           | MSI · exe portable   |
| macOS      | Apple Silicon · Intel | DMG                  |
| Linux      | x64 · arm64           | deb · rpm · AppImage |

> Nécessite macOS 11.0 (Big Sur) ou une version ultérieure.

> Si vous utilisez la version monofichier sous Windows, assurez-vous qu'une `VCRUNTIME140.dll` valide
> est présente sur le système.

## Compilation depuis les sources

Assurez-vous que [Rust 1.88+](https://rustup.rs/) est installé ; sous Linux, les
[dépendances de compilation](../../CONTRIBUTING.md#linux-build-dependencies) sont également
nécessaires.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # Développement et débogage
cargo build --release  # Build de production
```

## Contribuer

Les contributions sont les bienvenues ! Consultez [CONTRIBUTING.md](../../CONTRIBUTING.md) pour savoir
comment démarrer, configurer votre environnement de développement et soumettre une pull request.

## Licence

Ce projet est distribué sous [GPL-3.0](../../LICENSE), avec toutes les clauses supplémentaires prévues
par la section 7 de la GPLv3.

## Remerciements

- [Catppuccin](https://catppuccin.com) — des couleurs de thème douces et harmonieuses
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) par
  bangbang93 — amorçage de l'installation de Forge
- [MCIM](https://mod.mcimirror.top) — traductions chinoises CurseForge et mise en cache miroir
- [HMCL](https://github.com/HMCL-dev/HMCL) — gestion des plantages
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — référence pour l'analyse des mods locaux

## Avertissement

Conic Launcher n'est pas un produit Minecraft officiel et n'est ni approuvé ni associé à Mojang
Studios. « Minecraft » est une marque de Mojang AB. Toute utilisation de la marque Minecraft dans ce
projet respecte les
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">directives de marque et
d'actifs</a> de Mojang Studios.
