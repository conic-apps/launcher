# Slint migration

This directory holds the in-progress migration of the Conic Launcher frontend
from Tauri + Vue (`core/`, `src/`) to [Slint](https://slint.dev) 1.18.

The migration strategy is **copy-and-adapt**: UI and logic are gradually copied
into this directory and reworked for Slint. **The original Tauri/Vue code is not
modified**, so both frontends can coexist until the Slint app reaches parity.

> The repository's `AGENTS.md` describes the Tauri/Vue app. This README covers
> the `slint/` tree.

## Layout

```
slint/
  app/                              # the Slint application (binary crate)
    build.rs                        # slint-build: compiles ui/, bundles i18n/
    Cargo.toml                      # package: conic-launcher-slint
    i18n/<lang>/LC_MESSAGES/
      conic-launcher-slint.po       # gettext catalogs (see Internationalization)
    src/
      main.rs                       # Rust host: platform setup, window chrome, locale
      traffic_lights.rs             # macOS: native traffic lights vs. the custom title bar
      runtime.rs                    # the tokio runtime the background work runs on
      config_bridge.rs              # config ↔ UI bridges (file pickers, opening URLs)
      settings.rs                   # the settings screen's script
      game.rs                       # the game view's script
      create_instance.rs            # the create-instance dialog's script
    ui/
      app.slint                     # root `App` Window (mirrors src/App.vue)
      theme.slint                   # palette + typography tokens, embeds fonts
      icons.slint                   # icon path data (mirrors src/assets/icons/*.svg)
      fonts/ComfortaaNunito.ttf      # Comfortaa with Nunito digits merged (see Fonts)
      assets/                       # palette previews, about logos, version icons
      globals/
        navigation.slint            # global page navigation (src/store/navigation.ts)
        settings.slint              # global config state (the "config store")
        game.slint                  # game view state (src/store/instance.ts + …)
        dialogs.slint               # dialog store + the create-instance form state
        focus.slint                 # blur-the-focused-field helper
        window-drag.slint           # the Vue's `data-tauri-drag-region` regions
        scroll.slint                # wheel/trackpad classification + monotonic clock
      components/                   # shared/reusable pieces
        title-bar.slint
        account-avatar.slint
        base-loading.slint
        title-bar/navigation-button.slint
        title-bar/title-bar-action-button.slint
        search-bar.slint
        app-icon.slint
        setting-group.slint
        setting-item.slint
        setting-collapse.slint
        scroll-view.slint
        base-dialog.slint           # modal shell with its scrim/panel animation
        base-list-item.slint        # a row of a list panel
        zoom-transition.slint       # the zoom-in / zoom-out screen transition
        base-switch.slint
        base-select.slint
        base-dropdown-select.slint
        dropdown-overlay.slint
        base-slider-bar.slint
        base-input.slint
        base-button.slint
        item-loading-icon.slint
      views/
        settings-view.slint         # src/views/SettingsView.vue
        settings/                   # the eight settings sections + InfoBox
        game-view.slint             # src/views/GameView.vue
        game/                       # summary, list, toolbar, footer, dropdowns
        game-placeholder.slint      # stand-in for the not-yet-migrated views
      overlays/
        dialog-root.slint           # src/overlays/DialogRoot.vue
        dialogs/
          create-instance.slint     # src/overlays/dialogs/CreateInstance.vue
          create/
            minecraft-choose.slint  #   …/create/MinecraftChoose.vue
            mod-loader-choose.slint #   …/create/ModLoaderChoose.vue
  crates/
    platform/                       # Tauri-free mirror of crates/platform
    window/                         # window-control service (min/max/fullscreen)
    config/                         # Tauri-free mirror of crates/config
    folder/                         # Tauri-free mirror of crates/folder
    account/                        # Tauri-free mirror of crates/account
    instance/                       # Tauri-free mirror of crates/instance
    content/                        # Tauri-free mirror of crates/content (counts)
    install/                        # Tauri-free mirror of crates/install (version lists)
    java-runtime/                   # Tauri-free mirror of crates/java-runtime (the scan)
    single-instance/                # single-instance guard (no Tauri plugin here)
```

## Naming

- **`slint/app`** — the application shell. The root component is `App`
  (`ui/app.slint`), mirroring `src/App.vue`.
- **package `conic-launcher-slint`** — the binary is named
  `conic-launcher-slint` to avoid clashing with the Tauri app's binary. This is
  a temporary name until the Slint app replaces the Tauri one.
- The helper crates are `slint-platform` / `slint-window` (crate folder names
  are prefixed to stay greppable; Rust library names are unprefixed).

## Build & run

```bash
cargo run -p conic-launcher-slint          # dev
cargo build -p conic-launcher-slint        # build
cargo clippy -p conic-launcher-slint --all-targets
cargo fmt -p conic-launcher-slint
```

`build.rs` compiles `ui/app.slint` (which `import`s everything else) and emits
the Rust pulled in by `slint::include_modules!()`. Custom fonts and translations
are embedded/bundled at compile time.

## Fonts

The original stylesheet (`src/assets/styles/main.css`) used:

```css
font-family: "Nunito", "Comfortaa", system-ui, …; /* body */
```

where `Nunito` was a **digits-only subset** (`unicode-range: U+0030-0039`) so
numbers render in Nunito and everything else falls back to Comfortaa, both
variable `.woff2` fonts. Slint has no `unicode-range` fallback, so the digit
glyphs (with their variable-font deltas) are **merged into a new "Comfortaa
Nunito" variable font** instead:

- `slint/tools/merge-digit-font.py` regenerates `ui/fonts/ComfortaaNunito.ttf`
  from `slint/tools/fonts/Comfortaa-Latin.woff2` + `Nunito-Digits.woff2` (the
  original sources, kept next to the scripts): the Nunito digits instanced at
  weight 400 become the base glyphs, gvar tuples cover weights 400–700 (matching
  the Nunito subset outline on every master knot), Comfortaa's HVAR digit rows
  and the GPOS kerning pairs touching digits are zeroed, and everything else in
  the font is untouched — letters are byte-identical to the original Comfortaa
  and digit advances always equal the Nunito 600. The font's `name` records are
  re-tagged as **"Comfortaa Nunito"** (the sources' version/license/copyright
  names are inherited).
- `slint/tools/verify_merge.py` (run as `python3 merge-digit-font.py --verify`)
  checks the digit outlines point-exact against the instanced Nunito glyphs,
  letters against the original Comfortaa, and that no digit kerning remains.

The font is embedded at compile time and imported in `ui/theme.slint`
(`import "fonts/ComfortaaNunito.ttf";`); the root window sets
`default-font-family: Theme.font-family` and `font-family` is the only family —
digits and letters render from the same variable font, so numeric text needs no
extra `font-family` overrides.

Typography tokens live in the `Theme` global (`font-family`, `font-size-base`,
`font-weight-normal/bold`).

## Internationalization

Uses Slint's built-in translation support:

- UI strings are wrapped in `@tr("…")` in `.slint` files.
- Catalogs are gettext `.po` files at
  `app/i18n/<language>/LC_MESSAGES/conic-launcher-slint.po`. The file name must
  match the crate name (the gettext domain is `CARGO_PKG_NAME`).
- `build.rs` bundles them via
  `CompilerConfiguration::with_bundled_translations("i18n")`.
- `config_bridge::select_locale` picks a language at startup with
  `slint::select_bundled_translation(&lang)`: the language saved in the config,
  or the system locale when unset ("follow system"). An unknown code falls back
  to `en_US`.
- Changing the language in Settings → General calls
  `select_bundled_translation` again from the settings `changed` handler; Slint
  re-evaluates every `@tr` binding, so **no restart is needed**.
- `CONIC_LOCALE=fr_FR` forces a locale and disables runtime switching (useful
  for testing).
- The **default translation context is the Slint component name**, so entries
  use e.g. `msgctxt "App"` / `msgctxt "TitleBar"`. Globals can set an explicit
  context with `@tr("GameTime" => "…")`, which `GameTime` uses for the relative
  time strings.

All 12 launcher languages ship a catalog (`en_US` is the fallback):
`zh_CN`, `zh_TW`, `ja_JP`, `ko_KR`, `de_DE`, `fr_FR`, `es_ES`, `pt_BR`, `ru_RU`,
`tr_TR`, `pl_PL`. Catalogs were seeded from the Vue `src/locales/*.ts` settings
and game strings; the `msgid`s are the `@tr()` source strings and the `msgctxt`
is the component name. To add or refresh a language, add/update its directory
under `app/i18n/` (and the `bundled_locale()` mapping in
`src/config_bridge.rs`). Catalogs can be (re)generated with
`slint-tr-extractor` (not currently installed).

The `msgid` of a `@tr()` with more than one argument is the source string as it
is written — `@tr("{}/{}/{}", month, day, year)` is looked up as `"{}/{}/{}"`,
**not** as the extractor's positional `"{0}/{1}/{2}"`. A string the runtime
cannot find falls back to the source, so a catalog entry under the wrong form
looks like a missing translation: the `GameTime` date formats were catalogued
that way and only ever rendered as `9/15/2026`; they now use the source form
(their `msgstr`s stay positional, `"{2}年{0}月{1}日"`, which the formatter
supports) and are translated.

A `@tr()` in a component that is not the one the key belongs to (an inline
`component SettingsStep { … }` inside `create-instance.slint`) resolves under
_that_ component's name, so those call sites set the context explicitly:
`@tr("CreateInstance" => "Version settings")`.

The About disclaimer is the one rich-text string: `StyledText` renders it with
an interpolated markdown link (`@markdown("\{@tr(…)}[\{@tr(…)}](url)…")`), so
only "Brand and Asset Guidelines" is a link. Its sentence is therefore split
into three catalog entries — the text before the link, the link label, and the
text after it (the Vue original embedded the `<a>` in a single `v-html` string).
Slint always underlines `Style::Link` spans and 1.18 has no link-hover state, so
the underline is always visible (it can't be limited to hover).

## State ownership

Per the migration plan:

- **Domain state and logic stay in Rust** (the existing `crates/*`, exposed via
  a thin controller layer) — Rust is the source of truth.
- Slint exposes it through `in-out property` / `callback` bindings; pure-UI
  state may live in Slint `export global`s.
- Vue's Pinia stores and composables are **not** ported 1:1 — their
  responsibilities map to Rust controllers plus small Slint globals.

## Migrated so far

- Application window (`App`) with the custom title bar and native window chrome.
- Title bar: home/settings navigation, centered search bar + hotkey chip, music
  action (`components/title-bar*`, `components/search-bar.slint`).
- Global page navigation (`globals/navigation.slint`) and the `App` page stack.
- The settings screen: `SettingsView` (sidebar + scroll-spy + scroll-to-section)
  and all eight sections (`General`, `Launch`, `Java`, `Appearance`, `Audio`,
  `Download`, `Accessibility`, `About`) with the shared controls.
- Dropdown expand/collapse animation (200ms, `ease`, interruptible) and the
  flipping chevron (200ms, `ease-in-out`, with the 0.7 opacity dip), matching
  `BaseDropdownSelect.vue`. The list is drawn by `DropdownOverlay` at the app
  root because Slint's `z` only orders siblings and the scroll area clips.
- `SettingCollapse` expand/collapse animation (200ms height + opacity,
  `cubic-bezier(0.215, 0.61, 0.355, 1)`) and its chevron flip, matching
  `SettingCollapse.vue`.
- `BaseInput` numeric fields draw the two step buttons (up/down chevrons stacked
  in one column) that the native `<input type="number">` spinner provides in the
  Vue original; the caret is tinted with the text colour via the selection
  background. Clicking outside a text field blurs it (`globals/focus.slint`).
- Icon rendering for the migrated screens (`icons.slint` + `AppIcon`).
- Config load/save (`slint-config`) wired to the `AppConfig` global: every
  setting is persisted to the same `~/.conic[-debug]/config.toml` as the Tauri
  app. Editing text fields saves on a 400 ms debounce.
- Data directory layout (`slint-folder`), a Tauri-free mirror of
  `crates/folder`: the shared `~/.conic[-debug]` root, `config.toml`, music and
  log folders, Minecraft folder helpers, and the launcher-profiles seed written
  at startup. The data-root helpers that previously lived in `slint-config` were
  removed in favour of `slint_folder::DATA_LOCATION`, matching the Tauri app's
  `folder` plugin.
- Language switching across all 12 bundled locales, applied at runtime without a
  restart (see Internationalization).
- Theme switching: the four Catppuccin flavors (Latte/Frappé/Macchiato/Mocha),
  each with and without high contrast — eight schemes, mirroring
  `src/assets/styles/catppuccin-theme{,-hc}.css`. `Theme.active-palette` resolves
  `appearance.palette`, `palette_follow_system` (via `Palette.color-scheme`) and
  `accessibility.high_contrast_mode`. The semantic tokens (`--controllers-*`,
  `--setting-item-*`, `--setting-group-*`, `--toggle-switch-*`, `--card-*`,
  `--default-text-color`, …) are derived per scheme, so Latte's remapped
  surfaces and high contrast's opaque hairlines / whitened text match the
  original. Slint globals cannot animate, so `ThemeProvider` (instantiated once
  in `App`) owns the color tokens, eases them over 300ms `ease` on a palette
  change, and forwards each value into the `Theme` global — the equivalent of
  the Vue frontend's `.changing-theme` class.
- The game screen (`GameView`): `InstanceSummary` (current-instance title,
  metadata, launch/actions row and the local-content previews), `InstancesList`
  (grouped/filtered instance cards with the horizontal-offset rail) and
  `GameFooterBar` (account avatar/switcher, connect, new instance, install pack).
  The `GameState` global carries the domain data; `slint-instance`,
  `slint-account` and `slint-content` (a lightweight count-only mirror) provide
  it. Sorting/grouping/filtering and the flattened `[GameRow]` model are built
  in `app/src/game.rs`. The `InstanceListDropdown` and `AccountListDropdown`
  panels are self-contained.
- The instance list's **Lenis + GSAP "curved rail"** (`views/game/instances-list.slint`):
  cards glide along a parabola — pulled 160px to the left as they cross the
  middle of the viewport and released back at the edges — and the list scrolls
  with Lenis' smoothed offset (`lerp: 0.16`, scaled by the frame time, taken from
  the `ScrollInput.now-ms()` callback because Slint has no wall clock). A `Timer`
  drives `scroll-y` instead of a `Flickable`, which would layer its own
  fixed-duration wheel animation on top and fight the programmatic scrolling the
  view needs (centring on open, gliding to a selection, freezing during a group
  collapse). Wheel events are captured by the `TouchArea` wrapping the content:
  the cards' own `TouchArea`s reject scroll events, so the wheel bubbles to it.
  `components/scroll-view.slint`, the generic scroller the settings page and the
  create-instance screens use, runs the same machinery (the same `lerp: 0.16`)
  with its own slim scrollbar, and exposes `scroll-to(y, smooth)` for the
  programmatic scrolls the Vue does with Lenis' `scrollTo`.
  The same curve carries every list change — the rows' `y` is animated and their
  `x` is a pure function of it, so a reorder travels along the rail exactly like
  the gsap `railKeyframes` sampling `parallaxX`. Rows are reconciled in place by
  `uid` in `game.rs`, so their items survive a relayout; a collapsed group keeps
  its cards parked on their header's slot at `opacity: 0` so they fade and glide
  instead of being destroyed. The custom scrollbar, the toolbar and the
  `GameIntro` timeline (the Vue `onMounted`/`playIntro` GSAP entrance, staggered
  through the `delay` of each `animate`) are ported with it.
- The overlay layer's first dialog: `overlays/dialog-root.slint` (mirroring
  `DialogRoot.vue`) mounts `overlays/dialogs/create-instance.slint`, the
  "create instance" dialog, with its two screens under `dialogs/create/`. The
  dialog runs the Vue's two animations: `BaseDialog`'s scrim/panel entrance and
  exit (gsap timeline → the `delay` of each animated property) and the
  `mode="out-in"` swap between the three screens (`zoom-in` / `zoom-out`, 80ms
  out then 250ms in). `components/base-list-item.slint` (the version rows),
  `components/zoom-transition.slint` (the swap's two halves) and the new
  `slint-install` crate came with it. The dialog is opened by the footer's "New
  instance" button (`GameState.new-instance`), and the whole form is driven by
  `app/src/create_instance.rs` — the version lists are fetched on the tokio
  runtime and reported back through `upgrade_in_event_loop`.
- `slint-platform` (OS detection), `slint-window` (window controls),
  `slint-config`, `slint-folder`, `slint-account`, `slint-instance`,
  `slint-content`, `slint-install`, `slint-java-runtime` and
  `slint-single-instance`.
- `slint-install` mirrors the **version-list half** of `crates/install`:
  `VersionManifest`, the Fabric/Quilt/Forge/Neoforge lists and the caching the
  Tauri plugin keeps in its `PluginState` (30 minutes, like
  `CACHE_EXPIRATION_SECONDS`). Like the original's commands it is `async` and
  uses the shared async HTTP client, so it is awaited on a runtime; the install
  task itself (game files, loaders, Java) arrives with the launch view.
  `filterNeoforgeVersionList` lives in `crates/install/index.ts`, i.e. in the Vue
  frontend, so it is mirrored in `app/src/create_instance.rs` instead.
- `slint-java-runtime` mirrors the **scanning half** of `crates/java-runtime`:
  `models.rs`, `parser.rs` and `scanner.rs` are the original's files, so the two
  crates can be diffed against each other. The differences are the ones
  `slint-install` makes in its own error module — the `serde` derives go away,
  because the original's exist for the Tauri IPC boundary the Slint app does not
  have (the UI calls `JavaVendor::display_name()` instead) — plus a
  `SCAN_CACHE_TTL` cache behind `scan_java_runtimes_cached`, standing in for the
  Tauri plugin's `ScanState`. The scan walks `JAVA_HOME`, `PATH`, the platform's
  JVM directories (Homebrew and Minecraft's bundled runtimes included) and the
  Windows registry; the settings list hides whatever it flags `is_managed`, as
  the Vue does. `resolve.rs` (which runtime to launch with) and `mojang.rs` (the
  launcher-managed runtimes) arrive with the launch view, like the install task
  of `slint-install`.
- Window drag regions (`globals/window-drag.slint` + `WindowService::drag_window`):
  a press inside one starts the platform's own window drag — on macOS
  `performWindowDragWithEvent:`, the way Chromium, Electron and Tauri do it, so
  the drag keeps working outside the window and with the system's window
  snapping. This is what the Vue expresses with `data-tauri-drag-region`; the
  dialog's scrim uses it, so the dialog can be moved by its shadow area.
- `slint-single-instance`, the single-instance guard, replacing the Tauri app's
  `tauri-plugin-single-instance` (a dependency and three lines in
  `core/src/main.rs`). `main()` claims the role before the window exists, so a
  later launch quits without ever showing one, exactly as the plugin's does; the
  launch it was made with is handed to the instance that is already running,
  which brings the window forward through `WindowService::bring_to_front` — the
  equivalent of the plugin's callback, whose `set_focus()` goes to the first
  webview window. What that callback throws away (`|app, _, _|`) arrives here in
  full: the crate reports the later launch's `argv` and its working directory.
  The app owns its event loop and there is no plugin to host the backends, so
  there is one per platform, each the pair the plugin uses on that platform:

  | Platform | Lock                    | The later launch is reported by |
  | -------- | ----------------------- | ------------------------------- |
  | Linux    | a D-Bus well-known name | an `ExecuteCallback` method     |
  | macOS    | a `UDS` socket file     | a connection to that socket     |
  | Windows  | a named mutex           | a `WM_COPYDATA` message         |

  Only a process of the same user can reach any of them: the D-Bus name and the
  Windows object names derive from `APP_ID` (D-Bus hands a well-known name to
  one connection, and it is the session bus, which is per user), and the macOS
  socket is qualified with the uid rather than being a fixed name in the shared
  `/tmp`. None of it is load-bearing on failure either — a backend that cannot
  take its lock (no session bus, no socket to bind) logs a warning and lets the
  launch continue, so a headless session still starts the app. The deviations
  from the plugin are three: the `APP_ID` (`app.conicmc.launcher.slint` rather
  than the Tauri app's `app.conicmc.launcher`, so both frontends can run side by
  side while both exist — drop the suffix when the Slint app replaces it), the
  `WM_COPYDATA` payload being NUL-framed rather than `|`-joined (a path may
  contain a `|`), and a later launch waiting up to two seconds for the primary
  to publish its window instead of quietly running a second copy when it catches
  it mid-startup.
  The arguments are logged and nothing acts on them yet: they are the deep-link
  payload (`conic-launcher://…?code=…`, which the desktop entry hands over), and
  the accounts view that consumes it is not migrated. The Tauri app's half of
  that still runs as it always did — its own plugin, its own lock, its own
  `onOpenUrl` listener.
- `app/src/runtime.rs`: the tokio runtime the background work runs on, standing
  in for the one Tauri builds at startup. `spawn` carries the async work (the
  HTTP calls of `slint-install`) and `spawn_blocking` the disk work (the Java
  scan, creating an instance), both reporting back through
  `upgrade_in_event_loop` — Slint itself is not thread-safe.
- `app/src/scroll_input.rs` backs `globals/scroll.slint`. Slint's
  `PointerScrollEvent` carries the deltas and the modifiers and nothing else, so
  the two facts the Lenis-style scrollers are built on are read from the
  platform instead: a monotonic clock (the smoothing scales its step by the real
  frame time, and `DateNow` reports a date rather than an instant) and where a
  scroll event came from. On macOS an `NSEvent` local monitor — a supported
  hook, nothing swizzled — records `momentumPhase` and
  `hasPreciseScrollingDeltas` as the event goes by, before Slint dispatches it,
  and the `.slint` side pulls that while it handles the very same event. Every
  other platform answers `wheel`, i.e. what the containers did before.

Not yet migrated: `LaunchView`, `AccountsView`, the setup wizard, the remaining
overlays (dialogs, content panels, command palette, music player, instance
settings) and the background renderer. The game view's click-to-open content
overlays and the account skin/audio-visualizer rendering are stubbed;
`views/game-placeholder.slint` stands in for the not-yet-migrated views.

## Conventions

- `.slint` files use **kebab-case** names (`title-bar.slint`); exported
  components use **PascalCase** (`TitleBar`).
- Organize by feature folder to mirror the Vue tree: `components/` (shared),
  `views/` (screens), and — later — `overlays/` and `globals/`.
- The macOS window is Chrome-style (transparent, title-hidden, full-size content
  view), configured in `src/main.rs`; the native traffic lights are aligned to
  the custom title bar by `src/traffic_lights.rs` (see below).

## Known issues / notes

- **Instance list deviations** from the Vue original, all deliberate:
    - Cards no longer change opacity as they enter and leave the viewport. The
      original dims them to 0.6 outside the scroll view's visible area and brings
      them back to 1 inside it (the `ScrollTrigger` `visible` class); that effect is
      dropped in the migration, so cards always draw at full opacity.
    - Expanding a group glides its cards back out along the rail, while the
      original re-creates them and slides them in from +24px (it has no notion of a
      "parked" row). Collapsing matches the original.
    - Touch drag-panning is not implemented. The Vue list scrolled with a native
      `overflow-y: auto` container underneath Lenis, so a touchscreen still panned
      it; the Slint version is wheel-only, like a `Flickable` with
      `mouse-drag-pan-enabled: false` (which is what the rest of the app uses).
    - The wheel step is doubled (`wheel-step`) to match the browser's ~120px per
      notch; a trackpad's pixel deltas arrive in the same units and are applied
      as they are — that constant is the wheel's knob only, and is the one to
      change if a mouse wheel feels fast.
    - The toolbar and the footer have no backdrop blur: the original relies on
      `backdrop-filter: blur(4px)`, and Slint 1.18 has no backdrop blur (its only
      blur is `drop-shadow-blur`). They stay translucent, so the rows scrolling
      under the toolbar show through it unblurred.
- **A rounded `clip` is a Windows-only no-op** — the reason the settings page's
  cards came out with square corners there and nowhere else. Slint's desktop
  default is femtovg over OpenGL, but the winit backend **silently falls back to
  the software renderer** when the GL context cannot be created: no log, no
  error. That is far more likely on Windows than elsewhere — a VM, an RDP
  session, a disabled or outdated GPU driver, a machine with no OpenGL 3.3 ICD.
  Linux and macOS essentially always get the context, which is why only Windows
  showed it. `i-slint-renderer-software` does rounded *fills* and rounded
  *borders* (per corner), but its `combine_clip` is a plain rectangle
  intersection carrying a `// TODO: handle radius`: a `clip: true` +
  `border-radius` box is clipped **rectangularly** there.
  The cards therefore have to draw their own corners, the way the Vue does —
  `.setting-items > div:first-child` / `:last-child`, with no `overflow` on
  `.setting-items`. Slint has no `:first-child` selector and cannot reach into a
  `@children` slot, so `SettingItem`'s `group-first` / `group-last` stand in for
  the two selectors, and a caller whose content is a dynamic slot has to pass
  them on (`JavaRuntimeList`). The two ways of *not* doing that are both wrong:
  clipping the wrapper cuts off the value tooltip a control in the first row
  draws above itself, and rounding every row with the wrapper painting the card
  behind them leaves a notch at each row's corners and at every 1px seam that
  only the card's colour hides — a hovered row is a lighter colour, so its four
  corners show the base one through. A *rectangular* clip is fine, and is what
  `SettingCollapse`'s content keeps: it is there to cut the rows off while the
  height animates to zero.
  **Rule of thumb: never rely on a rounded `clip` in this app.** Still unfixed,
  in that what the corner is cut from is an image rather than a fill:
  `AccountAvatar` (a skin inside a `border-radius: 10000px; clip: true` circle)
  and `SettingItem`'s `custom-icon-rounded` — both show square corners on
  Windows.
- **Scrolling**, in both containers (`components/scroll-view.slint` and the
  instance list), follows the platform:
    - A mouse wheel's deltas are accumulated into the target and the offset eases
      after them — Lenis' wheel path, which is the design. A notch is a
      *distance*; there is no motion to continue.
    - A trackpad's deltas are applied as they arrive — the fingers' and the
      momentum macOS sends after they lift, alike. Content that eases towards a
      gesture trails the fingers, and an easing has nothing to start from once
      the gesture is over: macOS derives its momentum from the velocity the
      fingers left at, so at that moment the offset is exactly caught up, and
      easing would drop the content to a standstill on release and ramp it back
      up. Matching the handoff velocity *and* stretching the tail would travel
      further than the gesture asked for (a first-order filter can keep the
      velocity it inherits or keep the distance, not both), so the tail is the
      system's own glide, as in every native macOS scroll view and Chromium.
      On macOS the kinds are told apart by AppKit (see `scroll_input.rs`); on
      other platforms every event takes the wheel path, i.e. what the app had
      before.
    - Both containers use `lerp: 0.16`. The Vue disagrees with itself
      (`ScrollView.vue` 0.12, `InstancesListScrollView.vue` 0.16); one app should
      glide at one rate, so the instance list's value won.
    - Touch drag-panning is not implemented anywhere (the Vue's native
      `overflow-y: auto` wrapper still panned on a touchscreen; the Slint
      containers are wheel-only). No mouse drag-panning either, which the
      original does not have: `mouse-drag-pan-enabled: false` was what kept a
      press inside a scroll area from being delayed to tell a drag from a click.
    - `ScrollView.content-y` is read-only now (its callers only ever read it for
      scroll-spy maths); scrolling to a position goes through
      `scroll-to(y, smooth)`, mirroring the Vue's `scrollTo(target, smooth)`.
- **Slint notes** learned the hard way while porting the list, kept here because
  both cost real debugging time:
    - `z` on a _component's root_ is ignored when the parent instantiates it
      outside a `for` loop: the compiler's z-order pass only falls back to the
      component root's `z` for repeated children, so the value has to be set on the
      instance (`InstancesList` sets the toolbar's `z: 114` there).
    - `TouchArea.pointer-event` reports a _move_ with `PointerEventButton.other`,
      never `left`, so a drag has to be tracked with its own flag rather than by
      filtering on the button.
- **An `animate` only advances while its property is being read**, which decides
  how the dialog's animations had to be built. An element that is not painted —
  `visible: false`, or clipped or scrolled out of sight — is never read, so the
  first value its animated properties are ever asked for is the one they were
  meant to animate *to*: the fade was skipped and the element jumped to its end
  state. Everything that animates therefore has to be on screen, at the start
  values, one frame before the target changes:
    - `BaseDialog` shows itself for one frame at the start values (`phase:
      "starting"`, 16ms) before moving to "shown", which is what makes the scrim
      fade and the panel scale in instead of appearing — and the scrim's alpha has
      to be part of the dialog's background colour, not an element `opacity`,
      because the fade runs on a `visible: false`-when-closed root.
    - The create-instance swap plays the **leave on the mounted screen** (the step
      is only changed when it is over) instead of on a copy mounted ahead of time:
      a copy kept out of sight jumps straight to the leave values when it is shown.
      The arriving screen is then mounted at the enter rest and moved to "shown"
      one frame later (`enter-timer`), for the same reason.
    - `scrim-alpha`, `content-alpha`/`content-scale` and the swap's `phase`s are
      plain state, never derived from each other, so no animation depends on a
      value that is itself mid-animation.
- **Create-instance dialog** deviations, all deliberate:
    - The dialog's scroll area (`overflow-y: auto` in the Vue) draws no
      scrollbar: the webview hides scrollbars globally, so the original never
      showed one either. The version list keeps the app's `ScrollView` (and its
      slim thumb), exactly like the Vue nests that component there.
    - The version chooser's category filter and Cancel button keep the heights
      their padding and content ask for (40px and 30px). The Vue's flex column
      overflows — `height: calc(100% - 42px)` on the screen plus `height: 100%`
      on the list — so flexbox shrinks them to roughly 32px and 24px.
    - The **mod loader list is built like the version chooser's** — a 1px
      bordered, rounded box around the app's `ScrollView` — where the Vue lets it
      grow to its full height (13 700px for Fabric on 1.20.1, 16 600px for Quilt)
      and scrolls the whole dialog content, and the screen fills the dialog's
      inner area like the version chooser does. Slint's renderer cannot paint an
      element taller than 8192px *inside an opacity/transform group* — the layer
      it allocates for the screen is that tall — and the dialog then stops being
      painted altogether. Two additions the Vue does not have on that screen: the
      border (its list is a plain rounded box, and it is the version chooser's
      list that carries the border the two now share) and the scrollbar (the Vue
      scrolled the dialog's content area and hides scrollbars globally). Above the
      list the screen uses the settings screen's 4px gap, where the Vue has an
      empty `<SettingGroup>` hairline plus its item list's 16px margin — that read
      as a second border just above the list's own.
    - Both lists build **only the rows the viewport can show** (`window-top`) and
      position them absolutely, so a hidden row keeps its place. Past roughly 150
      rows drawn in a frame the renderer gives up the same way — Fabric has 253
      versions on 1.20.1, Quilt 307, and the snapshot category of the version
      chooser around 600 — so the rows outside the window (plus two rows of
      slack) are `visible: false`, and the list's height is computed from the row
      height instead of measured.
    - `BaseDialog` has no "let the content size the panel" mode yet (the Vue's
      `fit-content`): it needs a layout to measure the slot, and every migrated
      dialog passes an explicit size.
    - The dialog's scrim starts a window drag, like the Vue's
      `data-tauri-drag-region`, but the panel does not — a Tauri drag region
      covers the element that carries the attribute, not its children, so
      pressing the panel (or anything on it) leaves the window alone. The Vue's
      two other regions are not wired yet: `App.vue`'s title bar still relies on
      the window system's own title bar area, and the multiplayer dialog is not
      migrated.
    - The instance background's preview overlays the card's own colour with a
      left-to-right gradient instead of a CSS `mask-image` (Slint has no masks),
      and the image is loaded by Rust — Slint can only load a runtime path
      through Rust. The Rust-side loader reads the formats the file picker
      offers (the app enables Slint's `image-default-formats`); AVIF is the one
      it cannot decode yet, though picking one still stores it on the instance.
- **`@children` and `parent`**: the elements written between a component's
  braces are laid out in the slot that hosts `@children`, but their `parent` is
  the element they are written _in_ — for a dialog that is the whole window. A
  child that has to fill the slot (`BaseDialog`'s content) therefore sizes
  itself against properties the host exposes (`content-width` /
  `content-height`), not against `parent`. A `Text` that is not inside a layout
  is also centred on its own width; the create-instance name row pins it with an
  explicit `x`.

- **macOS traffic lights** (resolved): the buttons used to flash back to the
  stock position during the window-open animation and when swiping between
  Spaces. Writing their frames always loses that race — AppKit re-derives the
  titlebar from its own metrics on every layout pass, and during those
  animations the window is composited by the window server with no update
  notifications to re-apply from. `src/traffic_lights.rs` now answers the
  metrics AppKit asks for instead (the model Chromium moved to in 2018), so
  there is no stored position left to reset. It is private API, so a self-check
  verifies the result on the first visible frame and hands over to the older
  frame-writing path if it ever stops taking; run with
  `CONIC_FORCE_TRAFFIC_LIGHT_FALLBACK=1` to exercise that path, and
  `RUST_LOG=shell=debug` to see which one is active. Fullscreen is also the one
  case where nothing has to be cleared: AppKit hides the buttons, and the title
  bar's leading controls move into the corner they leave behind (the Vue keeps
  the 90px gap). AppKit's fullscreen notifications drive that, so the layout
  flips when the buttons do — while the title bar is *revealed* by the pointer
  at the top of a fullscreen window the two do overlap, which is what Chromium's
  window-controls overlay does as well.
- **Icons**: the original uses Font Awesome Pro (`fa-pro`), which can't be
  shipped. The search glyph is currently a hand-embedded path; a proper icon
  strategy (e.g. the SVGs in `src/assets/icons/`) is still to be decided.
- The placeholder view contains dev-only English strings; real localized text
  arrives with the actual views.
