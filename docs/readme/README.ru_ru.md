<div align="center">
  <img src="../../docs/screenshots/hero.png" width="100%" alt="Conic Launcher — современный кросс-платформенный лаунчер Minecraft, показывающий четыре темы Catppuccin">
  <p>
    <a href="https://github.com/conic-apps/launcher/actions/workflows/build.yml"><img src="https://img.shields.io/github/actions/workflow/status/conic-apps/launcher/build.yml?label=build&logo=github" alt="Build status"></a>
    <a href="https://github.com/conic-apps/launcher/releases"><img src="https://img.shields.io/github/v/release/conic-apps/launcher?include_prereleases&label=release" alt="Release"></a>
    <a href="../../LICENSE"><img src="https://img.shields.io/github/license/conic-apps/launcher?label=license" alt="License"></a>
    <a href="https://discord.gg/xWKY5NMuf7"><img src="https://img.shields.io/badge/Discord-5865F2?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
  <p><strong>Маленький, быстрый и лёгкий лаунчер Minecraft</strong></p>
  <p>
    🌐 <a href="../../README.md">English</a> · <a href="./README.zh_cn.md">简体中文</a> · <a href="./README.zh_tw.md">繁體中文</a> · <a href="./README.ja_jp.md">日本語</a> · <a href="./README.ko_kr.md">한국어</a> · <a href="./README.de_de.md">Deutsch</a> · <a href="./README.fr_fr.md">Français</a> · <a href="./README.es_es.md">Español</a> · <a href="./README.pt_br.md">Português (Brasil)</a> · <a href="./README.ru_ru.md"><strong>Русский</strong></a> · <a href="./README.tr_tr.md">Türkçe</a> · <a href="./README.pl_pl.md">Polski</a>
  </p>
</div>

---

## Что это такое

Conic Launcher — маленький, быстрый и лёгкий лаунчер Minecraft для настольных систем, реализованный
на Rust, с интерфейсом на [Slint](https://slint.dev). Небольшой размер установки, быстрый запуск,
низкое потребление ресурсов — он даже помогает сокращать выбросы CO₂.

> Проект находится на ранней стадии Alpha: функции и интерфейс быстро меняются. Попробуйте его и
> сообщайте о проблемах.

## Возможности

### Управление несколькими инстансами

Просматривайте инстансы, сгруппированные по загрузчику модов, с сортировкой, поиском и избранным; у
каждого инстанса могут быть свои особые настройки.

https://github.com/user-attachments/assets/ad4678ee-8462-4e55-89f6-99aea41107cb

### Поддержка всех основных загрузчиков

Полная поддержка Forge, NeoForge, Fabric и Quilt.

https://github.com/user-attachments/assets/73d28bc7-e743-4d16-b126-7b997eae797e

### Маркет модов и ресурс-паков

Встроенный поиск по Modrinth и CurseForge: смотрите сведения о проектах и зависимостях прямо в
лаунчере, устанавливайте моды и ресурс-паки в инстанс одним щелчком и управляйте локально
установленными модами напрямую.

https://github.com/user-attachments/assets/78f9e850-473e-4587-92b4-c9bed78afb17

### Сетевое взаимодействие Conic Nexus

Дополнительные инструменты не нужны: создайте группу, отправьте код приглашения другу — и вы вместе
войдёте в один мир. Лаунчер определяет параметры сети (тип NAT) и подсказывает, как их улучшить.

<img width="840" height="533" alt="Conic Nexus" src="https://github.com/user-attachments/assets/e689a0ed-ee06-4495-b963-22eb7371671b" />

### Несколько способов входа

Поддерживаются вход через Microsoft (поток с кодом устройства), офлайн-вход и любой внешний сервер
аутентификации Authlib-Injector / Yggdrasil.

<img width="840" height="533" alt="Скриншот 2026-08-24 16 30 02" src="https://github.com/user-attachments/assets/1bbcba1a-1c9b-4cd0-9984-b4acbb8e3a5d" />

### Беззаботный запуск

https://github.com/user-attachments/assets/1a6943f9-4e35-4391-9984-799cf42ee49d

### Сохранения и игровые данные

Просматривайте карты сохранённых миров, управляйте наборами данных и ресурс-паками и смотрите
скриншоты игры — всё прямо в лаунчере.

https://github.com/user-attachments/assets/195afb69-8c9f-4d83-aa40-50e7b4d61788

### Интерфейс и персонализация

Четыре темы Catppuccin (Mocha · Macchiato · Frappé · Latte) и контрастные варианты; можно задать
собственный фон лаунчера и фоновую музыку.

https://github.com/user-attachments/assets/5a262ff9-2ba8-45d4-b326-38f5d3911a80

https://github.com/user-attachments/assets/2847ea70-35e0-4ecc-9055-777f7545a260

## Скачать

Скачайте установщик для своей платформы на
[GitHub Releases](https://github.com/conic-apps/launcher/releases) или с
[официального сайта](https://conicmc.app):

| Платформа | Архитектура           | Форматы               |
| --------- | --------------------- | --------------------- |
| Windows   | x64 · arm64           | MSI · портативный exe |
| macOS     | Apple Silicon · Intel | DMG                   |
| Linux     | x64 · arm64           | deb · rpm · AppImage  |

> Требуется macOS 11.0 (Big Sur) или новее.

> Если в Windows вы используете однофайловую версию, убедитесь, что в системе есть рабочий
> `VCRUNTIME140.dll`.

## Сборка из исходного кода

Убедитесь, что установлен [Rust 1.88+](https://rustup.rs/); в Linux также нужны
[зависимости для сборки](../../CONTRIBUTING.md#linux-build-dependencies).

```bash
git clone https://github.com/conic-apps/launcher.git
cd launcher
cargo run              # Разработка и отладка
cargo build --release  # Сборка для распространения
```

## Участие

Мы рады вашему участию! Прочитайте [CONTRIBUTING.md](../../CONTRIBUTING.md), чтобы узнать, как
начать, настроить окружение разработки и отправить pull request.

## Лицензия

Проект распространяется под [GPL-3.0](../../LICENSE) вместе со всеми дополнительными условиями
раздела 7 GPLv3.

## Благодарности

- [Catppuccin](https://catppuccin.com) — мягкая и красивая палитра тем
- [forge-install-bootstrapper](https://github.com/bangbang93/forge-install-bootstrapper) от
  bangbang93 — начальная загрузка установки Forge
- [MCIM](https://mod.mcimirror.top) — китайские переводы CurseForge и кэш зеркала
- [HMCL](https://github.com/HMCL-dev/HMCL) — обработка сбоев
- [XMCL](https://github.com/Voxelum/x-minecraft-launcher) — ориентир для разбора локальных модов

## Отказ от ответственности

Conic Launcher не является официальным продуктом Minecraft и не одобрен Mojang Studios и не связан с
ними. «Minecraft» — товарный знак Mojang AB. Любое использование бренда Minecraft в этом проекте
соответствует
<a href="https://www.minecraft.net/en-us/terms#terms-brand_guidelines">правилам использования бренда
и ресурсов</a> Mojang Studios.
