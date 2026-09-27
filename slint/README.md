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
      windows_caption.rs            # Windows: the platform's own caption buttons
      runtime.rs                    # the tokio runtime the background work runs on
      config_bridge.rs              # config ↔ UI bridges (file pickers, opening URLs)
      settings.rs                   # the settings screen's script
      game.rs                       # the game view's script
      launch.rs                     # the launch view's script
      create_instance.rs            # the create-instance dialog's script
      account_add.rs                # the add-account dialog's script
      account_avatar.rs             # player-head avatars (the Vue's canvas crop)
    ui/
      app.slint                     # root `App` Window (mirrors src/App.vue)
      theme.slint                   # palette + typography tokens, embeds fonts
      icons.slint                   # icon path data (mirrors src/assets/icons/*.svg)
      fonts/ComfortaaNunito.ttf      # Comfortaa with Nunito digits merged (see Fonts)
      assets/                       # palette previews, about logos, version icons
      assets/skins/                 # the 18 bundled default skins (slim + wide)
      globals/
        background.slint            # window background state (the Vue's WindowBackground)
        navigation.slint            # global page navigation (src/store/navigation.ts)
        settings.slint              # global config state (the "config store")
        game.slint                  # game view state (src/store/instance.ts + …)
        launch.slint                # launch view state (src/views/LaunchView.vue)
        content.slint               # content overlay state (useContent.ts + store/content.ts)
        dialogs.slint               # dialog store + the create-instance form state
        focus.slint                 # blur-the-focused-field helper
        scroll.slint                # wheel/trackpad classification + monotonic clock
        dropdown.slint              # the dropdown overlay's anchor + selection
        tooltip.slint               # the description tooltip's anchor + hover state
        window-drag.slint           # the Vue's `data-tauri-drag-region` regions
      components/                   # shared/reusable pieces
        title-bar.slint
        window-background.slint     # the window's background layers (src/components/WindowBackground.vue)
        account-avatar.slint
        base-loading.slint
        base-progress.slint         # the launch view's linear progress bar
        title-bar/navigation-button.slint
        title-bar/title-bar-action-button.slint
        search-bar.slint
        app-icon.slint
        setting-group.slint
        setting-item.slint
        setting-collapse.slint
        scroll-view.slint
        base-dialog.slint           # modal shell with its scrim/panel animation
        description-tooltip.slint   # a pinned description's full text, on hover
        base-list-item.slint        # a row of a list panel
        zoom-transition.slint       # the zoom-in / zoom-out screen transition
        base-switch.slint
        base-select.slint
        base-dropdown-select.slint
        dropdown-overlay.slint
        base-slider-bar.slint
        base-input.slint
        base-button.slint
        base-checkbox.slint
        slide-transition.slint      # the slide-left / slide-right screen swap
        item-loading-icon.slint
      views/
        settings-view.slint         # src/views/SettingsView.vue
        settings/                   # the eight settings sections + InfoBox
        game-view.slint             # src/views/GameView.vue
        game/                       # summary, list, toolbar, footer, dropdowns
        launch-view.slint           # src/views/LaunchView.vue
        game-placeholder.slint      # stand-in for the not-yet-migrated views
        accounts/                   # the add-account dialog's three screens
      overlays/
        dialog-root.slint           # src/overlays/DialogRoot.vue
        dialogs/
          account-add.slint         # src/overlays/dialogs/AccountAdd.vue
          create-instance.slint     # src/overlays/dialogs/CreateInstance.vue
          launch-errors.slint       # the four launch error dialogs
          multiplayer-extension.slint  # …/dialogs/MultiplayerExtension.vue
          multiplayer/
            download-description.slint # …/multiplayer/DownloadDescription.vue
            download-progress.slint    # …/multiplayer/DownloadProgress.vue
            multiplayer-manager.slint  # …/multiplayer/MultiplayerManager.vue
          create/
            minecraft-choose.slint  #   …/create/MinecraftChoose.vue
            mod-loader-choose.slint #   …/create/ModLoaderChoose.vue
        content/                    # src/overlays/content/ — see below
          content-root.slint        #   the layer GameView mounts
          content-overlay.slint     #   the scrim + sliding container every panel uses
          content-title-bar.slint   #   styles/title-bar.less
          content-card.slint        #   styles/content-card.less `.content`
          content-card-grid.slint   #   styles/content-card.less `.content-card-grid`
          content-search-panel.slint#   ContentSearchPanel.vue
          content-pagination.slint  #   ContentPagination.vue
          content-not-found.slint   #   ContentNotFound.vue + the loading block
          markdown-plain.slint      #   styles/markdown-body.less, as plain text
          panels.slint              #   the five panels and the nine lists
          details.slint             #   the six detail panels
  crates/
    platform/                       # Tauri-free mirror of crates/platform
    window/                         # window-control service (min/max/fullscreen)
    config/                         # Tauri-free mirror of crates/config
    folder/                         # Tauri-free mirror of crates/folder
    account/                        # Tauri-free mirror of crates/account (whole crate)
    instance/                       # Tauri-free mirror of crates/instance
    content/                        # Tauri-free mirror of crates/content
    modrinth/                       # Tauri-free mirror of crates/modrinth
    curseforge/                     # Tauri-free mirror of crates/curseforge
    shared/                         # Tauri-free mirror of crates/shared (HTTP client, …)
    version/                        # Tauri-free mirror of crates/version
    download/                       # Tauri-free mirror of crates/download
    install/                        # Tauri-free mirror of crates/install (whole crate)
    launch/                         # Tauri-free mirror of crates/launch (whole crate)
    statistics/                     # Tauri-free mirror of crates/statistics
    java-runtime/                   # Tauri-free mirror of crates/java-runtime (whole crate)
    single-instance/                # single-instance guard (no Tauri plugin here)
    multiplayer/                    # Tauri-free mirror of crates/multiplayer (whole crate)
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
- **Windows caption buttons** (`app/src/windows_caption.rs`): the window has no
  system title bar, and the minimize/maximize/close controls are the platform's
  own — real non-client area, not buttons the app drew. `WM_NCHITTEST` answers
  `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE` for the three slots, which is what
  brings DWM's Windows 11 snap-layouts flyout, the system menu and the
  accessibility entries with them; a press sends the `WM_SYSCOMMAND` a caption
  button sends (`SC_MINIMIZE` / `SC_MAXIMIZE` / `SC_RESTORE` / `SC_CLOSE`) on the
  *release*, so dragging off a control cancels it. The same subclass owns the rest
  of the frameless window: the caption is gone (`WM_NCCALCSIZE`), the invisible
  resize border is the hit test, and a maximized window is exactly the monitor's
  work area. The glyphs come from the font Windows draws its caption buttons with
  and the fills are DWM's — see Known issues for why uxtheme could not supply
  them. macOS has the mirror-image arrangement in `traffic_lights.rs`.
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
- The description tooltip (`components/description-tooltip.slint` +
  `globals/tooltip.slint`): a `SettingItem` whose description is pinned to a
  fixed number of elided lines shows the whole string on hover, in a panel drawn
  at the app root and anchored to the description line — the same arrangement as
  `DropdownOverlay`, for the same reason (it is taller than its row, and both the
  collapse and the scroll area clip). The reveal is a height animation from one
  line at opacity 0, on `BaseDialog`'s phase machine. The anchor is re-pushed
  from the row that owns it (`DescriptionTooltip.anchor-seq`) on an 8ms tick (the
  same tick `ScrollView` and `BaseSliderBar` ease on) for as long as the panel is
  up, so **the panel rides along with the content** on every input path (wheel,
  trackpad, its momentum) rather than being left behind; a row that scrolls out of
  the viewport drops out of the hit test, which ends the hover and closes the
  panel. The tick is not laziness: `absolute-position` is a native call, and the
  Rust backend emits it without a property read, so it registers no dependency and
  never invalidates — a `changed absolute-position` handler is silent while the
  list scrolls, and the row's own `y`/`width` do not move when it does (the offset
  is applied by an ancestor `y` far above the row), so there is no property down
  there to watch instead. Ownership is a sequence number rather than a flag
  because "the panel is up" is global: with a flag every pinned row would push its
  own anchor on every tick and the last push would win. Nor can the tick be gated
  on the row's own hover: a trackpad gesture never moves the pointer, so a scroll
  takes the row out from under it and leaves the pointer on the panel, and the
  panel still has to travel from there. Nothing gates on "the content is moving"
  either: `ScrollView`'s only such signal is "the offset is still easing towards
  the wheel target", which a trackpad never is (it moves the offset and the target
  together), so that gate covered a wheel and not a trackpad.
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
  the Vue frontend's `.changing-theme` class. Whether a change eases is declared
  by whoever makes it (`Theme.transitions-enabled`), never inferred: see the note
  on the startup palette under Known issues.
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
- The **account layer**: `slint-account` mirrors the whole of `crates/account` —
  the Microsoft OAuth → Xbox Live → XSTS → Minecraft services chain (with the
  device-code flow), offline profiles and the Yggdrasil (authlib-injector)
  client — so a module can be diffed against its original file by file. The
  command layer of the original becomes plain API: the two pass-through command
  modules have no counterpart (their functions are the module functions), and
  `microsoft_commands.rs`'s at-most-one-login-task state machine is
  `microsoft_task::LoginTaskState`; `LoginReporter` reports through a closure
  instead of a `tauri::ipc::Channel`, and `shared::HTTP_CLIENT` / `UrlExt` are
  inlined in the crate's `shared` module the way `slint-install` inlines the
  client. The add-account dialog (`overlays/dialogs/account-add.slint` and the
  three screens under `views/accounts/`) is driven by `app/src/account_add.rs`,
  with `account_avatar.rs` reproducing the Vue's `<canvas>` head crop, and its
  18 bundled default skins, in a pixel buffer; the game footer's avatar and its
  account switcher draw from the same pipeline (memoised in `game.rs`, since a
  skin has to be decoded and cropped and `apply` runs on every list change).
- `slint-modrinth` and `slint-curseforge` mirror `crates/modrinth` and
  `crates/curseforge`. Neither original holds any plugin state and none of their
  eighteen commands takes a `State`, an `AppHandle` or a `Channel` — they are
  thin wrappers over the functions beside them — so the mirrors drop the whole
  command layer and keep everything else exactly: the same three base URLs each
  (the MCIM mirror for everything it serves, the official API for the one
  endpoint it does not), the same `slint-shared` client and URL builder, the
  same error enums and their `{"kind", "message"}` shape, `curseforge`'s
  `request_with_fallback` with its `response_is_valid` test and its
  `CURSEFORGE_API_KEY` gate (the `build.rs` that bakes the key in comes with
  it), and `compute_fingerprint` with its MurmurHash2. Two deviations:
  `get_multiple_projects` is not mirrored (it is broken upstream — it hands a
  `&[&str]` to `RequestBuilder::query`, which reqwest serializes through
  `serde_urlencoded`'s pair serializer and rejects, so it always fails;
  `get_projects` does the same job and works), and `Modrinth`'s two request
  structs have public fields here because the app builds them rather than Tauri
  deserializing them.
- `slint-content` grew from a file counter into the whole of `crates/content`:
  `mods/` (the four loader archive parsers with their nested-jar recursion, and
  `remote.rs`'s four-file cache with its 24h TTL and read-merge-write lock),
  `saves/`, `resourcepack.rs`, `screenshots.rs` and `favorites.rs`, all kept
  file for file against the original so the two can be diffed. `worldmap.rs` is
  the one module not mirrored (see the world map note below), so its three error
  variants are gone with it. The entry points take `&str` where the original
  took `String`, which was only ever what Tauri's IPC deserialization produced,
  and the four commands that had no function of their own to call
  (`cmd_get_save_icon`, `cmd_get_save_path`, `cmd_delete_save`,
  `cmd_remove_mod_files`) became plain functions with the `cmd_` prefix dropped.
- `slint-platform` (OS detection), `slint-window` (window controls),
  `slint-config`, `slint-folder`, `slint-account`, `slint-instance`,
  `slint-content`, `slint-install`, `slint-java-runtime` and
  `slint-single-instance`.
- `slint-install` mirrors the **version-list half** of `crates/install`:
  `VersionManifest`, the Fabric/Quilt/Forge/Neoforge lists and the caching the
  Tauri plugin keeps in its `PluginState` (30 minutes, like
  `CACHE_EXPIRATION_SECONDS`) — and, since the launch view, the **whole install
  pipeline** too: `install()` (the body of the original `cmd_spawn_install_task`,
  taking the same `Arc<Mutex<InstallEvent>>` the command thread polled rather
  than a Tauri `Channel`), the vanilla/libraries/assets download lists, the
  Mojang Java runtime download, the Fabric/Quilt/Forge/Neoforge installers (the
  two Forge bootstrapper JARs are copied over byte for byte) and the
  first-launch `options.txt`. Like the original's commands it is `async` and
  uses the shared async HTTP client, so it is awaited on a runtime.
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
  the Vue does. Its rows are the one place on the settings page that pins its
  description to a single elided line with a hover tooltip for the whole of it,
  where the Vue wraps the path over as many lines as it takes — a wrapping row
  height cannot be *measured* here (see the note in **Known issues**), and the
  path is the longest description on the page. `resolve.rs` (which runtime to
  launch with) and `mojang.rs` (the launcher-managed runtimes) are mirrored too,
  now that the launch view resolves a runtime; `resolve_java_executable` runs the
  blocking scan on the app's tokio runtime where the original used
  `tauri::async_runtime::spawn_blocking`.
- The **launch view** (`views/launch-view.slint` + `globals/launch.slint` +
  `app/src/launch.rs`), with the rest of the crate layer it needs:

    - `slint-launch` mirrors the whole of `crates/launch`: `complete.rs`
      (assets/libraries/Java completion with its lock files), `options.rs` (the
      memory allocator and the per-instance/global launch options), `arguments.rs`
      (the JVM/game argument list, the classpath and native-library extraction)
      and `lib.rs`'s `launch()` — file completion, version resolution, Java
      selection, authlib-injector setup, the per-platform launch script and its
      stdout watcher. The two Tauri commands become `LaunchEvent` + a plain
      `launch(config, instance, Arc<Mutex<LaunchEvent>>)`, which the app spawns
      and aborts to cancel.
    - `slint-download` mirrors `crates/download`: the downloader, the mirror
      picker, the checksum hashing and `DownloadState`. Only the
      `cmd_spawn_download_task` / `cmd_cancel_download_task` command layer is
      dropped (the app owns the task handle), and the **chunked** path is kept
      disabled — see the launch-view deviations below.
    - `slint-version` is a copy of `crates/version` (the `version.json` model,
      inheritance merge and argument resolution), and `slint-shared` is a copy of
      `crates/shared` (the launcher version, `HTTP_CLIENT` and its proxy
      preference, `UrlExt`), so `slint-download`, `slint-install` and
      `slint-launch` share one client exactly like the original workspace — where
      before `slint-install` had inlined its own. `slint-platform` grew the
      `DELIMITER` / `strip_unc_prefix` / `get_available_memory_bytes` helpers the
      argument builder and the memory allocator use.
    - `slint-statistics` mirrors `crates/statistics`' `log_launch`, so a launch
      still appends to `statistics.json`.
    - The view itself is the Vue's `.container`: the 48px account avatar, the
      32px name, the "Minecraft x / loader y" row, the 340px translucent progress
      panel (which grows/shrinks 300ms between its 58px and error 50px heights),
      the two-row label/value grid and the 240px red "Cancel launch" button.
      `BaseProgress` (`components/base-progress.slint`) is the Vue's 1px track
      with its 3px bar, both the determinate width and the indeterminate sweep.
      `app/src/launch.rs` plays the Vue's `launch()` — the "no Microsoft
      account" / "no account" checks, the Microsoft/Yggdrasil credential refresh,
      the install when the instance is not installed, the launch, and the
      `quit_app_after_launch` close — with the same progress texts (the
      description is a kind plus its parts, so `@tr` stays in the UI) and the
      same four error dialogs (`overlays/dialogs/launch-errors.slint`). Leaving
      the page aborts the task and reloads the instance list, the Vue's
      `onUnmounted`.
- The **window background** (`components/window-background.slint`,
  `globals/background.slint` and `app/src/background/`): the hyperbola sky, the
  3D block world and the user's custom backgrounds, replacing
  `src/components/WindowBackground.vue` — over the window colour, sky at 30%
  then world at 30%, exactly the two `opacity: .3` canvases of the original:

    - `background/scene.rs` is the terrain, trees and face culling, ported
      function for function (`hash2i`/`valueNoise`/`terrainHeight`/`heightAt`/
      `treeAt`/`isSolid`/`emitBlock`), with the original's constants and its
      ring-buffered height cache. It rebuilds only when the camera crosses a
      block boundary, and emits both the faces the software fallback draws and
      the vertex buffers the GPU uploads, in the original's own layouts.
    - `background/gl.rs` is the world **on the GPU**, which is how it is drawn
      by default. Slint's declarative API has no custom-shader hook, but
      `Window::set_rendering_notifier` hands over the current OpenGL context
      (and `get_proc_address`, so the same code loads the entry points on every
      platform), and `Image::from_borrowed_gl_texture` lets Slint composite a
      texture we drew into. Both shader pairs are the original's, line for
      line, minus what Slint now does for us: the corner-radius discard is gone
      (Slint rounds the window's corners when it composites the image, where
      the original had to rebuild window coordinates from the canvas's own
      position) and the layer's 0.3 opacity is folded in, since this image is
      one layer of Slint's composite rather than an element of its own. The
      fills blend `ONE, ONE_MINUS_SRC_ALPHA` into a depth buffer and the 2px
      outlines are screen-space quads expanded in the vertex shader, drawn over
      them with blending off — the frame is written into an offscreen
      framebuffer and never comes back to the CPU.
    - `background/raster.rs` + `background/world.rs` + `background/sky.rs` are
      the **fallback** for a renderer without OpenGL (the software renderer on
      a machine with no GL driver): the same two passes expressed as a
      z-buffered software rasteriser, and the hyperbolae (six curves, four
      branches, composited once from a max-coverage buffer so the polyline's
      joins are not double-blended, plus the `destination-out` band that lets
      them dissolve past the horizon) in the sky's own cached image.
    - `background/controller.rs` owns what is on screen: the current instance's
      background beats the global one, which beats the world; a change
      cross-fades over the one already there instead of waiting for it to
      leave, and rapid changes are coalesced so clicking through instances does
      not start a fade per click. The software rasteriser runs on its own
      thread (one frame in flight, dropped requests pace the loop) so a slow
      frame cannot stall the UI, and the camera only advances while the world
      is what the window shows. The rasteriser starts the app off and keeps
      drawing until the GPU has actually put a frame on screen, then stands
      down — a renderer that turns out to have no OpenGL never takes over, and
      keeps the frames the rasteriser drew.
    - The parallax is eased **in Rust**, not by the component's `animate`
      (`PARALLAX_EASE_SECS`, 35ms: about 100ms to settle, which is what an eased
      100ms transition looks like). Every change goes through it — following the
      pointer, the pointer entering the window, and leaving it — because the
      position always *approaches* its target instead of being set to it, a
      pointer flicking across the edge moves the background slightly and brings
      it back rather than flashing it to a corner. The Vue used a 50ms `quickTo`
      and snapped on entry and exit; this is a deliberate departure. It cannot
      be an `animate` duration on the component: **Slint reads an `animate`'s
      `duration` once, so a conditional in it never re-evaluates** — measuring it
      showed a `flag ? 5000ms : 50ms` behaving exactly like a plain 50ms. (The
      `snap` flags that the background's writes use rely on the same idea and so
      never take effect; what they guard is either a no-op or the fade that was
      wanted anyway.)
    - The parallax is the Vue's: a wrapper scaled 1.08 that follows the pointer
      by up to 4px, the images rendered that much larger so the scaling lands
      them 1:1 on device pixels. The pointer comes from
      `WinitWindowAccessor::on_winit_window_event` — a `TouchArea` cannot be
      used, because one that covers the window swallows every click in the app
      and one underneath the content never sees a move (any `TouchArea` the
      pointer is over accepts the event and ends the walk).

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
- **The multiplayer dialog** (`overlays/dialogs/multiplayer-extension.slint`,
  the three screens under `overlays/dialogs/multiplayer/`, `globals/multiplayer.slint`
  and `app/src/multiplayer.rs`). The footer's globe runs the Vue footer's
  `openConnect` (check the Conic Nexus library, then show either the download
  description or the manager), the description's "Start download" swaps to the
  download screen, and the completed download switches to the manager half a
  second later. The manager carries all seven of its screens (`waiting`,
  `hostScan`, `hostReady`, `guestCodeInput`, `guestJoining`, `guestReady`,
  `exception`), the `slide-left`/`slide-right` out-in swap, the invite code's
  copy-bubble `zoom-in`/`zoom-out`, the 60-second LAN-scan countdown and the
  NAT-type line. The whole `crates/multiplayer` surface is mirrored by
  `slint-multiplayer` (see below): the library download and its checksum, the
  FFI session (`nexus.rs`), the event poll thread with its `get_state`
  reconciliation and the room-code check.


- The **content overlays** the game view opens (`slint/app/ui/overlays/content/`,
  replacing `src/overlays/content/*.vue` — twenty components and their four
  stylesheets). The crate layer they need came with them (see below).
    - `content-overlay.slint` is the shell every panel shares: the Vue's
      `.game-content-wrapper` scrim and its `calc(100% - 150px)` sliding
      container. The transitions are the Vue's — a 400ms
      `cubic-bezier(0, .47, .25, 1)` entrance, a 280ms
      `cubic-bezier(.47, 0, 1, .75)` exit with the scrim's 200ms fade delayed
      100ms behind it. It runs on `BaseDialog`'s phase machine (one frame at the
      start values, because a hidden element is never painted and an animated
      property nothing reads snaps to its target). Only `y` moves: an `opacity`
      or `transform` on the panel would put the whole subtree in a render layer,
      and the renderer gives up on a layer taller than 8192px.
    - One panel per Vue component, except that the six detail panels are *one*
      component (`details.slint`). `Content{Modrinth,Curseforge}{Mod,
      Resourcepack,Pack}Details.vue` are six near-identical files differing only
      in which API fills them and in two conditionals (whether the remove button
      exists, whether there is a gallery); Rust already knows which is open, so
      the difference is carried by `ContentState` and about 1 200 lines of
      duplication go away.
    - The filter rows are a `FlexboxLayout` with `flex-wrap: wrap`, which is
      what Slint has instead of `flex-wrap`. The plain `HorizontalLayout` the
      rest of the app uses never wraps.
    - The **version-chip carousel** is built the other way round. The Vue reads
      `chips[page * 6].offsetLeft` and translates the whole track. An id inside a
      `for` is deliberately unreachable in Slint, so there is no way to *read*
      that offset — but each chip can report the width it measured, and the
      track's offset is the sum of the ones before the page. That is what
      `ContentSearch.chip-measured` carries, and the offset it comes to is what
      the `track` in `content-search-panel.slint` translates by. The carousel
      therefore draws the whole track and clips it, exactly like the original,
      and more than a page's worth of chips shows at once.
    - The **card grid** cannot use a Slint layout: there is no wrapping grid
      (`GridLayout` never wraps, and a repeated child is a single cell), so the
      column count has to come from outside in any case. Rust computes it from
      the panel's width and writes every card's `x`/`y`/`width`/`height` into
      the row, which also keeps a resize onto the *same* elements — the model is
      rewritten with `set_row_data`, so a card keeps its hover and its decoded
      icon. Cards outside the viewport are `visible: false`, as the renderer
      struggles past roughly 150 painted elements in a frame.

**The content overlay layer is complete and verified against the Vue.** The five
panels, the six detail panels, the search panel with its paginated version
carousel, the pagination bar and the empty states all render as the original
does, at the original's measurements. `GameState.open-content` opens the
panels, the scrim and the slide run, the cards carry their icons, translations
and tags, and the instal/remove actions go through `slint-download`.

- `app/src/content.rs` is the controller: `useContentActions.ts`,
  `useFavorites.ts`, `useSearchPagination.ts`, `useDescriptionTranslation.ts`
  and the nine panels' loading. It lives in a `thread_local`, because an
  `upgrade_in_event_loop` closure has to be `Send` and a Slint model is not; and
  cards travel to that closure as plain data (`PendingCard`) for the same
  reason. Images are fetched *and decoded* on the background thread and cross as
  raw pixels, since `Image` is not `Send` either.
- The twelve `.po` catalogs carry the overlay's 102 strings, with the category
  tables seeded from `src/locales/*.ts` in every language. The labels that a
  language change has to follow are resolved in Slint (`ContentText`), not
  composed in Rust.
- `config_bridge::reveal_in_dir` is the Vue's `revealItemInDir`.

Not yet migrated: `AccountsView`, the setup wizard, the remaining overlays
(dialogs, command palette, music player, instance settings). The
audio-visualizer rendering is stubbed;
`views/game-placeholder.slint` stands in for the not-yet-migrated views. The
account avatars (the footer's 56px head, its switcher's 18px rows and the
add-account dialog's profile rows) all draw the real skin now; `AccountAvatar`
still falls back to the placeholder disc for a skin Rust could not decode.


## Rendering the README bodies

A project detail panel shows the project's README (`Modrinth`'s `body`,
`CurseForge`'s description markup). The Vue renders it with `marked` into
`v-html`, so headings, lists, code blocks, links, images and tables all come out
styled. **Slint has nothing that can render an arbitrary Markdown document**, so
the body is drawn as plain text for now — `markdown-plain.slint`, in the Vue's
`.markdown-body` box (surface0, an 8px radius, 16px of padding, a 14px body),
with Rust having flattened the document to text (tags dropped, entities
unescaped, paragraph breaks kept). What that costs: no headings, no lists, no
code blocks, no links, no images, and — because Slint 1.18 has no `line-height`
at all — none of the `line-height: 1.6` the original sets on this one box.

The options, in the order they are worth considering:

1. **Parse in Rust into a block model and draw it with Slint components.** A
   small parser (`pulldown-cmark` is already in the workspace lockfile through
   Tauri) turns the body into a `[MarkdownBlock]` — heading level, paragraph,
   list item with depth, code block, quote, rule, image — and a handful of Slint
   components draw each kind. This is the shape the rest of the app already
   uses: plain data in a model, `for` in the view (`GameRow` is the same idea).
   It needs no browser engine, keeps every string translatable and selectable,
   and the model is where the panel's own images can be spliced in. The cost is
   the parser plus a component per block kind, and a block model has to grow to
   cover tables and inline emphasis when they matter.
2. **Render to a list of styled spans.** The same parser, but emitting runs of
   text with weight/colour/underline rather than blocks — enough for emphasis,
   inline code and links, at the cost of doing the line-breaking by hand.
3. **`StyledText`'s `@markdown`.** Slint has a Markdown-ish rich-text element,
   and the About tab already uses it. It is **not usable here**: `@markdown`
   takes a compile-time literal which the parser reads out of the source map, so
   it cannot be handed a runtime string. It would only work for bodies known at
   build time.
4. **An offscreen webview rendered to an image.** `wry`/`tao` are in the
   workspace, so the HTML could be laid out offscreen, rasterised and handed to
   Slint as a bitmap. The rendering would be exact — and it would pull a browser
   engine into the binary the migration exists to remove, break scrolling and
   text selection, and make the body unstyleable by the palette.
5. **Pre-render server-side.** The MCIM mirror this app already talks to could
   serve a rendered image or a pre-flattened document. No client work, but the
   mirror has to grow the capability and the body stops working offline.

Option 1 is the intended direction: it is the only one that keeps the panel's
typography, its palette and its scroll behaviour under the app's own control.

## Conventions

- `.slint` files use **kebab-case** names (`title-bar.slint`); exported
  components use **PascalCase** (`TitleBar`).
- Organize by feature folder to mirror the Vue tree: `components/` (shared),
  `views/` (screens), and — later — `overlays/` and `globals/`.
- The macOS window is Chrome-style (transparent, title-hidden, full-size content
  view), configured in `src/main.rs`; the native traffic lights are aligned to
  the custom title bar by `src/traffic_lights.rs` (see below).
- The Windows window is created undecorated
  (`Backend::builder().with_window_attributes_hook(|a| a.with_decorations(false))`)
  and its frame is taken over by `src/windows_caption.rs` (see below).

## Known issues / notes

- **The startup palette moves twice, and the second move is the platform's.**
  `Palette.color-scheme` is `Unknown` until the window exists — AppKit, Win32 and
  the XDG portal all need a window to answer — so `Theme.active-palette` resolves
  to `appearance.palette` until then, and the two can differ: `palette = "Latte"`
  with `palette_follow_system` on a dark system is the ordinary case, so the app
  was passing through Latte on its way to Mocha. **Whether that correction eases
  must not be decided by a clock**, and it used to be: a 100ms `Timer` in
  `ThemeProvider` stood in for "startup is over", and it was open one tick too
  early every time, because `SlintContext::update_timers_and_animations()` fires
  the due timers and *then* runs the change handlers in the same call, while the
  window — and with it the real scheme — is only created afterwards, in
  `about_to_wait`. Every startup therefore ran two 300ms cross-fades and the
  intermediate palette was on screen. The fix is the shape the Vue already had:
  `loadPalette()` (`App.vue`, once) swaps the theme class with no transition, and
  `reloadPalette()` (every settings screen) brackets the swap in
  `.changing-theme`, so the *caller* declares whether the change eases. Slint does
  the same through `Theme.transitions-enabled`, which the palette row and the
  high-contrast switch set and nothing else does. It is not a matter of painting
  the window differently: the window is created *before* the first frame, so once
  the moves are instant the intermediate palette is never painted at all. Worth
  remembering for any startup value the platform supplies late.
- **Window background** notes:
    - The world is drawn by `background/gl.rs` on the GPU, at the window's full
      device resolution with no scaling. `install` registers the notifier only
      if the renderer offers one, and the renderer reports in only once it has
      drawn a frame, so the software path is the fallback for a renderer
      without OpenGL — the software renderer on a machine with no GL driver —
      and never has to be chosen explicitly.
    - **Platforms.** Slint's winit backend defaults to femtovg over OpenGL on
      Linux, Windows and macOS alike (`renderer-femtovg` is a default feature),
      and the context it hands the notifier comes with `get_proc_address`, so
      there is no per-platform GL loading code here — no `dlsym`, WGL or GLX.
      The shaders are compiled as `#version 330 core` on a desktop context and
      `#version 300 es` on an ES one (ANGLE on Windows, or a GLES context on
      Wayland), which is decided at runtime from `GL_VERSION`. A context too
      old for either, a driver that cannot make a window at all (Slint then
      falls back to its software renderer, whose `set_rendering_notifier`
      returns `Unsupported`), or any other renderer — wgpu, vello, a
      non-OpenGL Skia — all end up in the same place: `install` or
      `Renderer::new` reports the failure and the CPU rasteriser keeps the
      world. **Verified on macOS** (GL 4.1 core / GLSL 4.10, Apple M4 Pro);
      Linux and Windows take the same code path through the same Slint
      abstraction but have not been run here.
    - The software fallback caps its target at `WORLD_PIXEL_BUDGET` (1.4 MP):
      the default window stays under it and is drawn at full device resolution,
      a maximised window on a Retina display is scaled down. Raise the constant
      on a fast machine — `cargo test --release -p conic-launcher-slint timings
      -- --nocapture` prints what a frame costs. The outline width follows the
      scale, so the lines keep their on-screen weight. The fallback's outline
      pass draws a few hairlines the GPU's does not: its per-row spans put a
      line between the quad's two long edges even when they are less than a
      pixel apart, where GL's pixel-centre rule covers nothing. They are only
      visible as faint dashes inside large faces at full zoom (and they are not
      a depth difference — forcing `GL_ALWAYS` on the GPU's outline pass does
      not bring them back). The GPU path is the one that matches the
      original's shaders exactly.
    - `CONIC_GL_DUMP=/tmp/world.png` (debug builds) writes the world's
      offscreen frame to a PNG on the first frame, and `CONIC_GL_CAM_Z=12.34`
      pins the camera so that frame is reproducible and can be compared with
      `cargo test -p conic-launcher-slint dump_background`'s reference.
    - The window colour under everything is now the palette's flat `crust`, as
      the Vue's `#window` is. The app previously drew a `mantle → crust`
      gradient there; since the sky is only 30% opaque, the base shows through
      and is part of the picture, so the gradient changed it. The container
      keeps the gradient's place in `app.slint` and it is easy to put back.
    - A cross-fade puts the incoming background *over* the one on screen, which
      stays opaque until the incoming one has arrived and is then dropped. The
      Vue instead hid the world's canvases the moment a custom background
      appeared, so the background it replaced vanished before the new one was
      in — a flash of the window colour. Fading both layers at once would do
      the same thing, which is why only the incoming one animates. An incoming
      image *with transparency* is the exception: there the outgoing layer is
      faded out over the same 400ms instead of snapping, since it would
      otherwise be visible through the new one.
    - Custom backgrounds take priority in the Vue's order: the current
      instance's (when it asks to be the launcher's) over the global one over
      the world. Changes are debounced — a burst of instance switches resolves
      to a single transition — and one arriving while a transition is on screen
      waits for it, unless the incoming layer has barely arrived, in which case
      its image is simply replaced.
    - The "background darkness" setting dims the *global* custom background
      only, as in the Vue (`v-if="isGlobalCustomBg"`); an instance background
      and the world are never dimmed.
    - Background images are decoded on a worker thread and cached by
      modification time, so switching back and forth does not re-decode. The
      Vue busts its cache with `?t=` on every switch, which re-decodes on the
      UI thread each time. Images are also scaled down once, to
      `IMAGE_MAX_EDGE` (2560), and the decode is what tells us whether an image
      has any transparency — the answer decides whether the world stays behind
      it.
    - The camera advances **per drawn frame** (the GPU steps it inside the
      render callback, the rasteriser between frames) — the Vue's
      `requestAnimationFrame`, where a frame that never happens also never moves
      the camera. It used to run off a wall clock, which meant a window nothing
      was repainting went on moving invisibly and then jumped when it was drawn
      again.
    - The background's clock is **not a Slint `Timer`**. Slint ticks timers out
      of `update_timers_and_animations`, which only runs while the window is
      being rendered — so a Slint timer stops exactly when rendering does, and a
      camera clock built on one cannot be what restarts it. It is a task on the
      app's own runtime instead, handing each tick to the event loop. (This is
      why `tokio`'s `time` feature is on.)
    - While the camera moves, `window-background.slint` keeps something bound to
      `animation-tick()`, so Slint has an animation in flight and paints every
      frame. `Window::request_redraw` was not enough on its own: on a machine
      whose compositor only repaints for animations, the world stood still until
      the user scrolled the instance list.
    - With **no OpenGL** the world is not drawn at all: `software_only` drops it
      for good once the renderer has had its grace period, the window falls back
      to the hyperbola sky (rendered at the wrapper's full size, so it fills),
      and the settings page greys out the camera option — nothing would move a
      world that is not being drawn. A CPU-rasterised world is many times too
      slow to be worth showing.
    - The camera pauses while a custom background is what the window shows, and
      so does the renderer — the Vue's `cancelAnimationFrame` and its hidden
      canvases. Between block crossings only the projection runs, as there.
    - The sky is still the CPU's, at the GPU's size, because it is a handful of
      curves drawn once per size and palette: it is cached per size and palette
      and re-rendered when either changes (about 30 ms), so a window resize
      drag re-renders it per frame.
    - The Vue's window-open fade (0.8s on the whole background, from `App.vue`)
      is not ported, matching the title bar's entrance animation.
    - The world is drawn **premultiplied** (as the original is) and then
      un-premultiplied by a resolve pass before Slint samples it. Slint hands a
      borrowed texture to femtovg *without* `ImageFlags::PREMULTIPLIED`, so
      femtovg multiplies by alpha as it samples: a premultiplied texture would be
      multiplied twice and everything would come out `alpha` times too dark — a
      hard colour step at the horizon in a dark palette, an obvious one in a
      light palette. Writing straight alpha from the shaders does *not* fix it,
      because fixed-function blending over the framebuffer's transparent black
      cannot hold straight alpha for the first thing drawn; the resolve can, and
      it costs one full-screen pass. The fallback's buffers go through
      `Image::from_rgba8_premultiplied` instead, which is what Slint expects
      there.
    - `Image::from_rgba8_premultiplied` is what feeds the fallback's layers,
      where the rest of the app uses `Image::from_rgba8`: the buffers are
      composited by hand with `source-over`, and premultiplied pixels are also
      what an image scaled by the parallax wants (the alternative bleeds its
      edges). The GPU's texture is premultiplied for the same reason — its
      shaders write `vec4(rgb * a, a)`, and Slint reads a borrowed texture as
      premultiplied — and it is borrowed as `BottomLeft`, since GL's first row
      is the bottom of the viewport.
    - The `gl.rs` callback runs inside Slint's own render pass, so it saves and
      restores every piece of GL state it touches. femtovg re-establishes most
      of what it needs at the top of each flush (its vertex array, attributes 0
      and 1, blending on, depth test off); the framebuffer, the program, the
      vertex array and the buffer bindings are what it does not, and those are
      the ones that would otherwise leak into the UI's own drawing.

- **Content overlay deviations**, all deliberate:
    - The **world map** is not migrated. The saves panel's card expansion shows
      a live map of the world (`.extra` in `ContentSaves.vue`), drawn by the
      814-line `WorldMap.vue` over the external `conic-worldmap` crate, which
      rasterises region files into tiles in Rust. It is a feature of its own —
      its own renderer, its own pan/zoom, its own tile cache — so the card, its
      hover, its selection and the expansion's 160px rise and blue outline are
      all migrated and the panel inside `.extra` is left empty. `worldmap.rs`'s
      error variants are gone from the mirrored `slint-content` with it.
    - A card's **name is not struck through** when a local mod is disabled. The
      Vue has `text-decoration: line-through` on it; Slint has no text
      decoration at all. The dimmed card and the `[disabled]` prefix carry the
      state, which is what the Vue shows too.
    - **`image-rendering: pixelated`** has no equivalent: the card icons are
      scaled smoothly where the webview kept them sharp.
    - The tags of one card are **4px apart**, where the Vue puts 4px before some
      of them and 8px between two pills (a pill's `margin-right` plus the next
      one's `margin-left`, which do not collapse in a line box). A run of pills
      therefore reads slightly tighter.
    - The last-played tag's **label and value are one colour**. The Vue draws
      "last played:" at 80% and the time at full strength; Slint has no per-span
      styling.
    - A card's **rounded icon corners** are rounded by a `clip`, which the
      software renderer ignores — square corners on Windows, exactly like
      `AccountAvatar`. (The filled corners the settings page had to work around
      are not involved here.)
    - The grid's **row height is a constant**. The Vue's rows stretch to their
      tallest card and every card in a list has the same shape, so one number
      per list is what the layout comes to; it is derived from the stylesheet
      rather than measured (see `app/src/content.rs`). If a list's card ever
      gains or loses a line, that constant is what has to change — a *measured*
      height is not available, because the cards are positioned absolutely and
      an absolutely-positioned child contributes nothing to its parent's
      measurement.
      The derivation is easy to get wrong in a way that only shows up in the
      icon: a card is `padding` + its four lines, and **each line box is its own
      paragraph's font size** (14 / 11+4 / 10+4 / 16), not the info block's 16px
      strut — a block's strut comes from its own font. Reading it as a 16px
      strut per line gives 88px instead of 75, and since `img { height: 100% }`
      makes the icon as tall as the card, a 13px-too-tall card shows a visibly
      squeezed icon.
    - An **inner scroll area does not chain to the outer one** (the details
      panel's body inside the panel's scroller, and the horizontal strips inside
      them): `ScrollView` accepts the wheel whenever it has anything to scroll,
      including at either end, so the outer container never sees it. The Vue let
      the browser chain. Related: there is still no horizontal scroll container
      in the Slint tree, so the screenshots strip and the gallery are laid out
      in a plain row that does not scroll.
    - A gallery image's **box follows its own aspect ratio**
      (`.gallery-item img { height: 100%; width: auto }`). Slint cannot read an
      image's natural size, so Rust computes the width from the bitmap it just
      decoded and the model carries it (`GalleryShot`).
    - The description translation (`useDescriptionTranslation.ts`) is in: with a
      Chinese locale the cards and the detail panel show the mirror's
      `translated` text where it has one, fetched per page and cached per
      project.
    - The screenshot viewer's **arrow-key and Escape handling is not wired**.
      Nothing in `slint/app/ui/` uses `FocusScope` or `key-pressed` yet — the
      title bar's ⌘/ hotkey is still a logging placeholder — so the viewer's
      close button and its thumbnails are its only controls for now.
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
- **Slint notes** learned the hard way while porting the content overlays, kept
  here because each cost real debugging time:
    - **`padding` and `border-radius` take exactly one value.** The CSS
      shorthands do not carry over: `padding: 8px 24px` and
      `border-radius: 8px 0 0 8px` are both parse errors, and landing a bare `0`
      in either (`40px 0`) does not help. Use the four longhand properties.
    - **A component has to be declared before it is used**, and that includes
      the local `component` blocks inside one file — a helper written below the
      component that instantiates it is "Unknown element". Ordering the helpers
      bottom-up is the fix; the file order is the declaration order.
    - **A child element is referenced by its bare id, not through `root`.**
      `root.width` reads the root's own property, but `root.some-child.has-hover`
      does not resolve at all — `some-child.has-hover` does.
    - **`horizontal-alignment` / `vertical-alignment` belong to native elements,
      not to components.** A component child of a layout cannot be aligned with
      them; wrap it in a `Rectangle` and centre it inside.
    - **`@tr` placeholders are positional** (`@tr("Installed: {}", version)`);
      a named one (`{version}`) is rejected by the parser.
      `vertical-alignment` is likewise not a property of a layout — use
      `alignment` inside one.
    - **`alignment` is the main axis, and anything but the default `stretch`
      switches every `*-stretch` off.** `LayoutAlignment::Start` (and `Center`,
      `End`, …) makes the layout hand each child its *preferred* size instead of
      running the stretch distribution — `i-slint-core/src/layout.rs` says so in
      as many words (`it.size = it.pref`). CSS's `align-items: start` is
      `cross-axis-alignment`, and writing it as `alignment` cost two visible
      bugs: the version carousel's `horizontal-stretch: 1` viewport collapsed to
      zero width (so the chips were clipped away), and the wrapping chip rows
      wrapped at their own "roughly square" preferred width instead of the
      panel's edge — a wrapping `FlexboxLayout` measures itself as √(total area),
      deliberately, not as its longest line.
    - **A child with an `x` or `y` contributes nothing to its parent's preferred
      size.** `gen_layout_info_prop` (`passes/default_geometry.rs`) skips those
      children, so a component or element whose children are *all* absolutely
      positioned reports a preferred size of **zero** — a bare
      `self.preferred-width` measures 0, silently. Three of this port's bugs
      were exactly that: the metadata entries of a detail panel stacked on top
      of each other, the README body's box collapsed to nothing, and the
      pagination's repeated wrapper stacked its page numbers. Read the size off
      an inner layout (`entry.preferred-width`) or give it one, and watch for
      *conditional* children too — a lone `if` child measures as zero as well.
    - **`TouchArea.has-hover` is true for the topmost area under the pointer
      only.** CSS `:hover` stays true on a parent while a descendant is hovered,
      so the Vue can reveal a card's action column with `.content:hover` and let
      the pointer reach the buttons. Slint cannot: the moment the pointer moved
      onto a button, the card stopped being hovered, the column faded out from
      under the pointer and the two chased each other — the flicker is
      unmistakable once you look for it. A card therefore holds exactly **one**
      `TouchArea` and routes its clicks by position; the buttons are drawings
      that are told whether they are pointed at.
- **Slint notes** learned the hard way while porting the list, kept here because
  both cost real debugging time:
    - `z` on a _component's root_ is ignored when the parent instantiates it
      outside a `for` loop: the compiler's z-order pass only falls back to the
      component root's `z` for repeated children, so the value has to be set on the
      instance (`InstancesList` sets the toolbar's `z: 114` there).
    - `TouchArea.pointer-event` reports a _move_ with `PointerEventButton.other`,
      never `left`, so a drag has to be tracked with its own flag rather than by
      filtering on the button.
    - **`/` is a floating-point division even between two integers.** The
      device code's countdown printed "14.966666 min 58 sec" until it was
      wrapped in `floor()`; the Vue's `Math.floor(total / 60)` is the same
      intent. `mod()` and `round()` behave as expected (`round` yields an int).
- **A repeated element's height is measured at its own *preferred* width**, which
  is the width its content wants unwrapped, while the layout that places it uses
  the width the container gives it. A `word-wrap` `Text` therefore never wraps in
  the measurement and does wrap in the layout, and the two heights differ by a
  line per wrapped line. This is a compiler decision, not a stale value: the
  width change re-triggers the measurement (`Item::layout_info(…, -1, …)` falls
  back to the item's current `width`, read through `Property::get()`, so the read
  is tracked) and the measurement comes out the same way every time. The
  generated code says it plainly — a repeated cell's *sum* goes through
  `layout_item_info(orientation, None)`, i.e. `layout_info(Vertical)` →
  `fn_layoutinfo_v_with_constraint(<the element's own preferred width>)`, while
  the *solve* that actually positions it goes through
  `layout_item_info_at_cross_width(container_width)`. So a container whose height
  is **measured** rather than computed comes out shorter than what it holds:
  `SettingCollapse`'s `content-wrapper` clips the overflow away, and
  `ScrollView`'s `content-height` is short too, so the bottom of the list is both
  cut off *and* out of reach. That is what the "manage installed Java runtimes"
  list did — it is the only `for`-built list of rows with a description long
  enough to wrap, and a *static* row is unaffected (its own `layoutinfo-v`
  measures the text at the width it is given, which is why the "advanced launch
  options" collapse right next to it looked fine).
  **The fix is a `height` binding**: Slint turns any `height` into a layout
  cell's min *and* max, so the measured height stops following the text and the
  measurement and the layout agree. `SettingItem`'s `description-lines` does
  that — `0` keeps the plain wrapping `Text` for every static row, and a pinned
  row puts the text in a fixed-height box with `max-lines` + `overflow: elide`.
  Worth remembering for any future `for`-built row whose height depends on its
  width, and for any list that has to be measured rather than computed.
- **A repeated element gets no layout info of its own**, which is the other half
  of the same story: `gen_layout_info_prop` synthesizes a `layoutinfo-*` for a
  plain element out of its children's, but it *skips* elements with `repeated` set
  — so a wrapper `Rectangle` written inside an `if` (which is what a conditional
  in a layout lowers to) reports the bare `{min: 0, max: ∞, preferred: 0}`
  struct, and a `cross-axis-alignment: start` parent then hands it a zero width
  and nothing inside it is drawn. The pinned description box hit exactly that:
  its `height` is what pins the row, and it still needs an explicit `width:
  100%` (which becomes the cell's min *and* max, as `height` does for the other
  axis) to be given the column's width. Worth remembering for any wrapper around
  a conditional child.
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
      the window system's own title bar area, and the multiplayer dialog's own
      `data-tauri-drag-region` on its 8px body padding is not reproduced (the
      scrim drag below it is).
    - The instance background's preview overlays the card's own colour with a
      left-to-right gradient instead of a CSS `mask-image` (Slint has no masks),
      and the image is loaded by Rust — Slint can only load a runtime path
      through Rust. The Rust-side loader reads the formats the file picker
      offers (the app enables Slint's `image-default-formats`); AVIF is the one
      it cannot decode yet, though picking one still stores it on the instance.
- **Add-account dialog deviations**, all deliberate:
    - The sentence that offers "copy the link" as a link *within* the paragraph
      has no hover tooltip: the Vue floats a "已复制！" bubble over the link
      span, and Slint 1.18 has neither a per-span click target nor a way to ask
      where a span landed. The link itself works (`StyledText`'s
      `link-clicked`), so only the bubble is missing — the dialog's other two
      "已复制！" bubbles (the link box and the device code) are exact.
    - The Microsoft **auth-code flow cannot be finished**. The browser hands the
      code back over the `conic-launcher://` scheme, which the Tauri app
      registers with `tauri-plugin-deep-link` + `tauri-plugin-single-instance`
      and the Slint app has no equivalent of. The screen is 1:1 and "Log in"
      opens the browser; the **device-code flow is the one that completes**,
      until the platform layer grows a deep-link handler.
    - Coming back to the device code after leaving it starts a fresh one. The
      Vue keeps the stale code on screen and never polls it again, which leaves
      the dialog stuck.
    - **The device code poll is fixed, and the Tauri original is not.** The token
      endpoint answers every non-success state — `authorization_pending` and
      `slow_down` above all — with `400 Bad Request` and the OAuth `error` in
      the body, so judging the status before reading the body ended the login
      with "HTTP request failed with status 400" on the very first poll, before
      the user had opened the browser. The body is now read first and the
      `error` becomes the poll's status. A poll that never got an answer at all
      (no network, a 5xx, a proxy's HTML page) is retried up to
      `MAX_FAILED_POLLS` times in a row rather than thrown away, since the user
      is given minutes to finish in the browser and expiry only buys them a new
      code and a new wait.
    - The device code's box is sized to the code rather than to the CSS `20ch`
      (Slint has no `ch` unit), and the sliding screens are clipped by
      `SlideTransition` rather than by the panel, so their travel stops 24px
      short of the original's (see below).
- **Multiplayer dialog deviations**, all deliberate:
    - The manager's bottom button bar is laid out **in flow**. The Vue's
      `.buttons` is `position: absolute; bottom: 24px; width: calc(100% - 48px)`,
      but no ancestor between it and the panel is positioned, so its containing
      block is the full-window `.dialog` scrim: the bar actually renders at the
      bottom of the *window*, detached from the panel, while the panel still
      reserves the manager's 48px `padding-bottom`. Reproducing that would move
      the button off the dialog, so the bar keeps the rule's own `margin-top:
      16px` above it and sits 24px above the panel's bottom edge instead.
    - `p.message`'s `font-style: italic` is dropped: Slint has no oblique style
      for the embedded variable font, so "Waiting for other players to join..."
      is upright.
    - The two "ready" screens drop their own `padding-bottom: 16px`. In the Vue
      that padding and the bar's `margin-top: 16px` stack (the bar's margin has
      no effect at all while it is absolutely positioned, so the padding is what
      the eye sees); with the bar back in flow both would apply, and the gap
      above it came out twice as large on those two screens as on the five
      without the padding. The bar's own 16px is kept, so every screen has the
      same gap.
    - The download screen's headline wraps. The Vue's `p { display: flex;
      justify-content: space-between }` shrinks both spans, so the long phase
      string wraps next to the byte counter rather than pushing it out of the
      panel; the Slint row stretches and wraps the headline the same way.
    - `BaseButton` grew three optional overrides (`button-border-color`,
      `hover-background`, `hover-text-color`) for the download screen's
      `.stop` rule — a red outline that fills red on hover. The Vue gets there
      with a scoped `.stop:hover` selector, which a component's own properties
      cannot express. (`border-color` is `Rectangle`'s own property, hence the
      `button-` prefix.)
    - The room-code format check lives in `slint-multiplayer`
      (`room_code.rs`) instead of the Vue's `index.ts`: it is the same regex and
      base-34 modulo-7 checksum, and the UI needs it without a session (the
      library's own `conic_nexus_room_code_is_valid` is still reachable through
      `NexusService::room_code_is_valid`).
    - The manager stays mounted while the dialog is hidden, where the Vue
      unmounts it (the panel would otherwise animate its height from nothing on
      every reopen, which the Vue's height tracking does not do). The only
      visible difference is that a LAN-scan countdown keeps ticking while the
      window is hidden — the Vue's `onUnmounted(stopScanCountdown)` would freeze
      it.
    - The copy bubble's text is drawn as a centred `Text` inside a 16px box
      rather than a `<p>` sized by `align-items: center`, so the two swapped
      strings stay in exactly the same place through the `zoom-in`/`zoom-out`
      out-in swap.
- **Launch view deviations**, all deliberate:
    - The progress line's `mode="out-in"` fade (100ms out, then 100ms in) is not
      reproduced — Slint has no transition groups, so the text swaps in place.
      The panel's own height still eases over 300ms between its 58px and its 50px
      error size, which is the `transition: all .3s ease` plus the gsap
      `height: auto` capture of the original.
    - The panel's `backdrop-filter: blur(2px)` is dropped (Slint 1.18 has no
      backdrop blur, like the toolbar and the footer); it stays translucent.
    - The indeterminate bar's CSS keyframes (`progress-loading`, 2.5s,
      `cubic-bezier(0.66, 0.01, 0.5, 0.97)`, with the jump back at 50%) become a
      1.25s sawtooth computed from `animation-tick()`, with a smoothstep standing
      in for the cubic-bezier — it is a 3px bar, and Slint has no keyframe
      timelines. The travel is `[-0.6W, +1.1W]`, not `[-0.85W, +0.85W]`: the
      Vue's bar is a flex item in a `justify-content: center` row, so it is
      centred first and `left: ±85%` is an offset from that centre — treating the
      offsets as absolute leaves a 15% tail visible on the right when the sweep
      restarts.
    - The Vue's outer `.avatar-image` carries 2px of padding, so it measures
      52px; the wrapper here is 52px with the 48px `AccountAvatar` inset by 2px,
      which keeps the whole column at the original's 344px. What still differs is
      the no-account placeholder: the Vue paints a flat `surface0` disc, while
      `AccountAvatar` draws its 2px ring and the footer's "unlogged" tint. The
      launch flow requires an account, so that is only visible for the frame
      before the no-account dialog opens.
    - The **chunked download path is disabled** in `slint-download` (every task
      takes the sequential path, and the `Accept-Ranges` probe with it). The
      original design's chunked download is known to misbehave, and the removed
      code is left commented out next to where it ran so the two crates can
      still be diffed.
    - The four error dialogs' sentences are **translated** in the Slint app; the
      Vue hard-codes Chinese (they are in its "not yet internationalized" set),
      so the Chinese stays as the catalog source text and an untranslated locale
      still reads like the original. `NoSuitableJavaError` omits its height in
      the Vue and lets the panel size itself; `BaseDialog` has no fit-content
      mode, so it is given the measured 144px.
    - `launch()` is a single task, so the Vue's two cancel handles
      (`cancelInstallHandle` / `cancelLaunchHandle`) are one abort. The music
      player's `pause_on_launch` is logged only, since the player is not
      migrated.
    - The "must have a Microsoft account" check reproduces the Vue's
      `config.language !== "zh_cn"` verbatim, so a config that follows the system
      locale (`language: null`) counts as non-Chinese and requires a Microsoft
      account — a quirk of the original, kept rather than silently fixed.
- **A word-wrapping `Text` has to be a direct child of the column that owns its
  width.** Slint measures such a text for the width it is going to be given only
  while that chain is short enough to resolve: one layout level deeper, the
  add-account dialog's Yggdrasil paragraph reported a single line for its three,
  the screen's `preferred-height` came out 40px short — and since the panel's
  height *is* that measurement (`body.preferred-height + 32px`), the panel
  followed it down and clipped the screen's buttons away. Both other screens
  already had their paragraphs as direct children, which is why only that one
  was affected. Worth remembering for any dialog whose panel measures itself.
- **`SlideTransition` is a `Rectangle`, where `ZoomTransition` is a layout.**
  Slint 1.18 has no translation transform (only rotation and scale), so the 80px
  slide has to be an animated `x` — and a layout child's `x` belongs to its
  layout. The screen therefore sits in a layout of its own inside a
  `min-height`-sized rectangle that the `x` moves and that clips.
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
- **Windows caption buttons** — the artwork is composed, the behaviour is not.
  Everything a caption button *does* is the system's (see Migrated so far), but
  the pixels come from this app, and only because Windows gives no way to have
  DWM draw them: a window whose caption has been removed from the non-client
  area gets no caption buttons painted over it, on any Windows since 8, with or
  without `DwmExtendFrameIntoClientArea` (that call succeeds; it is the Windows 7
  Aero Glass hook and has been a no-op for opaque windows since). Microsoft's own
  answers agree — `Window.ExtendsContentIntoTitleBar` and
  `AppWindowTitleBar.ExtendsContentIntoTitleBar` remove the frame the same way
  and then draw the buttons themselves.
  `DrawThemeBackground`, the obvious source, cannot supply them either:
    - the `Explorer` class (what the shell and Windows Terminal draw their
      captions from) does not open at all on Windows 11 — there are no
      `.msstyles` files left to load it from, so `OpenThemeData` returns a null
      handle, and there is no `DarkMode_Explorer` to ask for instead;
    - the `Window` class does open, and does draw `WP_CLOSEBUTTON` and friends,
      but it draws the *classic* caption: an opaque button face filled in even in
      its normal state, with the close button red throughout, and in light colours
      regardless of the system theme (a process only gets the dark variants by
      opting in per window, and that opt-in is only reachable through the class
      that is missing). On this app's dark title bar that is a row of pale blocks.
  So `windows_caption.rs` renders the glyphs with GDI from **the font Windows
  draws its caption buttons with** — `Segoe Fluent Icons` on Windows 11,
  `Segoe MDL2 Assets` on Windows 10, tried in that order and the first one that
  actually draws the glyph wins — and fills them with DWM's own colours: the
  caption ink at 10% on hover and 20% pressed, and `#c42b1c` / `#b02518` for the
  close button. The three shapes, the metrics and the behaviour are the
  platform's; the two colours DWM would have supplied are the constants above,
  which is the same trade Windows Terminal and Chromium make. (The wash
  percentages are the app's own numbers, chosen to read clearly against a dark
  title bar; six and twelve were tried first and a press was not legible at that
  strength.)
  One deliberate deviation: the ink follows the **app's** palette, not the
  system's, because the title bar the glyphs sit on is the app's. A white glyph
  is right on Catppuccin Mocha and invisible on Latte, and the system theme
  cannot know which one is in use. `ThemeProvider` reports the resolved palette
  through `App.set-caption-dark` and the artwork is re-rendered when it changes;
  the tokens themselves animate over 300ms, this does not, which is what the
  platform wants (it cannot interpolate its own glyphs). The close button's glyph
  is the one exception: once it is on its red plate it is white in both themes,
  because that is the only ink that reads on that red.
- **The window controls' fill is a colour, not part of the bitmap** — the one
  non-obvious decision in `windows_caption.rs`, and the reason hovering and
  pressing animate at all. Only the glyph is rendered; the wash behind it is
  published as a pair of colours, so `SystemWindowControl` can interpolate it.
  Three things follow from that, and they are worth knowing before changing it:
    - **Hovering and pressing are the fill changing**, so a fill that can be
      interpolated *is* the transition. Baking it into the bitmap would allow only
      a cross-fade between two finished rasters, and three of those cannot be
      stacked: they composite additively, so a press would read as hover *plus*
      pressed. The state is one animated float (0 / 1 / 2) and the glyph never
      moves.
    - **Two animated numbers, not one**: `hover-level` at 120ms and `press-level`
      at 60ms, summed. The press rides on top of the hover instead of replacing
      it, which is what lets go drop straight back to a hover while only *leaving*
      takes the full hover duration — the asymmetry the platform's own buttons
      have.
    - **`press-fill` is an increment, not a fill.** The two layers are stacked, so
      compositing washes of `a` and `b` lands on `a + b - a·b`; the increment that
      reaches the intended pressed strength is `(pressed - hover) / (1 - hover)`,
      worked out in Rust where the numbers live. `hover 10% / pressed 20%` becomes
      layers of 26 and 28, not 26 and 51.
    - Slint's `Color.mix` is *not* usable for this. Its factor weights the
      **receiver** (`a.mix(&b, 0)` is `b`), and it is alpha-aware per the Sass
      spec rather than a plain lerp. Both were found the hard way, from a fill
      that came out inverted. Two `Rectangle`s with animated `opacity` need
      nothing but `opacity` and are exactly predictable.
    - Only the close button's glyph changes ink (its fill is opaque, so the
      caption ink would not read on the red), which is what `plate` in
      `WindowControl` means. Minimize and maximize keep one image at full opacity
      and let the wash move — and are gated on `plate` rather than cross-faded
      unconditionally, because cross-fading an image with itself composites it
      twice, landing short of opaque mid-transition and making the glyph thin out
      as it moves.
- **Icons**: the original uses Font Awesome Pro (`fa-pro`), which can't be
  shipped. The search glyph is currently a hand-embedded path; a proper icon
  strategy (e.g. the SVGs in `src/assets/icons/`) is still to be decided.- The placeholder view contains dev-only English strings; real localized text
  arrives with the actual views.
