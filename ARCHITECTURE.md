# The Conic Launcher Architecture

## Architecture map

Two relations carry the app's layout: the layers under `app/src/` read one way, and `app/src/ui/…`
mirrors `ui/…` bucket for bucket.

```text
   app/src/main.rs     composition root: loads the config, wires every surface's setup
          |
   app/src/ui/         the Slint adapter — the only layer that names slint_backend
          |
   app/src/usecases/   UI-neutral orchestration — no file here mentions Slint or ui/
          |
   crates/*            domain crates, one capability each, independent of the UI
```

Each layer calls the one below it and nothing else. `app/src/support/` sits beside the chain rather
than on it: `runtime`, `logs`, `json`, `formatting` and `native/` are usable from any layer, and
`native/` is the only one of them that touches `slint_backend`.

```text
   app/src/ui/               ui/                         bucket
   ----------------------   --------------------------  --------------------------------------
   screens/<screen>      --> screens/<screen>.slint      the navigation destinations
   overlays/<surface>    --> overlays/<surface>.slint    above a screen: dialogs/, news/, settings/
   --                    --> graphics/<role>/*.slint     the design system: tokens and widgets
   services/<concern>    --> globals/<domain>.slint      cross-surface state and delivery
```

A surface is wired by reaching a `globals/` singleton or a property/callback of that surface:
`slint_backend` offers no other way in. The way back is the report path, not a call — work off the
event loop reaches the UI through `ui::services::report::{report, deliver}`, or through a crate's
`Sink`, whose implementation is a `ui/` port.

`ui/window/` and `ui/graphics/` are the buckets with no `app/src/ui/` counterpart, and neither is a
surface: the first is the window's own chrome, whose Rust side is `app/src/support/native/` with
`crates/window` behind it, and the second is the design system, reached from Rust only through a
surface that uses it.

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

Three roots are resolved at startup under `storage::LOCATIONS`: the launcher's own data, the shared
Minecraft install and the instances. The root is platform and packaging dependent —
`~/Library/Application Support/app.conicmc.launcher` on macOS, `$XDG_DATA_HOME/conic` (falling back
to `~/.local/share/conic`) on Linux, and `%APPDATA%\conic` on Windows — except a single-file
portable Windows build, which keeps everything in `.minecraft-conic` next to the executable. Debug
builds append `-debug` to the folder name.

`Settings → General → Data storage` redirects each root; the choice is written to a `locations.toml`
bootstrap file at the platform anchor and applied on each start.

Inside the launcher root, `config.toml` is the settings and `states/` the reconstructible session
state — the selected instance, the saved track, the version reminder, the staged-update records.
`states/` may be deleted at any time to get the defaults back; everything else there (`accounts/`,
`cache/`, `logs/`, `music/`, `native/`, `favorites.json`, `statistics.json`) is data the user would
miss.

A first run (`Config.setup_completed` is `false`) opens the setup wizard, whose storage step offers
the same three roots; finishing writes the config into the chosen root, moves the data there and
restarts. Closing the wizard keeps the defaults and marks it done. Delete the debug folder by hand
to reset a debug install.

## Directory

- **`app/`** — the application crate (`conic-launcher`), the binary everything ships. Its layers and
  the Markdown/HTML renderer are in [`app/README.md`](./app/README.md).
- **`ui/`** — the Slint interface crate (`conic-ui`): `ui/**.slint`, the assets and the twelve `.po`
  catalogues. `ui/build.rs` compiles the interface into the Rust module `app` re-exports as
  `slint_backend`, and the crate is separate from `app` so a change to the app's Rust does not
  recompile it. The bucket layout is in [`ui/README.md`](./ui/README.md).
- **`crates/*/`** — Rust domain crates, one per capability, independent of the UI. The interface
  talks to them directly; there is no IPC layer.
- **`packaging/`** — see [`packaging/README.md`](./packaging/README.md).
- **`ui/i18n/<locale>/LC_MESSAGES/conic-ui.po`** — owned by `tools/update-i18n.py`; do not
  hand-edit.

## Task centre

The notification centre also shows running work. A task is not a notice: it is live, cancellable and
removed when it ends, while a notice is the record of something that already happened. The two share
one panel but not a model.

The centre is a **scheduler**, not a registry. `submit` hands it a description and the work; nothing
starts until the centre admits the task, which one rule decides: the **lane** (`TaskKind::lane`).
`GameInstall` and `ModpackInstall` share one exclusive lane of capacity 1, so no two of them ever
run at once, while content and library downloads run in a parallel lane beside it. The queue is FIFO
and scanned rather than read from the front, so a task waiting for the exclusive lane does not hold
up a download submitted after it, and `started_at` is stamped at admission, so the ten-second
confirmation grace measures the run and not the wait. Tasks are runtime-only: they are not persisted
across restarts, and self-update stays out of the section because it is a silent startup check.

A launch is **not** a task. It has a screen and a cancel button, so a card in the queue would be a
second, worse copy of the same thing. It waits for the centre to go idle and then takes the lane
(`TaskCenter::reserve_exclusive`) for as long as the flow runs, which keeps the invariant the lane
exists for — no install under a launching instance — without scheduling the launch. Sessions are
separate again: a game outlives the flow that started it, so `ui/services/sessions.rs` owns the
app's **one** subscription (`running::subscribe` keeps a single slot) and fans the events out to the
crash overlay and the game screen; a second `subscribe` would silently replace the first.

- `crates/shared/src/task.rs` — the UI-neutral contract: ids, kinds (with their lane), states,
  stages, progress, targets, `TaskEvent` and the `TaskSink`/`TaskCancel` ports.
- `app/src/usecases/tasks.rs` — `TaskCenter`: `submit`, the admission scan, the reservation for work
  it does not schedule, `update`/`retarget`, `succeed`/`fail` and `request_cancel`, which asks after
  ten seconds unless the task has not started at all.
- `app/src/usecases/progress.rs` — the one mapping from `DownloadSnapshot` and `InstallProgress` to
  `TaskProgress`/`TaskStage`.
- `app/src/ui/services/tasks.rs` — the Slint adapter: it projects tasks into `TaskState`, mirrors a
  content task's progress into an open detail panel, implements the card's targets, turns a
  completed task's target into the action behind its "Click here to view" notice, and notifies its
  observers so a surface can re-derive its own state after every event. New tasks also get a
  five-second top-right preview while the centre keeps the ongoing card.
- `ui/globals/tasks.slint`, `ui/overlays/notifications.slint` and
  `ui/overlays/dialogs/confirm-cancel-task.slint` — the "Active tasks" section above the notices,
  its 4px bottom progress strip, the queued card and the ten-second confirmation.

### Install and launch are separate

Installing is not a step inside a launch. The game screen's button is the whole story — a
**Download** when the instance has no `.install.lock`, a **Launch** when it does, **Cancel** while
an install is queued or running, and the purple **Stop** while the instance's game process is alive
(behind `confirm-stop-game.slint`, because a stop loses unsaved progress):

- `screens/game/install.rs` turns an instance id into a `GameInstall` task and mirrors its progress
  into the strip beside the button (`install_progress.rs`). It is also the repair path: instance
  settings drops the lock and submits the same task, which then only installs.
- `screens/launch/wiring.rs`'s `request` is the only way to start a launch. It opens the screen at
  once, shows "Waiting for other tasks to finish" while the centre is busy, and starts
  `usecases::launch::run` when `reserve_exclusive` hands it the lane. Leaving the screen cancels it
  through `screen_left`, which `main`'s one `Navigation.screen-changed` handler calls — a Slint
  callback keeps a single handler, so a surface that registers its own is silently replaced. A
  cancel also stops a process the flow has already spawned, but only sessions whose pid was not
  running when the request was made, so leaving the screen never kills an older game of the same
  instance.
- `screens/launch/ports.rs` runs on a worker thread and reaches the UI only through
  `deliver`/`report`. The controller is a thread-local, so a terminal report settles it on the event
  loop; settling it in the sink itself panics there and leaks the lane reservation.
- The startup wait ends on a start marker, on the process exiting without one
  (`Error::FailedBeforeStart`, reported as `Finished`: back to the instance list, no error screen,
  no quit, the crash notice carries the reason), or on its twenty-second timeout.
- `usecases/launch.rs` refuses an uninstalled instance rather than installing it.
- `GameState.current-task`/`task-id` and `current-running` are the projections the button reads,
  re-derived from the centre and the session registry after every event, so the UI never keeps a
  second copy of state.
