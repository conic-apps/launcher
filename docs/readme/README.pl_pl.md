<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — nowoczesny, wieloplatformowy launcher Minecraft prezentujący cztery motywy Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>Mały, szybki i zwinny launcher Minecraft</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md"><strong>Polski</strong></a>
  </p>
</div>

---

## Co to jest

Conic Launcher to mały, szybki i zwinny launcher Minecrafta na komputery, zaimplementowany w Rust,
z interfejsem zbudowanym na [Slint](https://slint.dev). Niewielka instalacja, szybkie uruchamianie,
niskie zużycie zasobów — pomaga nawet ograniczać emisję CO₂.

> Projekt jest na wczesnym etapie Alpha; funkcje i interfejs szybko się zmieniają. Wypróbuj go i
> zgłaszaj problemy.

## Funkcjonalności

### Zarządzanie wieloma instancjami

Przeglądaj instancje pogrupowane według loadera modów, z sortowaniem, wyszukiwaniem i ulubionymi;
każda instancja może mieć własne, specjalne ustawienia.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Wsparcie dla wszystkich głównych loaderów

Pełne wsparcie dla Forge, NeoForge, Fabric i Quilt.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Rynek modów i paczek zasobów

Wbudowane wyszukiwanie Modrinth i CurseForge: przeglądaj szczegóły projektów i zależności w
launcherze, instaluj mod lub paczkę zasobów w instancji jednym kliknięciem i zarządzaj lokalnie
zainstalowanymi modami bezpośrednio.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Rozgrywka sieciowa Conic Nexus

Dodatkowe narzędzia nie są potrzebne: utwórz grupę, wyślij kod zaproszenia znajomemu i wejdźcie razem
do tego samego świata. Launcher wykrywa środowisko sieciowe (typ NAT) i sugeruje ulepszenia.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Wiele metod logowania

Obsługuje logowanie przez konto Microsoft (przepływ z kodem urządzenia), logowanie offline oraz
dowolny zewnętrzny serwer uwierzytelniania Authlib-Injector / Yggdrasil.

<img width="840" height="533" alt="Zrzut ekranu 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Beztroskie uruchamianie

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Zapisy i dane gry

Przeglądaj mapy zapisanych światów, zarządzaj paczkami danych i paczkami zasobów, oglądaj zrzuty
ekranu z gry — wszystko w launcherze.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Interfejs i personalizacja

Cztery motywy Catppuccin (Mocha · Macchiato · Frappé · Latte) oraz warianty o wysokim kontraście;
można ustawić własne tło launchera i muzykę w tle.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## Pobieranie

Pobierz instalator dla swojej platformy z
[GitHub Releases](https://github.com/conic-apps/launcher/releases) lub
[oficjalnej strony](https://conicmc.app):

| Platforma | Architektura          | Formaty              |
| --------- | --------------------- | -------------------- |
| Windows   | x64 · arm64           | MSI · przenośny exe  |
| macOS     | Apple Silicon · Intel | DMG                  |
| Linux     | x64 · arm64           | deb · rpm · AppImage |

> Wymaga macOS 11.0 (Big Sur) lub nowszego.

> Jeśli w systemie Windows używasz wersji jednoplikowej, upewnij się, że w systemie znajduje się
> prawidłowy plik `VCRUNTIME140.dll`.

## Budowanie ze źródeł

Upewnij się, że zainstalowano [Rust 1.88+](https://rustup.rs/); w systemie Linux potrzebne są także
[zależności budowania](../../CONTRIBUTING.md#linux-build-dependencies).

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # Programowanie i debugowanie
cargo build --release  # Kompilacja produkcyjna
```

## Wkład

Zapraszamy do współtworzenia! Przeczytaj [CONTRIBUTING.md](../../CONTRIBUTING.md), aby dowiedzieć się,
jak zacząć, skonfigurować środowisko programistyczne i wysłać pull request.

## Licencja

Projekt jest rozpowszechniany na licencji [GPL-3.0](../../LICENSE) wraz ze wszystkimi dodatkowymi
warunkami z sekcji 7 GPLv3.

## Podziękowania

- [Catppuccin](https://catppuccin.com) — miękkie, piękne kolory motywów
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) autorstwa
  bangbang93 — bootstrap instalacji Forge
- [MCIM](https://mod.mcimirror.top) — chińskie tłumaczenia CurseForge i pamięć podręczna serwera
  lustrzanego
- [HMCL](https://github.com/HMCL-dev/HMCL) — obsługa awarii
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — odniesienie dla analizy lokalnych modów

## Zastrzeżenie

Conic Launcher nie jest oficjalnym produktem Minecraft i nie jest zatwierdzony przez Mojang Studios ani
z nim powiązany. „Minecraft” jest znakiem towarowym Mojang AB. Wszelkie użycie marki Minecraft w tym
projekcie jest zgodne z
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">wytycznymi dotyczącymi marki i
zasobów</a> Mojang Studios.
