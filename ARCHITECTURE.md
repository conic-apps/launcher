# The Conic Launcher Architecture

## Data Location

The launcher resolves three storage roots at startup — the launcher's own data,
the shared Minecraft install and the instances — under `storage::LOCATIONS`. By
default they hang off one root, with the launcher data at the root itself and
`minecraft/` and `instances/` under it. The root is platform and packaging
dependent: `~/Library/Application Support/app.conicmc.launcher` on macOS,
`$XDG_DATA_HOME/conic` (falling back to `~/.local/share/conic`) on Linux, and
`%APPDATA%\conic` on Windows — except a single-file portable Windows build, which
keeps everything in `.minecraft-conic` next to the executable. Debug builds
append `-debug` to the folder name.

`Settings → General → Data storage` redirects each root. That choice cannot live
in `config.toml`, because the config file is inside the launcher location, so it
is written to a `locations.toml` bootstrap file at the platform anchor and applied
on the next start.

A first run (`Config.setup_completed` is `false`) opens the setup wizard. Its
storage step, right after the language step, offers the same three roots;
finishing writes the config into the chosen launcher directory, moves the data
there and restarts, so a mid-wizard location change does not lose the steps
before it. Closing the wizard keeps the defaults and marks it done.

Delete the debug folder by hand to reset a debug install.

## Tools

Python tooling in `tools/` is not part of any CI gate:

| Script                                   | Does                                                       |
| ---------------------------------------- | ---------------------------------------------------------- |
| `generate-icns.py`                       | writes `packaging/macos/conic-launcher.icns` (committed)   |
| `generate-ico.py`                        | writes `packaging/windows/conic-launcher.ico` (committed)  |
| `generate-icons.py`                      | writes `packaging/linux/icons/` (committed; see [Packaging](./CONTRIBUTING.md#packaging)) |
| `check-icon-names.py`                    | every `AppIcon`/`icon:` name in a `.slint` has an SVG      |
| `update-i18n.py`                         | re-keys the `.po` catalogues against `slint-tr-extractor`  |
| `merge-digit-font.py`, `verify_merge.py` | build and verify the merged font                           |

## Directory

- **`app/`** — the application crate (`conic-launcher`), the binary everything
  ships. `app/ui/**.slint` is the whole interface and `app/build.rs` compiles it,
  generating `app/ui/icons.slint` from the SVGs under `app/ui/assets/icons/`
  (gitignored). `app/src/` is layered — see *App layering* below. The
  Slint-bound Markdown/HTML renderer lives at `app/src/ui/components/markdown/`
  (its `.slint` half at `app/ui/components/markdown/`): `comrak` +
  `parley`/`fontique` render a content panel's body and push a display list into
  the Slint model, and the companion views draw it. It sits in `app/` rather than
  under `crates/` because it is not independent of the UI.
- **`crates/*/`** — Rust domain crates, one per capability, independent of the
  UI. The interface talks to them directly; there is no IPC layer.
- **`packaging/`** — see [Packaging](./CONTRIBUTING.md#packaging).
- **`app/i18n/<locale>/LC_MESSAGES/conic-launcher.po`** — 12 locales.
  `tools/update-i18n.py` owns them; do not hand-edit.

## App layering (`app/src/`)

Three layers, one direction: `main.rs` → `ui/` → `usecases/` → `crates/*`, with
`support/` usable by any of them.

- **`support/`** — process and OS plumbing with no UI counterpart: `runtime`,
  `logs`, `json`, `formatting`, and `platform/` (the winit backend hook, the
  macOS traffic lights and Dock icon, the Windows caption). `platform/` is the
  window-shell driver and the one support module that touches `slint_backend`.
- **`usecases/`** — the app's use cases: UI-neutral orchestration over the domain
  crates. Nothing here mentions Slint, `slint_backend` or `ui/`. A use case
  reports progress through a `shared::Sink<T>` port and staleness through
  `usecases::generation::{Gate, Token}`.
- **`ui/`** — the Slint adapter, and the only layer that touches `slint_backend`.
  It **mirrors `app/ui/` bucket for bucket**: `ui/views/<surface>` ↔
  `app/ui/views/…`, `ui/overlays/<surface>` ↔ `app/ui/overlays/…`,
  `ui/components/<component>` ↔ `app/ui/components/…`, plus `ui/services/` for
  the cross-surface concerns (`report` — delivery onto the event loop;
  `app_config` — the `AppConfig` adapter and locale; `scroll`). The mirror is by
  surface/component, not per `.slint` file, and `ui/components/` does not repeat
  the `controls`/`display`/… role split.

`main.rs` is the composition root: it loads the config, seeds `AppConfig`, and
wires every surface's `setup`. A background task never touches the UI; it reports
through `crate::ui::services::report::{report, deliver}` (or a crate's `Sink`,
whose implementation lives in a `ui/` port).

## UI layout (`app/ui/`)

`app.slint` is the root `Window`, `theme.slint` the design tokens and
`icons.slint` a `build.rs` product. Everything else is one of four buckets, and
the bucket — not the feature — is what decides where a file goes:

- **`components/`** — reusable, feature-agnostic widgets, split by role:
  - `controls/` — form and display primitives, with **no prefix**: `Button`,
    `Checkbox`, `Input`, `Select`, `DropdownSelect`, `Switch`, `SliderBar`,
    `ListItem`, `Progress`, `Loading`.
  - `containers/` — layout/behaviour wrappers: `ScrollView`,
    `ScrollViewHorizontal`, `SlideTransition`, `ZoomTransition`.
  - `display/` — presentational: `AppIcon`, `AccountAvatar`, `InstanceCard`,
    `PaletteRow`, `WindowBackground`.
  - `settings/` — `SettingItem`, `SettingGroup`, `SettingCollapse`.
  - `markdown/` — the `.slint` half of `app/src/ui/components/markdown/`.
  - `title-bar.slint` plus a `title-bar/` folder of its private children
    (`navigation-button`, `title-bar-action-button`, `search-bar`). A component
    gets a same-named folder only when it has private children, and keeps them
    there rather than in `components/`.
- **`views/`** — the top-level pages: one `*-view.slint` each (`game-view`,
  `settings-view`, `launch-view`, `setup-view`, `todo-placeholder`) with a
  same-named folder for its parts (`game/`, `settings/`, `setup/`).
- **`overlays/`** — everything drawn above a page. The app-root layers
  (`dialog-root`, `dropdown-overlay`, `description-tooltip`, `command-palette`,
  `music-player`, `instance-settings`) sit here; a dialog *flow* gets a folder
  (`dialogs/account-add/`, `dialogs/create-instance/`, `dialogs/multiplayer/`)
  with `dialogs/dialog.slint` as the shared modal shell, and the content browser
  is `content/`.
- **`globals/`** — one singleton per domain (`Navigation`, `AppConfig`,
  `ContentState`, …). Slint components are only reachable from Rust through a
  global, and a global is also how cross-screen state is shared without prop
  drilling; the owning `app/src/ui/…` surface wires each one, and the ones no
  single surface owns (`scroll`) live in `app/src/ui/services/`.

Naming, once a file is placed: `*View` is a page, `*Overlay`/`*Dialog`/`*Panel`/
`*Card` is a surface, `*Root` is an overlay layer host. There is **no `base-`
family** — the control itself is the base. Imports are relative paths, so moving
a file means updating every importer; `cargo check` compiles the `.slint` tree
and is what catches a missed one.

## Platform plumbing worth knowing

- `crates/platform` — OS detection (`PLATFORM_INFO`, `OsFamily`).
- `crates/storage` — the three storage roots (`LOCATIONS`), which resolve the
  launcher data, the shared Minecraft install and the instances from the platform
  default plus the `locations.toml` bootstrap file.
- `crates/window` — window operations (minimize/maximize/fullscreen,
  `bring_to_front`) and the winit event-filter fan-out. macOS gets a
  transparent title bar with the real traffic lights; Windows gets
  `with_decorations(false)` plus `app/src/support/platform/windows/caption.rs` drawing the
  controls itself; Linux draws them in the title bar.
- `crates/single-instance` — the second launch is not a second window. Linux uses
  D-Bus (`zbus`), macOS a socket, Windows a named mutex.
- `crates/account` — the account model and its flows; the Microsoft browser
  flow's loopback listener (the `authcode` module) lives here too, next to the
  login it serves.
- `crates/music` — local music listing **and** playback. `symphonia` decodes and
  `cpal` plays natively.

## Crate map

| Crate             | Purpose                                                    |
| ----------------- | ---------------------------------------------------------- |
| `account`         | Microsoft / offline / Authlib / Yggdrasil accounts, and the Microsoft login loopback callback listener (`authcode`) |
| `config`          | App config load/save, background image                     |
| `content`         | Saves, datapacks, resourcepacks, screenshots, mods         |
| `curseforge`      | CurseForge API client (`CURSEFORGE_API_KEY` at build time) |
| `download`        | Generic downloader tasks                                   |
| `storage`         | Data directory layout (`LOCATIONS`)                       |
| `install`         | Minecraft + loader installation                            |
| `instance`        | Instance CRUD, playtime                                    |
| `java-discovery`  | Java discovery/parsing                                     |
| `launch`          | Game launch with progress reporting                        |
| `modrinth`        | Modrinth API client                                        |
| `multiplayer`     | Conic Nexus cross-LAN multiplayer                          |
| `music`           | Local music files + playback                               |
| `platform`        | OS detection                                               |
| `shared`          | Common types/utilities (the shared `HTTP_CLIENT`)          |
| `single-instance` | One instance per machine                                   |
| `tilemap`         | World-map tile source port (`TileSource`) and its plain data |
| `statistics`      | Playtime statistics                                        |
| `version`         | Minecraft version metadata                                 |
| `window`          | Window operations and the winit backend hook               |
