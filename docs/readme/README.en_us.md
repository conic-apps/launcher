<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher —— a modern, cross-platform Minecraft launcher showcasing four Catppuccin themes">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>A small, fast, and nimble Minecraft launcher</strong><br>
  Multi-instance management · Mod & resource pack marketplace · Cross-LAN multiplayer · Multi-theme support</p>
  <p>
    🌐 <a href="../../README.md"><strong>English</strong></a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## What is this

Conic Launcher is a modern desktop Minecraft launcher: core logic implemented in Rust, UI built with Tauri 2 + Vue 3. It features a small installation footprint, fast startup, low resource consumption, and even helps reduce carbon emissions.

From creating instances and installing loaders, to searching for mods and inviting friends to play together, to launching the game and tracking playtime — the entire workflow can be completed in a single app.

> The project is currently in early Alpha. Features and UI are rapidly iterating — feel free to try it out and report issues.

## Features

### Multi-instance Management

Browse instances grouped by mod loader, with sorting, search, and favorites support. Each instance has its own settings, custom background, playtime tracking, and last-played timestamp at a glance.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Support for All Major Loaders

Choose your Minecraft version and loader version — the launcher handles the rest. Vanilla, Forge, NeoForge, Fabric, and Quilt are all supported, with real-time installation progress.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Mod & Resource Pack Marketplace

Built-in Modrinth and CurseForge search: browse project details and dependencies, read translated summaries, and install mods or resource packs into your instances with one click. You can also manage locally installed mods directly.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus Cross-LAN Multiplayer

No extra tools needed: create a group, share the invite code with friends, and join the same world together. The launcher detects your network environment (NAT type) and provides optimization suggestions.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Multiple Login Methods

Supports Microsoft account login (device code flow), offline login, and any Authlib-Injector / Yggdrasil external authentication server. Accounts feature 3D skin preview, cape display, and skin uploading.

<img width="840" height="533" alt="Screenshot 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Hassle-free Launch Experience

Automatically scans for Java on your system, or can download a suitable runtime automatically. Memory is auto-allocated (with 32-bit Java cap protection). The launch process is fully visualized: downloading game files, installing loaders, resolving dependencies, and waiting for the process to start.

Advanced users are not forgotten either — JVM garbage collector, JVM arguments, game arguments, classpath, wrapper command, and pre/post launch commands are all customizable.

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Saves & Game Data

Browse save world maps, manage data packs and resource packs, and view game screenshots — all within the launcher.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### UI & Personalization

Four Catppuccin themes (Mocha · Macchiato · Frappé · Latte), plus high-contrast variants, with automatic light/dark mode switching following your system setting. Custom launcher backgrounds are supported. Built-in music player with local music playback and spectrum visualization.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### And More

- In-app auto-updates and game version update notifications
- Multi-threaded downloads with adjustable connection limits and speed caps; mirror server and system proxy support
- Global search (`/`) and command palette (`;`)
- Playtime tracking and activity calendar

## Download

Download the installer for your platform from [GitHub Releases](https://github.com/conic-apps/launcher/releases):

| Platform | Architecture          | Formats                       |
| -------- | --------------------- | ----------------------------- |
| Windows  | x64 · arm64           | MSI · NSIS installer · Portable exe |
| macOS    | Apple Silicon · Intel | DMG                           |
| Linux    | x64 · arm64           | deb · rpm · AppImage          |

The app includes built-in auto-updates — no manual upgrades needed after installation.

## Build from Source

Make sure you have [Rust 1.88+](https://rustup.rs/), [Node.js 24](https://nodejs.org/), and [pnpm](https://pnpm.io/) installed, and follow the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your platform.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # Development & debugging
pnpm tauri build    # Production build
```

## Contributing

Issues and Pull Requests (targeting the `dev` branch) are welcome. Before submitting, please run:

```bash
pnpm check
```

## Architecture

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="Architecture diagram: Vue 3 frontend communicates with Rust core via Tauri IPC, with domain-split crates below">
</div>

The frontend (Vue 3 + Pinia) communicates with Rust capabilities via Tauri IPC. The `core` handles Tauri app assembly, while specific capabilities are split into domain-focused `crates/*` workspace modules that evolve independently and compose as needed.

## License

This project is distributed under [GPL-3.0](../../LICENSE) with additional terms under GPLv3 Section 7:

1. When distributing modified versions, you must reasonably change the software name or version number to distinguish it from the original (per [GPL-3.0 §7(c)](../../LICENSE)); you need to replace all project-name references in the source code.
2. You must not remove copyright notices displayed in the software (per [GPL-3.0 §7(b)](../../LICENSE)).
3. If anyone enters contractual terms with recipients and assumes liability, the licensor and authors are not held jointly liable (per [GPL-3.0 §7(b)](../../LICENSE)).

## Acknowledgements

- [Catppuccin](https://catppuccin.com) — Beautiful, pastel theme palette
- [BMCLAPI](https://bmclapi2.bangbang93.com/) and [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) by bangbang93 — Download acceleration and Forge install bootstrapping
- [MCIM](https://mod.mcimirror.top) — CurseForge translated summaries and mirror caching
- All players who have provided suggestions and feedback

## Disclaimer

Conic Launcher is not an official Minecraft product and is not approved by or associated with Mojang Studios. "Minecraft" is a trademark of Mojang AB. Any use of the Minecraft brand in this project complies with Mojang Studios' <a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Brand and Asset Guidelines</a>.
