<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — nowoczesny, wieloplatformowy launcher Minecraft prezentujący cztery motywy Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-89b4fa" alt="Windows · macOS · Linux">
  </p>
  <p><strong>Mały, szybki i zwinny launcher Minecraft</strong><br>
  Zarządzanie wieloma instancjami · Rynek modów i paczek zasobów · Rozgrywka sieciowa cross-LAN · Wsparcie wielu motywów</p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md">Русский</a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md"><strong>Polski</strong></a>
  </p>
</div>

---

## Co to jest

Conic Launcher to nowoczesny launcher Minecraft na komputery: logika główna jest zaimplementowana w Rust, interfejs zbudowany na Tauri 2 + Vue 3. Wyróżnia się małą instalacją, szybkim uruchamianiem i niskim zużyciem zasobów, nawet pomagając zmniejszyć emisję dwutlenku węgla.

Od tworzenia instancji i instalowania loaderów, przez wyszukiwanie modów i zapraszanie znajomych do wspólnej gry, po uruchamianie gry i śledzenie czasu gry — cały przepływ pracy można ukończyć w jednej aplikacji.

> Projekt jest obecnie we wczesnej fazie alfa. Funkcjonalność i interfejs szybko ewoluują — wypróbuj go i zgłoś problemy.

## Funkcjonalności

### Zarządzanie wieloma instancjami

Przeglądaj instancje pogrupowane według loadera modów, z sortowaniem, wyszukiwaniem i ulubionymi. Każda instancja ma własne ustawienia, niestandardowe tło, śledzenie czasu gry i znacznik czasu ostatniego uruchomienia.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Wsparcie dla wszystkich głównych loaderów

Wybierz wersję Minecraft i wersję loadera — launcher zajmie się resztą. Obsługiwane są Vanilla, Forge, NeoForge, Fabric i Quilt, z postępem instalacji w czasie rzeczywistym.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Rynek modów i paczek zasobów

Wbudowane wyszukiwanie Modrinth i CurseForge: przeglądaj szczegóły projektów i zależności, czytaj przetłumaczone opisy i instaluj mody lub paczki zasobów do swoich instancji jednym kliknięciem. Możesz również zarządzać lokalnie zainstalowanymi modami bezpośrednio.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Rozgrywka sieciowa Conic Nexus

Nie są potrzebne żadne dodatkowe narzędzia: utwórz grupę, udostępnij kod zaproszenia znajomym i dołącz do tego samego świata razem. Launcher wykrywa środowisko sieciowe (typ NAT) i sugeruje ulepszenia.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Wiele metod logowania

Obsługuje logowanie kontem Microsoft (przepływ kodu urządzenia), logowanie offline i dowolny zewnętrzny serwer uwierzytelniania Authlib-Injector / Yggdrasil. Konta oferują podgląd 3D skórek, wyświetlanie peleryn i przesyłanie skórek.

<img width="840" height="533" alt="Zrzut ekranu 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Beztroskie uruchamianie

Automatyczne skanowanie Java w systemie lub automatyczne pobieranie odpowiedniego runtime. Pamięć jest automatycznie przydzielana (z ochroną limitu 32-bitowej Java). Proces uruchamiania jest w pełni wizualizowany: pobieranie plików gry, instalacja loaderów, rozwiązywanie zależności i oczekiwanie na uruchomienie procesu.

Zaawansowani użytkownicy nie są pomijani — garbage collector JVM, argumenty JVM, argumenty gry, classpath, polecenie opakowujące i polecenia przed/po uruchomieniu — wszystko jest konfigurowalne.

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Zapisy i dane gry

Przeglądaj mapy świata zapisów, zarządzaj pakietami danych i paczkami zasobów oraz przeglądaj zrzuty ekranu gry — wszystko bezpośrednio w launcherze.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Interfejs i personalizacja

Cztery motywy Catppuccin (Mocha · Macchiato · Frappé · Latte) oraz warianty o wysokim kontraście, z automatycznym przełączaniem jasny/ciemny zgodnie z ustawieniami systemu. Obsługiwane niestandardowe tła launchera. Wbudowany odtwarzacz muzyki z odtwarzaniem lokalnej muzyki i wizualizacją spektrum.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

### I więcej

- Wbudowane automatyczne aktualizacje i powiadomienia o aktualizacjach wersji gry
- Pobieranie wielowątkowe z konfigurowalnymi limitami połączeń i ograniczeniem prędkości; obsługa serwerów lustrzanych i proxy systemowego
- Wyszukiwanie globalne (`/`) i paleta poleceń (`;`)
- Śledzenie czasu gry i kalendarz aktywności

## Pobieranie

Pobierz instalator dla swojej platformy z [GitHub Releases](https://github.com/conic-apps/launcher/releases):

| Platforma | Architektura            | Formaty                        |
| --------- | ---------------------- | ------------------------------ |
| Windows   | x64 · arm64            | MSI · instalator NSIS · przenośny exe |
| macOS     | Apple Silicon · Intel  | DMG                            |
| Linux     | x64 · arm64            | deb · rpm · AppImage           |

Aplikacja zawiera wbudowane automatyczne aktualizacje — po instalacji nie jest wymagana ręczna aktualizacja.

## Budowanie ze źródeł

Upewnij się, że masz zainstalowane [Rust 1.88+](https://rustup.rs/), [Node.js 24](https://nodejs.org/) i [pnpm](https://pnpm.io/), i postępuj zgodnie z [wymaganiami wstępnymi Tauri](https://tauri.app/start/prerequisites/) dla swojej platformy.

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
pnpm install
pnpm tauri dev      # Programowanie i debugowanie
pnpm tauri build    # Build produkcyjny
```

## Wkład

Issues i Pull Requesty (kierowane na gałąź `dev`) są mile widziane. Przed przesłaniem proszę uruchomić:

```bash
pnpm check
```

## Architektura

<div align="center">
  <img src="../../docs/screenshots/architecture.png" width="100%" alt="Schemat architektury: frontend Vue 3 komunikuje się z rdzeniem Rust przez Tauri IPC, z modułami crates podzielonymi według domen">
</div>

Frontend (Vue 3 + Pinia) komunikuje się z funkcjonalnościami Rust przez Tauri IPC. Moduł `core` zajmuje się montażem aplikacji Tauri, podczas gdy konkretne możliwości są podzielone na domenowe moduły `crates/*` workspace'u, które niezależnie ewoluują i łączą się w razie potrzeby.

## Licencja

Ten projekt jest dystrybuowany na licencji [GPL-3.0](../../LICENSE) z dodatkowymi warunkami na mocy sekcji 7 GPLv3:

1. Dystrybuując zmodyfikowane wersje, musisz rozsądnie zmienić nazwę oprogramowania lub numer wersji, aby odróżnić ją od oryginału (zgodnie z [GPL-3.0 §7(c)](../../LICENSE)); musisz zastąpić wszystkie odniesienia do nazwy projektu w kodzie źródłowym.
2. Nie możesz usuwać powiadomień o prawach autorskich wyświetlanych w oprogramowaniu (zgodnie z [GPL-3.0 §7(b)](../../LICENSE)).
3. Jeśli ktoś wejdzie w warunki umowne z odbiorcami i przejmie odpowiedzialność, licencjodawca i autorzy nie ponoszą odpowiedzialności solidarnej (zgodnie z [GPL-3.0 §7(b)](../../LICENSE)).

## Podziękowania

- [Catppuccin](https://catppuccin.com) — Piękna paleta pastelowych motywów
- [BMCLAPI](https://bmclapi2.bangbang93.com/) i [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) autorstwa bangbang93 — Przyspieszanie pobierania i bootstrapping instalacji Forge
- [MCIM](https://mod.mcimirror.top) — Przetłumaczone opisy CurseForge i cache lustrzany
- Wszyscy gracze, którzy zgłaszali sugestie i informacje zwrotne

## Zastrzeżenie

Conic Launcher nie jest oficjalnym produktem Minecraft ani nie jest zatwierdzony lub powiązany z Mojang Studios. "Minecraft" jest znakiem towarowym Mojang AB. Jakiekolwiek użycie marki Minecraft w tym projekcie jest zgodne z <a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">Wytycznymi dotyczącymi marki i zasobów</a> Mojang Studios.
