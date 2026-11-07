# The Conic Launcher Architecture

## Data Location

The launcher resolves three storage roots at startup — the launcher's own data, the shared Minecraft
install and the instances — under `storage::LOCATIONS`. By default they hang off one root, with the
launcher data at the root itself and `minecraft/` and `instances/` under it. The root is platform
and packaging dependent: `~/Library/Application Support/app.conicmc.launcher` on macOS,
`$XDG_DATA_HOME/conic` (falling back to `~/.local/share/conic`) on Linux, and `%APPDATA%\conic` on
Windows — except a single-file portable Windows build, which keeps everything in `.minecraft-conic`
next to the executable. Debug builds append `-debug` to the folder name.

`Settings → General → Data storage` redirects each root. That choice cannot live in `config.toml`,
because the config file is inside the launcher location, so it is written to a `locations.toml`
bootstrap file at the platform anchor and applied on the next start.

A first run (`Config.setup_completed` is `false`) opens the setup wizard. Its storage step, right
after the language step, offers the same three roots; finishing writes the config into the chosen
launcher directory, moves the data there and restarts, so a mid-wizard location change does not lose
the steps before it. Closing the wizard keeps the defaults and marks it done.

Delete the debug folder by hand to reset a debug install.

## Tools

Python tooling in `tools/` is not part of any CI gate:

| Script                                   | Does                                                                                      |
| ---------------------------------------- | ----------------------------------------------------------------------------------------- |
| `generate-icns.py`                       | writes `packaging/macos/conic-launcher.icns` (committed)                                  |
| `generate-ico.py`                        | writes `packaging/windows/conic-launcher.ico` (committed)                                 |
| `generate-icons.py`                      | writes `packaging/linux/icons/` (committed; see [Packaging](./CONTRIBUTING.md#packaging)) |
| `check-icon-names.py`                    | every `AppIcon`/`icon:` name in a `.slint` has an SVG                                     |
| `update-i18n.py`                         | re-keys the `.po` catalogues against `slint-tr-extractor`                                 |
| `merge-digit-font.py`, `verify_merge.py` | build and verify the merged font                                                          |
| `render-dmg-background.py`               | renders `packaging/macos/dmg/background.png` from its `.svg` (committed)                  |
| `dmg-ds-store.py`                        | writes the disk image's `.DS_Store`; runs inside `package-macos.sh`                       |

## Directory

- **`app/`** — the application crate (`conic-launcher`), the binary everything ships. `app/src/` is
  layered — see _App layering_ below. The Slint-bound Markdown/HTML renderer lives at
  `app/src/ui/components/markdown/` (its `.slint` half at `ui/components/markdown/`): `comrak` +
  `parley`/`fontique` render a content panel's body and push a display list into the Slint model,
  and the companion views draw it. It sits in `app/` rather than under `crates/` because it is not
  independent of the UI.
- **`ui/`** — the Slint interface crate (`conic-ui`). `ui/**.slint` is the whole interface and
  `ui/build.rs` compiles it into the generated Rust module that `app` re-exports as `slint_backend`,
  generating `ui/icons.slint` from the SVGs under `ui/assets/icons/` (gitignored). The crate also
  owns the interface's assets — `ui/assets/`, which holds the icons, artwork, skins and the two
  embedded fonts — and the translation catalogues, and it is separate
  from `app` so a change to the app's Rust does not recompile the generated interface.
- **`crates/*/`** — Rust domain crates, one per capability, independent of the UI. The interface
  talks to them directly; there is no IPC layer.
- **`packaging/`** — see [Packaging](./CONTRIBUTING.md#packaging).
- **`ui/i18n/<locale>/LC_MESSAGES/conic-ui.po`** — 12 locales. `tools/update-i18n.py` owns
  them; do not hand-edit.

## App layering (`app/src/`)

Three layers, one direction: `main.rs` → `ui/` → `usecases/` → `crates/*`, with `support/` usable by
any of them.

- **`support/`** — process and OS plumbing with no UI counterpart: `runtime`, `logs`, `json`,
  `formatting`, and `native/` (the winit backend hook, the macOS traffic lights and Dock icon, the
  Windows caption). `native/` is the window-shell driver and the one support module that touches
  `slint_backend`.
- **`usecases/`** — the app's use cases: UI-neutral orchestration over the domain crates. Nothing
  here mentions Slint, `slint_backend` or `ui/`. A use case reports progress through a
  `shared::Sink<T>` port and staleness through `usecases::generation::{Gate, Token}`.
  `usecases::tasks::TaskCenter` is the registry of running work: it owns task ids, the cancel
  policy and the completion notices, while `usecases::progress` flattens the download/install
  progress vocabularies into one task stage.
- **`ui/`** — the Slint adapter, and the only layer that touches `slint_backend`. It **mirrors the
  interface crate bucket for bucket**: `app/src/ui/views/<surface>` ↔ `ui/views/…`,
  `app/src/ui/overlays/<surface>` ↔ `ui/overlays/…`, `app/src/ui/components/<component>` ↔
  `ui/components/…`, plus `app/src/ui/services/` for the cross-surface concerns (`report` — delivery
  onto the event loop; `app_config` — the `AppConfig` adapter and locale; `scroll`; `notifications` —
  the notification model and its action table; `tasks` — the task model, click targets and cancel
  confirmation; `sessions` — the running-session fan-out). The mirror is by surface/component, not per `.slint`
  file, and `ui/components/` does not repeat the `controls`/`display`/… role split.

`main.rs` is the composition root: it loads the config, seeds `AppConfig`, and wires every surface's
`setup`. A background task never touches the UI; it reports through
`crate::ui::services::report::{report, deliver}` (or a crate's `Sink`, whose implementation lives in
a `ui/` port).

## UI layout (`ui/`)

`app.slint` is the root `Window`, `theme.slint` the design tokens and `icons.slint` a `build.rs`
product. Everything else is one of four buckets, and the bucket — not the feature — is what decides
where a file goes:

- **`components/`** — reusable, feature-agnostic widgets, split by role:
  - `controls/` — form and display primitives, with **no prefix**: `Button`, `Checkbox`, `Input`,
    `Select`, `DropdownSelect`, `Switch`, `SliderBar`, `ListItem`, `Progress`, `Loading`.
  - `containers/` — layout/behaviour wrappers: `ScrollView`, `ScrollViewHorizontal`,
    `SlideTransition`, `ZoomTransition`.
  - `display/` — presentational: `AppIcon`, `AccountAvatar`, `InstanceCard`, `PaletteRow`,
    `WindowBackground`.
  - `settings/` — `SettingItem`, `SettingGroup`, `SettingCollapse`, `SettingReveal`.
  - `markdown/` — the `.slint` half of `app/src/ui/components/markdown/`.
  - `title-bar.slint` plus a `title-bar/` folder of its private children (`navigation-button`,
    `title-bar-action-button`, `search-bar`). A component gets a same-named folder only when it has
    private children, and keeps them there rather than in `components/`.
- **`views/`** — the top-level pages: one `*-view.slint` each (`game-view`, `launch-view`,
  `setup-view`, `todo-placeholder`) with a same-named folder for its parts (`game/`, `setup/`);
  `views/settings/` holds the section bodies (`settings-general`, `settings-jvm`, …) that the
  settings drawer and the setup wizard compose.
- **`overlays/`** — everything drawn above a page. The app-root layers (`dialog-root`,
  `dropdown-overlay`, `instance-menu`, `description-tooltip`, `command-palette`, `music-player`,
  `settings-overlay`, `instance-settings`) sit here; a dialog _flow_ gets a folder
  (`dialogs/account-add/`, `dialogs/create-instance/`, `dialogs/multiplayer/`) with
  `dialogs/dialog.slint` as the shared modal shell, the content browser is `content/`, and the news
  browser — a list flow over the Mojang news feed and the Java changelogs, with an in-panel
  changelog reader — is `news/`.
- **`globals/`** — one singleton per domain (`Navigation`, `AppConfig`, `ContentState`,
  `PageMotion`, `SettingsShell`, `NotificationState`, `TaskState`, …). Slint components are only
  reachable from Rust through a global, and a global is also how cross-screen state is shared
  without prop drilling; the owning `app/src/ui/…` surface wires each one, and the ones no single
  surface owns (`scroll`, `tasks`) live in `app/src/ui/services/`.

### Settings placement

Settings is one drawer, `overlays/settings-overlay.slint`, over the game view; there is no settings
page and no page/nav/content component split. The section list, the scroll-spy, the scrolling body
and the section imports all live in that file. The drawer is declared before the other app-root
panels, which is why news, music and notifications cover it, while the settings controls' dropdowns
(the `z: 1000` layer) stay above it.

The drawer is available at every window width. Its sidebar is fixed at 208px — with trimmed
padding, the narrowest width that still fits the longest translated section label — while the body
scales with the window as a clamped three fifths (520–680px), leaving at least 80px of game visible.
That keeps the body usable at the 840px minimum window without stretching the settings into a
full-screen sheet on a large display. `globals/settings-shell.slint` owns only the open flag and the
page mirror: the title bar's gear toggles it, opening it from another page first navigates home, and
any later navigation away from the game closes it. The home button drops its selected state while
the drawer is open.

The drawer's two parts animate independently: each starts with its right edge on the window's left
edge and at zero opacity, and settles at its own target (the sidebar at 0, the content after it).
The game view behind it is dimmed by the notification centre's 35% scrim, and the content's base
colour and 24px shadow live on a panel behind the scrolling body, so the body can stay transparent
and keep its own clipping. A fast live resize is kept out of the way by the background: the GPU
framebuffer and the sky keep their old size and stretch until the window has been still for 350ms,
instead of recreating a full-size framebuffer on every drag frame.

## Task centre

The notification centre also shows running work. A task is not a notice: it is live, cancellable and
removed when it ends, while a notice is the record of something that already happened. The two share
one panel but not a model.

The centre is a **scheduler**, not a registry. `submit` hands it a description and the work
(`TaskWork`, a `Box<dyn FnOnce(TaskHandle)>`); nothing starts until the centre admits the task, which
it decides by one rule:

- **Lane** (`TaskKind::lane`) — the mutex. `GameInstall` and `ModpackInstall` share one exclusive lane
  of capacity 1, so no two of them ever run at once; content and library downloads sit in a parallel
  lane and run beside it. This is the one place that says what may overlap.

The queue is FIFO and scanned, not read from the front, so a task waiting for the exclusive lane does
not hold up the parallel download submitted after it. `started_at` is stamped at admission, which is
why the ten-second confirmation grace measures the run and not the wait.

A launch is **not** a task. It has a screen and a cancel button, so a card in the queue would be a
second, worse copy of the same thing. It waits for the centre to go idle and then takes the lane
(`TaskCenter::reserve_exclusive`), holding it for as long as the flow runs; dropping the guard admits
whatever queued in the meantime. That keeps the invariant the lane exists for — no install under a
launching instance — without scheduling the launch.

The launch crate's `running` registry is separate again: a session outlives the flow that started
it, so `ui/services/sessions.rs` owns the app's **one** subscription (`running::subscribe` keeps a
single slot) and fans the events out to the crash overlay and the game view. A second `subscribe`
would silently replace the first.

- `crates/shared/src/task.rs` — the UI-neutral contract: ids, kinds (with their lane), states,
  stages, progress, targets, `TaskEvent` and the `TaskSink`/`TaskCancel` ports.
- `app/src/usecases/tasks.rs` — `TaskCenter`: `submit`, the admission scan (`drain`), the reservation
  for work it does not schedule, `update`/`retarget`, `succeed`/`fail` and `request_cancel`, which
  asks after ten seconds unless the task has not started at all.
- `app/src/usecases/progress.rs` — the one mapping from `DownloadSnapshot` and `InstallProgress` to
  `TaskProgress`/`TaskStage`.
- `app/src/ui/services/tasks.rs` — the Slint adapter. It projects tasks into `TaskState`, mirrors a
  content task's progress into an open detail panel, implements the card's targets, turns a
  completed task's target into the action behind its “Click here to view” notice, and notifies its
  observers so a surface can re-derive its own state after every event. New tasks also get a
  five-second top-right preview while the centre keeps the ongoing card.
- `ui/globals/tasks.slint` and `ui/overlays/notifications.slint` — the “Active tasks” section above
  the notices, its 4px bottom progress strip, the queued card, and
  `ui/overlays/dialogs/confirm-cancel-task.slint` for the ten-second confirmation.

Tasks are runtime-only and are not persisted across restarts. Self-update deliberately stays out of
the section: it is a silent startup check, not work the user started.

### Install and launch are separate

Installing is not a step inside a launch. The game view's button is the whole story — a **Download**
when the instance has no `.install.lock`, a **Launch** when it does, **Cancel** while an install is
queued or running, and the purple **Stop** while the instance's game process is alive (behind the
`ui/overlays/dialogs/confirm-stop-game.slint` warning, because a stop loses unsaved progress):

- `app/src/ui/views/game/install.rs` turns an instance id into a `GameInstall` task and mirrors its
  progress into the strip beside the button (`install_progress.rs`, which is the launch screen's old
  install panel). It is also the repair path: instance settings drops the lock and submits the same
  task, which then only installs.
- `app/src/ui/views/launch/wiring.rs`'s `request` is the only way to start a launch. It opens the
  page at once, shows "Waiting for other tasks to finish" while the centre is busy, and starts
  `usecases::launch::run` when `reserve_exclusive` hands it the lane. The wait lives on the launch
  page because that is where the player can call it off. Leaving the page cancels the launch through
  `page_left`, which `main`'s one `Navigation.page-changed` handler calls — a Slint callback keeps a
  single handler, so a surface that registers `on_page_changed` for itself is replaced by the next
  one and its teardown silently never runs. A cancel also stops a process the flow has already
  spawned (`LaunchController::cancel`); it only stops sessions whose pid was not running when the
  request was made, so leaving the page never kills an older game of the same instance. The page's
  cancel button is therefore never disabled.
- `views/launch/ports.rs` runs on a worker thread: it may only reach the UI through `deliver`/`report`.
  The controller is a thread-local, so a terminal report settles it on the event loop
  (`settle_on_event_loop`) — settling in the sink itself panics there, which drops the terminal report
  and leaks the lane reservation.
- The startup wait in `crates/launch` ends on a start marker, or on the process itself exiting with
  no marker (`running::ExitWatch`), or on its twenty-second timeout. An exit without a marker is
  `Error::FailedBeforeStart`, which `usecases/launch.rs` reports as `Finished`: back to the instance
  list, no error page, no quit, and the crash notice carries the reason.
- `usecases/launch.rs` refuses an uninstalled instance rather than installing it.
- `GameState.current-task`/`task-id` are the projection the button reads, re-derived from the centre
  after every task event, so the UI never keeps a second copy of the scheduler's state.
  `GameState.current-running` is re-derived from the session registry after every session event and
  turns the button into Stop; the right-click menu's `running` comes from the same query.

Naming, once a file is placed: `*View` is a page, `*Overlay`/`*Dialog`/`*Panel`/ `*Card` is a
surface, `*Root` is an overlay layer host. There is **no `base-` family** — the control itself is
the base. Imports are relative paths, so moving a file means updating every importer; `cargo check`
compiles the `.slint` tree and is what catches a missed one.

## Platform plumbing worth knowing

- `crates/platform` — OS detection (`PLATFORM_INFO`, `OsFamily`).
- `crates/storage` — the three storage roots (`LOCATIONS`), which resolve the launcher data, the
  shared Minecraft install and the instances from the platform default plus the `locations.toml`
  bootstrap file.
- `crates/window` — window operations (minimize/maximize/fullscreen, `bring_to_front`) and the winit
  event-filter fan-out. macOS gets a transparent title bar with the real traffic lights; Windows
  gets `with_decorations(false)` plus `app/src/support/native/windows/caption.rs` drawing the
  controls itself; Linux draws them in the title bar.
- `crates/single-instance` — the second launch is not a second window. Linux uses D-Bus (`zbus`),
  macOS a socket, Windows a named mutex.
- `crates/account` — the account model and its flows; the Microsoft browser flow's loopback listener
  (the `authcode` module) lives here too, next to the login it serves.
- `crates/music` — local music listing **and** playback. `symphonia` decodes and `cpal` plays
  natively.

## Crate map

| Crate             | Purpose                                                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------------------------------- |
| `account`         | Microsoft / offline / Authlib / Yggdrasil accounts, and the Microsoft login loopback callback listener (`authcode`) |
| `config`          | App config load/save, background image                                                                              |
| `content`         | Saves, datapacks, resourcepacks, screenshots, mods                                                                  |
| `curseforge`      | CurseForge API client (`CURSEFORGE_API_KEY` at build time)                                                          |
| `download`        | Generic downloader tasks                                                                                            |
| `storage`         | Data directory layout (`LOCATIONS`)                                                                                 |
| `install`         | Minecraft + loader installation                                                                                     |
| `instance`        | Instance CRUD, playtime                                                                                             |
| `java-discovery`  | Java discovery/parsing                                                                                              |
| `launch`          | Game launch with progress reporting                                                                                 |
| `modrinth`        | Modrinth API client                                                                                                 |
| `modpack`         | CurseForge modpack import: manifest/file resolution, overrides, instance creation and install orchestration         |
| `multiplayer`     | Conic Nexus cross-LAN multiplayer                                                                                   |
| `music`           | Local music files + playback                                                                                        |
| `news`            | Mojang launcher-content client: the news feed and the Java changelogs                                               |
| `platform`        | OS detection                                                                                                        |
| `shared`          | Common types/utilities (the shared `HTTP_CLIENT`, the `Sink` ports, the notification `Notice` request and the task contract) |
| `single-instance` | One instance per machine                                                                                            |
| `tilemap`         | World-map tile source port (`TileSource`) and its plain data                                                        |
| `statistics`      | Playtime statistics                                                                                                 |
| `version`         | Minecraft version metadata                                                                                          |
| `window`          | Window operations and the winit backend hook                                                                        |
