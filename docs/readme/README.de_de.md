<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher —— ein moderner, plattformübergreifender Minecraft-Launcher, der vier Catppuccin-Themen zeigt">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>Ein kleiner, schneller und wendiger Minecraft-Launcher</strong><br>
  Multi-Instanz-Management · Mod- & Resourcepack-Marktplatz · Cross-LAN-Mehrspieler · Multi-Theme-Unterstützung</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md"><strong>Deutsch</strong></a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Was ist das

Conic Launcher ist ein moderner Desktop-Minecraft-Launcher: Die Kernlogik ist in Rust implementiert, die Oberfläche basiert auf Tauri 2 + Vue 3. Er zeichnet sich durch kleine Installationsgröße, schnellen Start und geringen Ressourcenverbrauch aus und hilft sogar, den CO2-Ausstoß zu reduzieren.

Vom Erstellen von Instanzen und Installieren von Loadern über das Suchen nach Mods bis hin zum Einladen von Freunden zum gemeinsamen Spielen, Starten des Spiels und Verfolgen der Spielzeit — der gesamte Workflow kann in einer einzigen App abgeschlossen werden.

> Das Projekt befindet sich derzeit in einer frühen Alpha-Phase. Funktionen und Oberfläche entwickeln sich schnell weiter — probieren Sie es gerne aus und melden Sie Probleme.

## Funktionen

### Multi-Instanz-Management

Durchstöbern Sie Instanzen, nach Mod-Loadern gruppiert, mit Sortierung, Suche und Favoriten. Jede Instanz hat eigene Einstellungen, einen individuellen Hintergrund, Spielzeitverfolgung und einen Zeitstempel der letzten Ausführung.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Unterstützung aller wichtigen Loader

Wählen Sie Ihre Minecraft-Version und Loader-Version — der Launcher erledigt den Rest. Vanilla, Forge, NeoForge, Fabric und Quilt werden alle unterstützt, mit Echtzeit-Installationsfortschritt.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Mod- & Resourcepack-Marktplatz

Integrierte Modrinth- und CurseForge-Suche: Durchstöbern Sie Projektdetails und Abhängigkeiten, lesen Sie übersetzte Zusammenfassungen und installieren Sie Mods oder Resourcepacks mit einem Klick in Ihre Instanzen. Lokal installierte Mods können ebenfalls direkt verwaltet werden.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Conic Nexus Cross-LAN-Mehrspieler

Keine zusätzlichen Tools nötig: Erstellen Sie eine Gruppe, teilen Sie den Einladungscode mit Freunden und treten Sie gemeinsam derselben Welt bei. Der Launcher erkennt Ihre Netzwerkumgebung (NAT-Typ) und gibt Optimierungsvorschläge.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Mehrere Anmeldemethoden

Unterstützt Microsoft-Kontoanmeldung (Gerätecode-Flow), Offline-Anmeldung und jeden Authlib-Injector / Yggdrasil externen Authentifizierungsserver. Konten bieten 3D-Skin-Vorschau, Umhang-Anzeige und Skin-Upload.

<img width="840" height="533" alt="Screenshot 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Müheloses Start-Erlebnis

Scannen Sie automatisch nach Java auf Ihrem System oder laden Sie eine passende Runtime automatisch herunter. Der Speicher wird automatisch zugewiesen (mit 32-Bit-Java-Obergrenze). Der Startvollzug wird vollständig visualisiert: Herunterladen von Spieldateien, Installieren von Loadern, Auflösen von Abhängigkeiten und Warten auf den Prozessstart.

Auch fortgeschrittene Spieler werden nicht vergessen — JVM-Garbage-Collector, JVM-Argumente, Spielargumente, Klassenpfad, Wrapper-Befehl und Befehle vor und nach dem Start sind alle anpassbar.

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Spielstände & Spieldaten

Durchstöbern Sie Spielstand-Weltkarten, verwalten Sie Datensätze und Resourcepacks und sehen Sie Spielcreenshots direkt im Launcher.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Oberfläche & Personalisierung

Vier Catppuccin-Themen (Mocha · Macchiato · Frappé · Latte) plus hohe Kontrast-Varianten mit automatischem Hell/Dunkel-Wechsel basierend auf Ihren Systemeinstellungen. Individuelle Launcher-Hintergründe werden unterstützt. Integrierter Musikplayer mit lokaler Musikwiedergabe und Spektrumvisualisierung.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### Und mehr

- In-App-Auto-Updates und Spielversions-Update-Benachrichtigungen
- Mehrthread-Downloads mit anpassbaren Verbindungslimits und Geschwindigkeitsbegrenzung; Mirror-Server- und Systemproxy-Unterstützung
- Globale Suche (`/`) und Befehlspalette (`;`)
- Spielzeitverfolgung und Aktivitätskalender

## Herunterladen

Laden Sie den Installer für Ihre Plattform von [GitHub Releases](https://github.com/conic-apps/launcher/releases) herunter:

| Plattform | Architektur            | Formate                        |
| --------- | ---------------------- | ------------------------------ |
| Windows   | x64 · arm64            | MSI · NSIS-Installer · Portables exe |
| macOS     | Apple Silicon · Intel  | DMG                            |
| Linux     | x64 · arm64            | deb · rpm · AppImage           |

Die App verfügt über integrierte Auto-Updates — nach der Installation sind keine manuellen Upgrades erforderlich.

## Aus dem Quellcode erstellen

Stellen Sie sicher, dass Sie [Rust 1.88+](https://rustup.rs/), [Node.js 24](https://nodejs.org/) und [pnpm](https://pnpm.io/) installiert haben, und bereiten Sie die plattformspezifischen Abhängigkeiten gemäß der [Tauri-Dokumentation](https://tauri.app/start/prerequisites/) vor.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # Entwicklung & Debugging
pnpm tauri build    # Produktionsbuild
```

## Beiträge

[Issues](https://github.com/conic-apps/launcher/issues/new/choose) und Pull Requests (Richtung `dev`-Branch) sind willkommen. Bitte führen Sie vor dem Einreichen aus:

```bash
pnpm check
```

## Architektur

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="Architekturdiagramm: Vue 3 Frontend kommuniziert über Tauri IPC mit dem Rust-Kern, darunter domänenspezifische crates-Module">
</div>

Das Frontend (Vue 3 + Pinia) kommuniziert über Tauri IPC mit den Rust-Fähigkeiten. Das `core`-Modul übernimmt die Tauri-App-Zusammenstellung, während spezifische Funktionen in domänenspezifischen `crates/*`-Workspace-Modulen aufgeteilt sind, die unabhängig weiterentwickelt und nach Bedarf kombiniert werden.

## Lizenz

Dieses Projekt wird unter [GPL-3.0](../../LICENSE) mit zusätzlichen Bedingungen gemäß GPLv3 Abschnitt 7 verteilt:

1. Bei Verteilung modifizierter Versionen müssen Sie den Softwarenamen oder die Versionsnummer vernünftig ändern, um sie von der Originalversion zu unterscheiden (gemäß [GPL-3.0 §7(c)](../../LICENSE)); Sie müssen alle Projektname-Referenzen im Quellcode ersetzen.
2. Sie dürfen urheberrechtliche Hinweise in der Software nicht entfernen (gemäß [GPL-3.0 §7(b)](../../LICENSE)).
3. Wenn jemand vertragliche Bedingungen mit Empfängern eingeht und Haftung übernimmt, haften Lizenzgeber und Autoren nicht gemeinsam (gemäß [GPL-3.0 §7(b)](../../LICENSE)).

## Danksagungen

- [Catppuccin](https://catppuccin.com) — Wunderschöne Pastell-Farbpalette
- [BMCLAPI](https://bmclapi2.bangbang93.com/) und [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) von bangbang93 — Download-Beschleunigung und Forge-Installations-Bootstrapping
- [MCIM](https://mod.mcimirror.top) — CurseForge-Übersetzungen und Mirror-Caching
- Alle Spieler, die Vorschläge und Feedback gegeben haben

## Haftungsausschluss

Conic Launcher ist kein offizielles Minecraft-Produkt und wurde nicht von Mojang Studios genehmigt oder unterstützt. „Minecraft" ist ein eingetragenes Trademark von Mojang AB. Jede Verwendung der Marke Minecraft in diesem Projekt entspricht den <a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Marken- und Asset-Richtlinien</a> von Mojang Studios.
