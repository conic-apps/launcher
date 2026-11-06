<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — a modern, cross-platform Minecraft launcher showcasing four Catppuccin themes">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>A small, fast, and nimble Minecraft launcher</strong></p>
  <p>
    🌐 <a href="../../README.md"><strong>English</strong></a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## What is this

Conic Launcher is a small, fast, and nimble desktop Minecraft launcher, implemented in Rust, with an
interface built on [Slint](https://slint.dev). Small footprint, fast startup, low resource usage — it
even helps reduce carbon emissions.

> The project is currently in early Alpha. Features and UI are iterating rapidly — feel free to try
> it out and report issues.

## Features

### Multi-instance Management

Browse instances grouped by mod loader, with sorting, search, and favorites; each instance can have
its own special settings.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Support for All Major Loaders

Full support for Forge, NeoForge, Fabric, and Quilt.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Mod & Resource Pack Marketplace

Built-in Modrinth and CurseForge search: browse project details and dependencies inside the
launcher, install a mod or resource pack into an instance with one click, and manage locally
installed mods directly.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus Cross-LAN Multiplayer

No extra tools needed: create a group, send the invite code to a friend, and join the same world
together. The launcher detects your network environment (NAT type) and suggests improvements.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Multiple Login Methods

Supports Microsoft account login (device code flow), offline login, and any Authlib-Injector /
Yggdrasil external authentication server.

<img width="840" height="533" alt="Screenshot 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Hassle-free Launch Experience

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Saves & Game Data

Browse save world maps, manage data packs and resource packs, and view game screenshots, all inside
the launcher.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### UI & Personalization

Four Catppuccin themes (Mocha · Macchiato · Frappé · Latte), plus high-contrast variants; custom
launcher backgrounds and background music are supported.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## Download

Download the installer for your platform from
[GitHub Releases](https://github.com/conic-apps/launcher/releases) or the
[official website](https://conicmc.app):

| Platform | Architecture          | Formats              |
| -------- | --------------------- | -------------------- |
| Windows  | x64 · arm64           | MSI · Portable exe   |
| macOS    | Apple Silicon · Intel | DMG                  |
| Linux    | x64 · arm64           | deb · rpm · AppImage |

> Requires macOS 11.0 (Big Sur) or later.

> If you use the single-file build on Windows, make sure a valid `VCRUNTIME140.dll` exists on your
> system.

## Build from Source

Make sure [Rust 1.88+](https://rustup.rs/) is installed; on Linux you also need the
[build dependencies](../../CONTRIBUTING.md#linux-build-dependencies).

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # Development & debugging
cargo build --release  # Production build
```

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](../../CONTRIBUTING.md) to learn how to get
started, set up your development environment, and submit a pull request.

## License

This project is distributed under [GPL-3.0](../../LICENSE), together with all the additional terms
under GPLv3 Section 7.

## Acknowledgements

- [Catppuccin](https://catppuccin.com) — soft, beautiful theme colors
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) by
  bangbang93 — Forge install bootstrapping
- [MCIM](https://mod.mcimirror.top) — CurseForge Chinese translations and mirror caching
- [HMCL](https://github.com/HMCL-dev/HMCL) — crash handling
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — reference for the local mod parsing
  flow

## Disclaimer

Conic Launcher is not an official Minecraft product and is not approved by or associated with Mojang
Studios. "Minecraft" is a trademark of Mojang AB. Any use of the Minecraft brand in this project
complies with Mojang Studios'
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Brand and Asset
Guidelines</a>.
