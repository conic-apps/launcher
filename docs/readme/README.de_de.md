<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — ein moderner, plattformübergreifender Minecraft-Launcher, der vier Catppuccin-Themen zeigt">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>Ein kleiner, schneller und wendiger Minecraft-Launcher</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md"><strong>Deutsch</strong></a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Was ist das

Conic Launcher ist ein kleiner, schneller und wendiger Minecraft-Launcher für den Desktop, in Rust
implementiert und mit [Slint](https://slint.dev) als Oberfläche. Kleine Downloadgröße, schneller
Start, geringer Ressourcenverbrauch – und er hilft sogar, CO₂-Emissionen zu senken.

> Das Projekt befindet sich in einer frühen Alpha-Phase; Funktionen und Oberfläche entwickeln sich
> schnell weiter. Probieren Sie es aus und melden Sie Probleme.

## Funktionen

### Multi-Instanz-Management

Instanzen nach Mod-Loader gruppiert durchsuchen, mit Sortierung, Suche und Favoriten; jede Instanz
kann eigene spezielle Einstellungen haben.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Unterstützung aller wichtigen Loader

Vollständige Unterstützung für Forge, NeoForge, Fabric und Quilt.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Mod- & Ressourcenpaket-Marktplatz

Integrierte Modrinth- und CurseForge-Suche: Projektdetails und Abhängigkeiten im Launcher ansehen,
Mods oder Ressourcenpakete mit einem Klick in eine Instanz installieren und lokal installierte Mods
direkt verwalten.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus Cross-LAN-Mehrspieler

Keine zusätzlichen Tools nötig: Gruppe erstellen, Einladungscode an Freunde schicken und gemeinsam
dieselbe Welt betreten. Der Launcher erkennt die Netzwerkumgebung (NAT-Typ) und gibt
Verbesserungsvorschläge.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Mehrere Anmeldemethoden

Unterstützt die Microsoft-Anmeldung (Device-Code-Verfahren), Offline-Anmeldung sowie beliebige
Authlib-Injector- / Yggdrasil-Authentifizierungsserver.

<img width="840" height="533" alt="Screenshot 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Müheloses Start-Erlebnis

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Spielstände & Spieldaten

Spielstand-Weltkarten durchsuchen, Datenpakete und Ressourcenpakete verwalten und Screenshots
ansehen – alles direkt im Launcher.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Oberfläche & Personalisierung

Vier Catppuccin-Themes (Mocha · Macchiato · Frappé · Latte) plus kontrastreiche Varianten; eigener
Launcher-Hintergrund und Hintergrundmusik werden unterstützt.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## Herunterladen

Laden Sie das Installationspaket für Ihre Plattform von
[GitHub Releases](https://github.com/conic-apps/launcher/releases) oder der
[offiziellen Website](https://conicmc.app) herunter:

| Plattform | Architektur           | Formate              |
| --------- | --------------------- | -------------------- |
| Windows   | x64 · arm64           | MSI · Portable exe   |
| macOS     | Apple Silicon · Intel | DMG                  |
| Linux     | x64 · arm64           | deb · rpm · AppImage |

> Erfordert macOS 11.0 (Big Sur) oder neuer.

> Wenn Sie unter Windows die Einzeldatei-Version verwenden, stellen Sie sicher, dass eine gültige
> `VCRUNTIME140.dll` im System vorhanden ist.

## Aus dem Quellcode erstellen

Stellen Sie sicher, dass [Rust 1.88+](https://rustup.rs/) installiert ist; unter Linux werden
zusätzlich die [Build-Abhängigkeiten](../../CONTRIBUTING.md#linux-build-dependencies) benötigt.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # Entwicklung & Debugging
cargo build --release  # Produktions-Build
```

## Beiträge

Beiträge sind willkommen! Lesen Sie [CONTRIBUTING.md](../../CONTRIBUTING.md), um zu erfahren, wie Sie
beginnen, Ihre Entwicklungsumgebung einrichten und einen Pull Request einreichen.

## Lizenz

Dieses Projekt wird unter [GPL-3.0](../../LICENSE) verbreitet, zusammen mit allen zusätzlichen
Bedingungen nach GPLv3 Abschnitt 7.

## Danksagungen

- [Catppuccin](https://catppuccin.com) — sanfte, schöne Themenfarben
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) von
  bangbang93 — Bootstrapping der Forge-Installation
- [MCIM](https://mod.mcimirror.top) — chinesische CurseForge-Übersetzungen und Mirror-Caching
- [HMCL](https://github.com/HMCL-dev/HMCL) — Absturzbehandlung
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — Referenz für die lokale Mod-Erkennung

## Haftungsausschluss

Conic Launcher ist kein offizielles Minecraft-Produkt und wird von Mojang Studios weder genehmigt
noch unterstützt. „Minecraft“ ist eine Marke von Mojang AB. Jede Verwendung der Minecraft-Marke in
diesem Projekt entspricht den
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Marken- und
Asset-Richtlinien</a> von Mojang Studios.
