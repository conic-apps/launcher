# The Conic Launcher Architecture

## Architecture map

Two relations carry the app's own layout: the three layers under `app/src/` read one way, and
`app/src/ui/…` mirrors `ui/…` bucket for bucket.

```text
      +------------------------------------------------------------------+
      | app/src/main.rs      composition root                            |
      | loads the config, seeds AppConfig, wires every surface's setup   |
      +------------------------------------------------------------------+
                                 |
                                 | constructs
                                 v
      +------------------------------------------------------------------+
      | app/src/ui/          the Slint adapter                           |
      | screens/  overlays/  components/  services/                      |
      | the only layer that names slint_backend                          |
      +------------------------------------------------------------------+
                                 |
                                 | calls
                                 v
      +------------------------------------------------------------------+
      | app/src/usecases/    UI-neutral orchestration                    |
      | launch  tasks (TaskCenter)  progress  generation::{Gate, Token}  |
      | no file here mentions Slint, slint_backend or ui/                |
      +------------------------------------------------------------------+
                                 |
                                 | calls
                                 v
      +------------------------------------------------------------------+
      | crates/*             domain crates                               |
      | one capability each, independent of the UI                       |
      +------------------------------------------------------------------+
```

`app/src/support/` sits beside the chain rather than on it: `runtime`, `logs`, `json`, `formatting`
and `native/` are usable from any layer, and `native/` is the only one of them that touches
`slint_backend`.

```text
   app/src/ui/               ui/                         bucket
   ----------------------   --------------------------  --------------------------------------
   screens/<screen>      --> screens/<screen>.slint      the navigation destinations
   overlays/<surface>    --> overlays/<surface>.slint    above a screen: dialogs/, news/, settings/
   components/<widget>   --> components/<widget>.slint   widgets, grouped by role
   services/<concern>    --> globals/<domain>.slint      cross-surface state and delivery
```

- The arrow into `ui/` lands on a `globals/` singleton or on a property/callback of the surface
  being wired: a widget is reachable from Rust no other way.
- The way back is the report path, not a call. Work off the event loop reaches the UI through
  `ui::services::report::{report, deliver}`, or through a crate's `Sink`, whose implementation is a
  `ui/` port.

## Crate map

| Crate             | Purpose                                                                                                                      |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `account`         | Microsoft / offline / Authlib / Yggdrasil accounts, and the Microsoft login loopback callback listener (`authcode`)          |
| `config`          | App config load/save, background image                                                                                       |
| `content`         | Saves, datapacks, resourcepacks, screenshots, mods                                                                           |
| `curseforge`      | CurseForge API client (`CURSEFORGE_API_KEY` at build time)                                                                   |
| `download`        | Generic downloader tasks                                                                                                     |
| `storage`         | Data directory layout (`LOCATIONS`)                                                                                          |
| `install`         | Minecraft + loader installation                                                                                              |
| `instance`        | Instance CRUD, playtime                                                                                                      |
| `java-discovery`  | Java discovery/parsing                                                                                                       |
| `launch`          | Game launch with progress reporting                                                                                          |
| `modrinth`        | Modrinth API client                                                                                                          |
| `instance-import` | Importing an instance from a modpack archive or another launcher: sources, file resolution, copy and install                 |
| `multiplayer`     | Conic Nexus cross-LAN multiplayer                                                                                            |
| `music`           | Local music files + playback                                                                                                 |
| `news`            | Mojang launcher-content client: the news feed and the Java changelogs                                                        |
| `platform`        | OS detection                                                                                                                 |
| `shared`          | Common types/utilities (the shared `HTTP_CLIENT`, the `Sink` ports, the notification `Notice` request and the task contract) |
| `single-instance` | One instance per machine                                                                                                     |
| `tilemap`         | World-map tile source port (`TileSource`) and its plain data                                                                 |
| `statistics`      | Playtime statistics                                                                                                          |
| `version`         | Minecraft version metadata                                                                                                   |
| `window`          | Window operations and the winit backend hook                                                                                 |

## Data Location

The launcher resolves three storage roots at startup — the launcher's own data, the shared Minecraft
install and the instances — under `storage::LOCATIONS`.

The root is platform and packaging dependent: `~/Library/Application Support/app.conicmc.launcher`
on macOS, `$XDG_DATA_HOME/conic` (falling back to `~/.local/share/conic`) on Linux, and
`%APPDATA%\conic` on Windows — except a single-file portable Windows build, which keeps everything
in `.minecraft-conic` next to the executable. Debug builds append `-debug` to the folder name.

`Settings → General → Data storage` redirects each root. That choice is written to a
`locations.toml` bootstrap file at the platform anchor and applied on each start.

Inside the launcher location, `config.toml` is the settings and `states/` is the reconstructible
session state — the selected instance, the saved track, the version reminder, the staged-update
records. `states/` may be deleted at any time to get the defaults back; everything else there
(`accounts/`, `cache/`, `logs/`, `music/`, `native/`, `favorites.json`, `statistics.json`) is data
the user would miss.

A first run (`Config.setup_completed` is `false`) opens the setup wizard. Its storage step, right
after the language step, offers the same three roots; finishing writes the config into the chosen
launcher directory, moves the data there and restarts, so a mid-wizard location change does not lose
the steps before it. Closing the wizard keeps the defaults and marks it done.

Delete the debug folder by hand to reset a debug install.

## Directory

- **`app/`** — the application crate (`conic-launcher`), the binary everything ships. Its layers and
  the Markdown/HTML renderer are in [`app/README.md`](./app/README.md).
- **`ui/`** — the Slint interface crate (`conic-ui`). `ui/**.slint` is the whole interface and
  `ui/build.rs` compiles it into the generated Rust module that `app` re-exports as `slint_backend`,
  generating `ui/icons.slint` from the SVGs under `ui/assets/icons/` (gitignored). The crate also
  owns the interface's assets — `ui/assets/`, which holds the icons, artwork, skins and the two
  embedded fonts — and the translation catalogues, and it is separate from `app` so a change to the
  app's Rust does not recompile the generated interface. The bucket layout is in
  [`ui/README.md`](./ui/README.md).
- **`crates/*/`** — Rust domain crates, one per capability, independent of the UI. The interface
  talks to them directly; there is no IPC layer.
- **`packaging/`** — see [`packaging/README.md`](./packaging/README.md).
- **`ui/i18n/<locale>/LC_MESSAGES/conic-ui.po`** — 12 locales. `tools/update-i18n.py` owns them; do
  not hand-edit.

## Task centre

The notification centre also shows running work. A task is not a notice: it is live, cancellable and
removed when it ends, while a notice is the record of something that already happened. The two share
one panel but not a model.

The centre is a **scheduler**, not a registry. `submit` hands it a description and the work
(`TaskWork`, a `Box<dyn FnOnce(TaskHandle)>`); nothing starts until the centre admits the task,
which it decides by one rule:

- **Lane** (`TaskKind::lane`) — the mutex. `GameInstall` and `ModpackInstall` share one exclusive
  lane of capacity 1, so no two of them ever run at once; content and library downloads sit in a
  parallel lane and run beside it. This is the one place that says what may overlap.

The queue is FIFO and scanned, not read from the front, so a task waiting for the exclusive lane
does not hold up the parallel download submitted after it. `started_at` is stamped at admission,
which is why the ten-second confirmation grace measures the run and not the wait.

A launch is **not** a task. It has a screen and a cancel button, so a card in the queue would be a
second, worse copy of the same thing. It waits for the centre to go idle and then takes the lane
(`TaskCenter::reserve_exclusive`), holding it for as long as the flow runs; dropping the guard
admits whatever queued in the meantime. That keeps the invariant the lane exists for — no install
under a launching instance — without scheduling the launch.

The launch crate's `running` registry is separate again: a session outlives the flow that started
it, so `ui/services/sessions.rs` owns the app's **one** subscription (`running::subscribe` keeps a
single slot) and fans the events out to the crash overlay and the game screen. A second `subscribe`
would silently replace the first.

- `crates/shared/src/task.rs` — the UI-neutral contract: ids, kinds (with their lane), states,
  stages, progress, targets, `TaskEvent` and the `TaskSink`/`TaskCancel` ports.
- `app/src/usecases/tasks.rs` — `TaskCenter`: `submit`, the admission scan (`drain`), the
  reservation for work it does not schedule, `update`/`retarget`, `succeed`/`fail` and
  `request_cancel`, which asks after ten seconds unless the task has not started at all.
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

Installing is not a step inside a launch. The game screen's button is the whole story — a **Download**
when the instance has no `.install.lock`, a **Launch** when it does, **Cancel** while an install is
queued or running, and the purple **Stop** while the instance's game process is alive (behind the
`ui/overlays/dialogs/confirm-stop-game.slint` warning, because a stop loses unsaved progress):

- `app/src/ui/screens/game/install.rs` turns an instance id into a `GameInstall` task and mirrors its
  progress into the strip beside the button (`install_progress.rs`, which is the launch screen's old
  install panel). It is also the repair path: instance settings drops the lock and submits the same
  task, which then only installs.
- `app/src/ui/screens/launch/wiring.rs`'s `request` is the only way to start a launch. It opens the
  screen at once, shows "Waiting for other tasks to finish" while the centre is busy, and starts
  `usecases::launch::run` when `reserve_exclusive` hands it the lane. The wait lives on the launch
  screen because that is where the player can call it off. Leaving the screen cancels the launch
  through `screen_left`, which `main`'s one `Navigation.screen-changed` handler calls — a Slint
  callback keeps a single handler, so a surface that registers `on_screen_changed` for itself is
  replaced by the next one and its teardown silently never runs. A cancel also stops a process the
  flow has already spawned (`LaunchController::cancel`); it only stops sessions whose pid was not
  running when the request was made, so leaving the screen never kills an older game of the same
  instance. The screen's cancel button is therefore never disabled.
- `screens/launch/ports.rs` runs on a worker thread: it may only reach the UI through
  `deliver`/`report`. The controller is a thread-local, so a terminal report settles it on the event
  loop (`settle_on_event_loop`) — settling in the sink itself panics there, which drops the terminal
  report and leaks the lane reservation.
- The startup wait in `crates/launch` ends on a start marker, or on the process itself exiting with
  no marker (`running::ExitWatch`), or on its twenty-second timeout. An exit without a marker is
  `Error::FailedBeforeStart`, which `usecases/launch.rs` reports as `Finished`: back to the instance
  list, no error screen, no quit, and the crash notice carries the reason.
- `usecases/launch.rs` refuses an uninstalled instance rather than installing it.
- `GameState.current-task`/`task-id` are the projection the button reads, re-derived from the centre
  after every task event, so the UI never keeps a second copy of the scheduler's state.
  `GameState.current-running` is re-derived from the session registry after every session event and
  turns the button into Stop; the right-click menu's `running` comes from the same query.

Naming, once a file is placed: `*Screen` is a destination `Navigation` can navigate to, which is
the only thing a screen is; `*View` may name any other drawn thing (an overlay's contents, the
Markdown renderer, a scroller). `*Overlay`/`*Dialog`/`*Panel`/ `*Card` is a surface, `*Root` is an
overlay layer host. There is **no `base-` family** — the control itself is
the base. Imports are relative paths, so moving a file means updating every importer; `cargo check`
compiles the `.slint` tree and is what catches a missed one.

## Instance import

`crates/instance-import` turns something dropped on the window into a new instance: a modpack
archive in one of five pack formats, or another launcher's instance folder. The frontend only has to
word it — `ImportSource::kind()` says which of the two it is.

- `source.rs` resolves where a pack's files are — a zip or an already extracted folder, with the
  pack root at the top or one folder down — and every reader goes through it.
- `pack/` holds the five formats and `launcher/` the instance layouts; each returns the common
  `Metadata` plus its own half, which `resolve` turns into downloads and file trees.
- `import.rs` creates the instance and fills it in one call: the game install, the downloads, then
  the trees. The new id is reported through `Progress::InstanceCreated` before the install starts,
  so the task card can point at it and the instance list can select it; no caller sequences "create"
  before "install". `crates/install` keeps its own meaning — it installs Minecraft, Java and the
  loader into an instance that already exists.
- `files.rs` writes the trees and reports the bytes it copies, so an instance import shows a live
  bar instead of a spinner.

| Source                    | Marker                                                        | Files                                                                                                                         | Copied                                                                                        |
| ------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| CurseForge                | `manifest.json`                                               | `projectID`/`fileID` pairs, resolved through the `curseforge` client                                                          | `overrides/`                                                                                  |
| Modrinth                  | `modrinth.index.json`                                         | the index's own URLs and hashes; a SHA-1 lookup only for an entry whose `downloads` is empty                                  | `overrides/`, then `client-overrides/`                                                        |
| Prism/MultiMC             | `mmc-pack.json`, `instance.cfg`                               | none — the pack carries everything                                                                                            | `.minecraft/`, minus `logs/` and `crash-reports/`                                             |
| MCBBS                     | `mcbbs.packmeta`, or a `manifest.json` with an `addons` array | `addon` entries the archive does not carry come from the pack's `fileApi`; `curse` entries go through the CurseForge resolver | `overrides/`                                                                                  |
| HMCL                      | `modpack.json`                                                | none                                                                                                                          | `minecraft/`, minus `pack.json` and the content the launcher installs itself                  |
| Version-isolated instance | `<folder>/<folder>.json`                                      | none                                                                                                                          | the folder, minus `versions/`, `libraries/`, `assets/`, the logs and the launcher's own files |

A Prism/MultiMC pack is an exported instance, and a live instance folder has the same layout, so
both are the same reader and both are an `Instance` import. `manifest.json` is shared, and a pack
converted from another launcher can keep a stale one: the `addons` array is what makes a manifest
MCBBS rather than CurseForge, and `mmc-pack.json` and `modrinth.index.json` are checked before it.

A source may also carry settings for the instance it becomes — mostly launch settings
(Prism/MultiMC's `instance.cfg` carries `JvmArgs`, `MaxMemAlloc`, the auto-join server and the
window size; MCBBS's `launchInfo` the JVM and game arguments), but nothing stops a source from
naming something else. `settings.rs` keeps them off the format modules, `Plan::settings` carries
them, and the import writes them into the new instance's configuration; a source that asks for
nothing leaves the instance on the launcher's defaults, and one that names a heap turns the
automatic allocation off so its choice survives. Prism/MultiMC's `patches/` and `jarmods/` patch the
game jar itself; they are skipped with a log line rather than half-applied.
