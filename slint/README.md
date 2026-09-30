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
      setup.rs                      # the first-run wizard's script
      music.rs                      # the music player's script (see Music player)
      command_palette.rs            # the command palette's script
      instance_settings.rs          # the instance settings overlay's script
    ui/
      app.slint                     # root `App` Window (mirrors src/App.vue)
      theme.slint                   # palette + typography tokens, embeds fonts
      icons.slint                   # GENERATED icon path data (see slint/app/build.rs)
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
        music.slint                 # background-music state (src/store/music.ts)
        command-palette.slint       # the command palette's state + its labels
        instance-settings.slint     # the overlay's own form state (useInstanceSettings.ts)
        setup.slint                 # the first-run wizard's import-instances state
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
        beat-map.slint              # the footer audio visualizer (src/components/BeatMap.vue)
        instance-card.slint         # the instance card (InstanceSetting.vue's `.instance`)
        palette-row.slint           # the four palette tiles (SettingsAppearance.vue /
                                    #   SetupWizardPalette.vue — the same block)
      views/
        settings-view.slint         # src/views/SettingsView.vue
        settings/                   # the eight settings sections + InfoBox
        game-view.slint             # src/views/GameView.vue
        game/                       # summary, list, toolbar, footer, dropdowns
        launch-view.slint           # src/views/LaunchView.vue
        setup-view.slint            # src/views/SetupView.vue
        setup/                      # the wizard's six screens + their shared paragraphs
        game-placeholder.slint      # stand-in for the not-yet-migrated views
        accounts/                   # the add-account dialog's three screens
      overlays/
        dialog-root.slint           # src/overlays/DialogRoot.vue
        music-player.slint          # src/overlays/MusicPlayer.vue
        command-palette.slint       # src/overlays/CommandPalette.vue
        instance-settings.slint     # src/overlays/InstanceSetting.vue
        dialogs/
          account-add.slint         # src/overlays/dialogs/AccountAdd.vue
          create-instance.slint     # src/overlays/dialogs/CreateInstance.vue
          confirm-delete-instance.slint # …/dialogs/ConfirmDeleteInstance.vue
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
          markdown-body.slint      #   styles/markdown-body.less, over slint-markdown
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
    markdown/                       # Markdown/HTML layout engine: parses, measures,
                                    #   lays out, hands the view positioned boxes
    authcode/                       # the loopback callback the Microsoft browser flow
                                    #   hands its code to, and the page it serves
      page.html                    #   the page itself — open it, it is the design
      lib.rs                       #   the listener: bind, accept, route, hand over
      http.rs                      #   the request line and the response, and nothing
                                    #     else (no framework: see Migrated so far)
      page.rs                      #   fills page.html in from Theme and @tr
      error.rs                     #   binding the socket is the only failure
      tests/callback.rs            #   it, over a real socket
    music/                          # Tauri-free mirror of crates/music + the Web Audio
                                    # graph the webview's store owned (see Music player)
      lib.rs                        #   the original crate verbatim: the folder listing
      decode.rs                     #   a file becomes PCM (the element's `src`)
      analyser.rs                   #   the AnalyserNode
      player.rs                     #   the graph + the transport
      session.rs                    #   the saved track and position
      error.rs                      #   crates/music/src/error.rs
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

The instance settings overlay's strings name their context explicitly
(`@tr("InstanceSettings" => …)`) and the delete dialog's do the same
(`ConfirmDeleteInstance`), so a lookup does not move when one of them is written
into a local `component` instead. The overlay's 42 are seeded from
`game.instance.*` in `src/locales/*.ts`; the dialog's four were hard-coded
Chinese in the Vue, so their Chinese is the source text and the catalogs
translate them.

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
  action (`components/title-bar*`, `components/search-bar.slint`) — the last one
  gated on `config.music.enabled`, as the Vue's `v-if` is.
- **The background-music player**, in full: the title bar's action, the panel
  (`overlays/music-player.slint` — card, playlist popup, progress bar, the seven
  transport buttons), and the footer visualizer
  (`components/beat-map.slint`). `slint-music` is the Tauri-free mirror of
  `crates/music` and carries the Web Audio graph the webview used to own; see
  [Music player](#music-player) for the split and Known issues for the
  deviations.
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
  The arguments are logged and nothing acts on them yet: the desktop entry
  hands over the deep-link payload (`conic-launcher://…?code=…`), and nothing
  asks for that URI any more — the Microsoft browser flow comes back to a
  loopback listener of the app's own (see `slint-authcode` below). The Tauri
  app's half of the deep link still runs as it always did: its own plugin, its
  own lock, its own `onOpenUrl` listener.
- `slint-authcode`, the loopback callback the Microsoft **browser** flow hands
  its authorization code to, and the page the browser is shown there. This
  replaces the deep link the Tauri app registers with
  `tauri-plugin-deep-link`, which a Slint application cannot do: registering a
  scheme means asking the desktop environment, claiming single-instance
  ownership of it, and trusting each of the three platforms to route it back.
  Microsoft's own guidance for a native app is a loopback redirect, and that is
  what the browser is sent to — `http://localhost:<port>/callback`, on a port
  the OS picks.
  Four decisions are worth knowing about, and the crate's own module docs say
  more about each:
  - **The port is port 0.** The socket is bound with `bind(…, 0)` and the port
    the OS hands back is read off the bound address, so there is no port to
    collide over — a second launcher, a leftover listener, or anything else
    already holding it cannot stop the flow, and Microsoft accepts
    `http://localhost` on any port for a native client. The port is gone with
    the socket, which is why the URL on the dialog's browser screen cannot be a
    constant the way the Vue's was: it is filled in when the listener is bound.
  - **The listener is the screen's.** It is bound when the browser screen comes
    on with the dialog open and released the moment it goes — the user switched
    to the device code, or closed the dialog — which is exactly as long as the
    URL beside it can be used. `AccountAddMicrosoft.slint` owns *when*
    (`prepare-auth-code-flow` / `release-auth-code-flow`, one `changed` handler
    on `shown == "auth-code" && account-add-visible`); `app/src/account_add.rs`
    owns *what* it is bound to and what an answer does.
  - **It is `state`-checked.** The authorize URL carries a random `state`, and
    a callback whose `state` is not ours is refused and does **not** end the
    login. A loopback listener is reachable by every process on the machine, and
    a port can be guessed; without this, a page in a browser could hand the
    launcher an authorization code of its own choosing and sign the user into
    somebody else's account. This is OAuth's own CSRF token (RFC 6749 §10.12).
  - **The server is a request line and a body.** It answers one `GET` and goes
    away, so the framing is `http.rs`'s: a request line, a status line, a
    `Content-Length`, and `Connection: close`. A web framework would bring a
    dependency tree, a router, a middleware stack and a connection state
    machine for that. The only dependency added is `tokio`'s `net`/`io-util`/
    `sync` (the app already carries `net`) and `regex`, already in the tree
    through `slint-launch`. Every read is bounded in size and in time, because
    the listener is not reachable only by the browser.
  The page the browser lands on is `slint/crates/authcode/page.html`, filled in
  by `page.rs`: a self-contained document — no stylesheet, no script, no image,
  no font file, because it goes over a loopback socket in one response — in the
  app's own theme. The colours are read off the `Theme` tokens
  (`--window-background`'s `crust`, `--dialog-*` for the card, `--ctp-green` and
  `--ctp-red` for the two states, `main.css`'s own font stack) and the six
  sentences off the `@tr` catalog, both handed in by the app rather than looked
  up in the crate, so the tab and the window the login came from are the same
  application in the same language. The glyphs are transcribed from
  `ui/icons.slint`: `checkmark-outline` and `warning` for the two answers, and
  `BaseLoading`'s arc (1.9s and all) for the one that says to go on waiting.
  `cargo run -p slint-authcode --example preview` renders the three screens to
  files if you want to look at them.
  The token request has to repeat the `redirect_uri` byte for byte
  (RFC 6749 §4.1.3), so `slint_account::microsoft::redeem_access_token` takes
  it and `LoginTaskState::spawn` takes a `LoginRequest` rather than the original's
  `Option<String>` — the code and the URI it was issued against travel together
  or not at all.
- `app/src/runtime.rs`: the tokio runtime the background work runs on, standing
  in for the one Tauri builds at startup. `spawn` carries the async work (the
  HTTP calls of `slint-install`, the `slint-authcode` listener) and
  `spawn_blocking` the disk work (the Java scan, creating an instance), both
  reporting back through `upgrade_in_event_loop` — Slint itself is not
  thread-safe.
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
    - The detail panels' **README body is a real renderer**, not flattened text:
      `slint/crates/markdown` and `overlays/content/markdown-body.slint`, in the
      Vue's own `.markdown-body` box at its own measurements. See
      [Rendering the README bodies](#rendering-the-readme-bodies) for what that
      does and does not carry over, and the crate's `README.md` for the engine.

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

- **The command palette** (`overlays/command-palette.slint`,
  `globals/command-palette.slint` and `app/src/command_palette.rs`), replacing
  `src/overlays/CommandPalette.vue` — the panel the title bar's search field and
  the `Ctrl`/`⌘` + `/` shortcut open. All three modes are there (the five
  commands and the instance list at the root, the instance list to launch from,
  and a Modrinth or CurseForge search), with the same rows, section headings,
  placeholder, breadcrumb, footer hints, empty states, 250ms debounce and
  selection rules, and every box at the original's measurement.

    Three things about it are not the shape of the rest of the tree:

    - **The key handling is a `capture-key-pressed`, not a `key-pressed`.** The
      Vue hangs `@keydown` on the `<input>`, and a `TextInput` answers for three
      of the four keys before an ancestor ever sees them: Backspace is accepted
      unconditionally (it has nothing to delete and says so anyway) and the arrow
      keys are accepted to move the caret. `capture-key-pressed` runs on the
      ancestors *first*, and `accept` stops delivery altogether; rejecting it is
      what lets an ordinary Backspace delete a character. Enter is the exception
      — the `TextInput`'s own `accepted` callback is the one Slint calls for a
      single-line field, so that is what it uses.
    - **The `FocusScope` that reads those keys has to *contain* the search
      field**, because a `FocusScope` only sees the events of what it contains.
      And a `FocusScope` *centres* the children it lays out — a child with no `y`
      lands at `(scope.height - child.height) / 2`, where a plain `Rectangle`
      would leave it at 0. That put the input row in the middle of the panel
      until every child was given an explicit `y`; the component says so where it
      matters.
    - **The shortcut is one `KeyBinding`, on a `FocusScope` that wraps the whole
      window.** Slint's `@keys` modifier names are the *physical* keys, so
      `Control` is the control key on macOS too; a second binding naming `Meta`
      is what `⌘` would match, and one binding per platform is the shape the
      Vue's `isMacOS() ? event.metaKey : event.ctrlKey` has. The binding sits on
      a scope that wraps the window, which is what the Vue's document-level
      listener amounts to: a window with nothing focused discards its key events,
      and a scope beside the content would not be an ancestor of a settings text
      field's. `forward-focus` points at it, so it holds the focus from the first
      frame, and the palette hands it back when it closes (`app.slint`) for the
      same reason.
    - **`revealSelected` is in the `content-y` sign, and so is `scroll-to`.** The
      Vue reads the row's `offsetTop`; `ScrollView.content-y` counts *down*, so a
      row 400px into the content is `-400px`, and that is the sign `scroll-to`
      takes as well. The two targets do not read alike — the row below the
      viewport comes out as `bottom + height - 8px` and the one above as
      `top + 8px` — and the second is the quiet one: the other sign still scrolls
      in the right direction, just 16px short, which leaves the row's top above
      the viewport and a fifth of it cut off.

    The list is built in Rust — the order, the filter, the section headings and
    every row's height have to agree, and a Slint expression can neither build a
    model nor index one, which is why `game.rs` and `content.rs` lay their lists
    out there too. Rows carry the *key* of every label rather than the label
    itself, and `CommandText` resolves them, so a language change follows as it
    does everywhere else. The searches run on the tokio runtime with the content
    overlays' icon pipeline (`content::fetch_icon` / `content::cached_icon`, and
    the same `ICONS` cache — a project icon is the same bitmap wherever it is
    shown), and a project opens through `content::open_project_detail`, the
    `open_detail` a card click takes. The twelve `.po` catalogs carry its 25
    strings, seeded from `src/locales/{en_us,zh_cn}.ts` and worded after each
    catalog's existing entries.

    One deviation, on the **filter that picks which of the two root-mode groups
    is shown**: the Vue matches the five commands against `command.title`, and
    `title` is whatever `t()` returned, so a query in one language matches a
    command named in another. Rust builds the list and has no translated titles,
    so it matches the *source* strings instead — the behaviour is exactly right
    in English and in any locale that falls back to it. The rows still show the
    translation. Making the filter see the translation would mean filtering in an
    expression, and Slint can neither build a model nor index one.

- **The instance settings overlay** (`overlays/instance-settings.slint`,
  `globals/instance-settings.slint`, `app/src/instance_settings.rs`), and the
  **delete-instance dialog** its last row opens
  (`overlays/dialogs/confirm-delete-instance.slint`). The panel is the Vue's
  `.game-content-wrapper.instance-settings-wrapper`, so it is mounted on the same
  shared `ContentOverlay` as the content panels with `panel-inset: 200px`, and
  everything inside it is the original's markup: the 52px mantle header that
  scrolls away with the content, the instance card, and the eight settings
  blocks. `InstanceCard` is one component for the card the overlay and the
  delete dialog both draw (`InstanceSetting.vue`, `ConfirmDeleteInstance.vue`
  and `CreateInstance.vue` carry the same block verbatim).
  `app/src/instance_settings.rs` owns the instance being edited: it re-reads it
  from `instance.toml`, merges the overlay's fields into the config, and writes
  it back — the work the Vue's `watchEffect` did. Rust, not the UI, is where the
  two branches of that effect live (switching
  `enable_instance_specific_settings` on copies the launcher's own launch
  settings into the instance, switching it off drops everything but the flag), so
  the panels are pushed what was actually stored rather than what was asked for.
  The card's background picture is decoded with the `image` crate, like the
  window background and the content overlays — see the deviations.

- **The first-run setup wizard** (`views/setup-view.slint`,
  `views/setup/Setup*.slint`, `globals/setup.slint`, `app/src/setup.rs`).
  `SetupView.vue` and the six `views/setup/SetupWizard*.vue` screens: the
  header band, the translucent card, the button bar, and the
  `mode="out-in"` `slide-left` / `slide-right` swap between the steps. The two
  steps that are only *settings* are the settings screen's own sections —
  `SettingsJvm` and `SettingsGame` — taken with `group-inset: 0px`, which is the
  wizard's `:deep(.setting-group) { width: 100% }`; and the palette step reuses
  the settings appearance's palette group, which `components/palette-row.slint`
  now holds for both (the two Vue files carry the same markup and CSS verbatim).
  The add-account step embeds `AccountAdd` with `wizard-host`, which is what the
  Vue's `:deep()` rules over that screen say: the dialog's 8px of padding is the
  wizard's own, and its "Cancel" buttons go.
  `app/src/setup.rs` holds the two things the screens cannot do: the
  import-instances step's two "create a blank instance" buttons, which fetch
  the Mojang manifest and write an `instance.toml` (the `@conic/install` and
  `@conic/instance` calls the component makes), and the platform answer the Java
  screen asks — the Mojang runtimes are published for x86-64 and arm64 on
  Windows and macOS and for x86-64 only on Linux, so anything else gets
  `prefer_mojang_java` turned off on arrival, which is the Vue's `onMounted`.
  The two instance names are `@tr`s on the global rather than Rust strings, so a
  language change renames them with the rest of the screen.
  The twelve catalogs gain 36 entries each, seeded from `setup.*` in
  `src/locales/*.ts` by `slint/tools/seed-setup-i18n.py`.

Not yet migrated: `AccountsView`;
`views/game-placeholder.slint` stands in for the not-yet-migrated views. The
account avatars (the footer's 56px head, its switcher's 18px rows and the
add-account dialog's profile rows) all draw the real skin now; `AccountAvatar`
still falls back to the placeholder disc for a skin Rust could not decode.

- **Instance settings overlay deviations**, all deliberate:
  - **The write is debounced by 400 ms and the app is never locked.** The Vue
    adds `saving-instance-settings` to `<body>` on every edit, which
    `main.css` turns into `pointer-events: none !important` over the whole app
    until the write resolves, and `updateInstance` runs once per keystroke. A
    Slint write is a synchronous call on the UI thread with no IPC in front of
    it, so there is no window for a second edit to slip into and nothing to lock;
    what is left is the cost of one `toml` write per keystroke, which the debounce
    the settings screen already uses absorbs. The instance travels with the
    scheduled write rather than being looked up when it fires, so closing the
    overlay and selecting another instance in between cannot write the edit to the
    wrong one.
  - **`java_path` survives a write.** The Vue replaces the whole `launch_config`
    with an object literal on every save, and `java_path` is not a field of its
    TypeScript type, so the original silently drops the instance's Java override
    whenever any setting is touched — including a rename. The field is read by the
    launcher (`instance_java_path`), so it is carried over here instead.
  - **A rename asks the game view to read the list again.** The Vue edits the very
    object its Pinia store holds, so the summary and the instance's list card
    re-render off it for free; here the store is on disk, so `GameState.refresh()`
    is called — and only for the edits the game view actually shows (the name and
    the two flags the window background resolves on), so a keystroke in the JVM
    arguments does not rebuild the list. The one visible consequence is that an
    instance whose name was changed re-sorts if the list is sorted by name, which
    the original leaves in place until the next `loadInstances`.
  - **The card's background is decoded, not handed to `Image::load_from_path`.**
    The instance keeps the picture at a bare `background` with no extension, and
    Slint's `image-default-formats` covers png and jpeg only while the app's own
    `image` dependency also reads webp and gif — so this goes through the same
    decode as the window background, and a format none of them reads (avif, svg,
    bmp, ico) still stores and still shows as no picture, which is the
    create-instance dialog's documented limit too.
  - **The `Minecraft` row and the `Reset instance` row stay inert.** Both are
    `navigable` in the Vue — they light up, grow a chevron and take the hover
    background — and neither binds a handler; the mod loader and loader version
    rows below the first are commented out in the original. Reproduced as it is,
    rather than quietly wired to something.
  - **The file picker's filter label is translated.** `InstanceSetting.vue`
    hard-codes the English `"Images"` — one of the strings `AGENTS.md` lists as
    not yet internationalized — where the create-instance dialog translates the
    same filter. The Slint app follows the latter.
  - **`SettingGroup` and `SettingCollapse` grew a `disabled` state.** The launch
    options, the memory group and the advanced options are inert until the
    instance overrides the launcher's own launch settings
    (`.setting-group-disabled` / `.setting-collapse-disabled`: `opacity: 0.6` and
    `pointer-events: none` on everything). Slint has no inherited "ignore the
    pointer" switch, so the rows are covered by a bare `TouchArea` instead —
    declared after them, and rejecting the wheel so the scroller still takes it.
    The collapse's header uses `clickable: false` rather than a cover, which also
    takes the hover fill with it, as CSS `pointer-events: none` does.
  - **The delete dialog's four strings are translated.** They are hard-coded
    Chinese in the Vue (that dialog was never internationalized), so the Chinese
    is kept as the `@tr` source text and the catalogs translate it — the treatment
    the four launch error dialogs already have. Its warning paragraph's
    `line-height: 1.3` is dropped: Slint 1.18 has no CSS `line-height`, and
    `line-height-factor` multiplies the font's *natural* line height rather than
    the font size, which is not in a fixed ratio to it (Comfortaa's is 1.115em and
    the CJK fallback this sentence actually draws with is nearer 1.4em), so one
    factor cannot hold both scripts. Its buttons' 4px radius needed one new
    `BaseButton` override (`button-radius`), the same kind the multiplayer dialog's
    red "Stop download" button needed.
  - The **`image` icon is missing its frame and its sun**, so the "Set background
    image" row draws the bare mountain range of `src/assets/icons/image.svg`:
    `icons.slint` is transcribed by hand and the file is one of the ten
    `slint/tools/check-icons.py` reports as differing, because the checker's
    `d`-only reading cannot see a `<rect>` and this icon's frame is one. Left
    alone here (it is the icon the already-migrated settings and create-instance
    screens draw too) rather than fixed inside an overlay migration; the fix
    belongs in the icon pass, where `--fix` can be taught the missing shape.
    **Fixed since**: `icons.slint` is generated from the SVGs now, so the frame
    and the sun are there.


## Rendering the README bodies

A project detail panel shows the project's README (`Modrinth`'s `body`,
`CurseForge`'s description markup). The Vue renders it with `marked` into
`v-html`, so headings, lists, code blocks, links, images and tables all come out
styled by `.markdown-body`. **Slint has nothing that can render an arbitrary
Markdown document**, so this is its own crate: `slint/crates/markdown` parses the
body in Rust (`comrak`, which also takes CurseForge's HTML), measures it with the
same text engine Slint renders with (`parley`), lays every run out itself, and
hands `overlays/content/markdown-body.slint` a flat list of positioned boxes to
paint — with the run's `y`s in the view's own coordinate space, because Slint
offers no baseline to place a run on and no line box to grow. The crate's own
`README.md` is the reference; this is only where the decision sits.

The shape is the one the rest of the app already uses: plain data in a model,
`for` in the view. `GameRow` is the same idea. It needs no browser engine, every
string stays translatable, and the model is where the panel's own images are
spliced in.

What it costs against the Vue, all of it recorded in the crate's README:

- **No text selection or copy.** The display list has shaped runs, not source
  spans, so there is nothing to map a pointer back to a selection range — and
  the Vue's `.markdown-body` explicitly *allows* selection (`:deep(*) {
  -webkit-user-select: unset }`), so this is a real loss.
- **No horizontal scrolling.** `pre { overflow: auto }` and `table { display:
  block; overflow: auto }` both scroll in the Vue. A table wider than its box is
  scaled to fit with wrapping cells, and a code block wraps or is clipped.
- **Raw HTML is parsed, not passed through.** Safer, but different: the
  CurseForge path went through `v-html` before, and a `tagfilter` now escapes
  `<script>`, `<style>`, `<iframe>` and friends to text.
- **`line-height: 1.6` is reproduced by the engine, not by the view.** 1.18 has
  no CSS `line-height`; `Text` grew `line-height-factor`, but it multiplies the
  font's natural line box rather than the font size, and `StyledText` has no such
  property at all.

And what it gains, which the Vue did not have: GFM footnotes, `^superscript^`,
description lists, and a `<details>` drawn as the app's own collapsible row
rather than a bare browser triangle.

The two options that were weighed and rejected are worth keeping on the record
because they are the obvious ones:

- **`StyledText`'s `@markdown`.** Slint has a Markdown-ish rich-text element and
  the About tab already uses it. It is not usable here: `@markdown` takes a
  compile-time literal the parser reads out of the source map, so it cannot be
  handed a document that arrives at runtime.
- **An offscreen webview rendered to an image.** `wry`/`tao` are in the
  workspace, so the HTML could be laid out offscreen, rasterised and handed to
  Slint as a bitmap. The rendering would be exact — and it would pull a browser
  engine into the binary the migration exists to remove, break scrolling and
  selection, and make the body unstyleable by the palette.

## Music player

The background-music player is the one place where the _architecture_ of the Vue
original is a webview detail, so it is worth spelling out where the pieces went.

`crates/music` is a Tauri plugin whose only command lists the music folder. The
rest of the player — decoding, the output device, the analyser, the transport, the
playlist, the saved position — lived in `src/store/music.ts`, driving an
`<audio>` element through the Web Audio API. A native app has no webview, so
`slint/crates/music` is the mirror of `crates/music` _plus_ everything the
webview's store owned, split the way the store's responsibilities were:

| `slint/crates/music` | the Vue's                                                                            |
| -------------------- | ------------------------------------------------------------------------------------ |
| `lib.rs`             | `crates/music` verbatim: `list_music_files`, `MusicFile`                             |
| `decode.rs`          | the `<audio>` element's `src` — a file becomes PCM                                   |
| `analyser.rs`        | the `AnalyserNode`, down to the Blackman window and the 0.8 smoothing                |
| `player.rs`          | the graph (`source → analyser → gain → destination`) and the `useMusicStore` actions |
| `session.rs`         | the `localStorage` entry the position was kept in                                    |
| `error.rs`           | `crates/music/src/error.rs`, minus the Tauri IPC derives                             |

The graph is `decoded track → resampler → (analyser tap) → gain → output device`,
in that order because the order is visible: the analyser sits **before** the gain,
so the footer visualizer is unaffected by the volume, exactly as in the Vue. The
resampler is the one new stage — the element was handed the device's own pipeline
and never had to convert rates, while a 44.1 kHz track on a 48 kHz device would
otherwise play 9% fast.

**The threads.** The device pulls samples on cpal's callback thread, which only
touches the graph under one lock: the cursor, the gain ramp and the seek all have
to be instantaneous. Decoding happens on a worker thread and swaps the track in
over a channel; the callback reports the end of a track back to it, which is the
`audio.onended` the store handled in `handleTrackEnded`. Reads (`state()`,
`spectrum()`) are a lock and a clone, so the UI can poll as often as it likes.

**What stays in the app** (`app/src/music.rs`) is the app's and not the player's:
the configuration (the two volumes, the enable switch) and the window focus the
background volume follows — the store's `init` and its two `watch`es; the panel's
`panelOpen`/`showPlaylist`, which the store only held so the title bar could reach
it; and `BeatMap.vue`'s mapping of the analyser's bins onto bars, which is a
component's own maths and cannot be a Slint binding (see Known issues). The player
itself is not `Send` (cpal's stream is not), so it lives in a `thread_local` and is
only ever touched from the event loop's thread — the same reason `content.rs`
keeps its caches there.

**The clock.** The state is pushed into `MusicState` on a 16ms tick that runs on
the app's own runtime, not on a Slint `Timer`: a Slint timer stops the moment the
window goes quiet, which is exactly when the progress bar has to keep moving. The
tick only runs while something is moving — a track playing, a selection decoding,
or the panel open — and `PlayerState::pending` is what covers the second case, or
the very first play at startup would start silently. This is the same clock
`background/controller.rs` runs the camera on.

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
      title bar's ⌘/ hotkey is still a logging placeholder — so the viewer is
      driven by its two arrow buttons, its thumbnails and its close button.
      Its GSAP intro and outro (`playIntro` / `playOutro`) *are* there.
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
- **Boxes the port made taller than the stylesheet's, on purpose.** Slint 1.18
  has no CSS `line-height`: `Text` grew `line-height-factor`, but that multiplies
  the *font's* line box (ascent + descent + line gap, ~1.115em for Comfortaa
  Nunito) rather than the font size, so it is not a drop-in for a stylesheet's
  `line-height: 1` — the original's `* { line-height: 1 }` boxes a line at its
  font size, and Slint's is a shade taller. Two boxes are therefore grown rather
  than matched:
    - the instance list's group-count pill and loader tag, `13px` where the CSS
      box is `10px + 1px × 2 = 12px` (`views/game/instances-list.slint`), and
    - `SettingItem`'s pinned description line, `18px` where the CSS is `13.2px`
      (`components/setting-item.slint`).

  In both cases the CSS number would clip the text. This is the same shape as
  the `for`-built row heights above, which are pinned for a related reason.
  Everywhere else the port takes the stylesheet's number.
- **A `StyledText` has no line height at all** in 1.18 — not even
  `line-height-factor`, which `Text` has and `StyledText` does not — so the About
  tab's disclaimer (`font-size: 9px; line-height: 1.5` in
  `SettingsAbout.vue`) renders at the embedded font's natural ~10px leading and
  comes out a few pixels shorter than the original over its three or four
  lines. The one rich-text string in the app, and the only place it shows.
- **An element that carries `clip` *or* an `opacity` below 1 bounds its
  children's rendering to its own box.** `clip` is explicit about it, and it is
  the reason the palette tile's 4px selected outline and its label had to be
  drawn outside the 60px tile it belongs to. `opacity` does it silently: a
  translucent element is composited through a layer the size of that element,
  so a child that overflows is simply not in the layer. The palette tiles set
  `opacity: 0.6` on the component root when *Follow system dark mode* is on,
  and the label — which is the Vue's `margin-top: calc(100% - 20px)`, ten
  pixels below the tile — vanished with it, so the palette showed four
  unlabelled tiles for as long as that switch was on and looked fine the moment
  it was turned off. The Vue is `.color-style-disabled * { opacity: 0.6 }`: the
  opacity belongs on each **descendant**, which is both what it says and the only
  arrangement where nothing that dims has a child escaping its box. The general
  rule: an element that clips or dims must not have a child drawn outside it —
  put the opacity on the child.
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
    - **`x` and `y` already default to centring.** An element outside a layout
      with no `x`/`y` binding is placed at `(parent.width - self.width) / 2` and
      `(parent.height - self.height) / 2` — the values that centre it. Writing
      that formula out is a no-op, and the port had 137 copies of it before a
      pass took them out. (Inside a layout it is not a no-op in the other
      direction: see the `x`-on-a-child note below.)
    - **So `x: 0` is only dead when the width really is the parent's.** The
      centring default above is `(parent.width - self.width) / 2`, which is `0`
      *only* when `self.width == parent.width`. The music player's progress fill
      binds `width: parent.width * progress`, so deleting its `x: 0` as a
      no-op centred the drawn bar inside the track instead of pinning it left —
      a half-drawn bar floating in the middle. An unbound `x` is a positioning
      decision, not a stylistic one; the sweep that removed the 137 copies
      guarded on the width being the full parent for exactly this reason, and
      the guard is the part worth keeping.
    - **A layout child cannot set `x` or `y` at all.** Not "is overridden" —
      Slint *rejects* it: "The property 'x' cannot be set for elements placed in
      this layout, because the layout is already setting it." So an element that
      slides in on a `y` or an `x` cannot be the layout's child. It can be a
      child *of* the cell the layout places, which is where the intro slides in
      `views/game/instance-summary.slint` and the cards' entrance in
      `overlays/music-player.slint` ended up.
    - **An overlay is not a layout child.** A `TouchArea` that covers a whole
      row belongs to the row's *parent*, not to the row: as a layout child it is
      a column like any other, it takes its share of the free space away from
      the element that wanted it, and a `width: 100%` on it hands it the entire
      row as its *preferred* size besides. That is what the sort/group
      dropdown's `head-touch` was doing — every layout mistake in this file
      arrived through it at once, and the symptom was never a missing button but
      a label squeezed to `…` with its chevron stranded mid-dropdown. A
      `Rectangle` fills its parent by default and a layout does not, which is
      the same distinction as the note below, and the reason a *nested* layout
      needs an explicit `width`/`height` to fill the `Rectangle` around it.
    - **A `TouchArea` defaults to 100% of its parent only *outside* a layout.**
      Inside one it is laid out like any other child, so with no size of its own
      its preferred size is 0x0 and it is a zero-area hit target that looks
      perfectly fine. This is the sort/group dropdown: its `head` was a
      `Rectangle` in the baseline and became a `HorizontalLayout` here, and the
      `TouchArea` that makes the whole component clickable went with it — the
      two dropdowns stopped responding and nothing looked wrong. The baseline
      carried `width: parent.width; height: parent.height` on it, and dropping
      those looked safe because the doc sentence above is true *most* of the
      time. Same trap as the `x: 0` sweep, one level down: a default that
      holds outside a layout does not hold inside one.
    - **`z` is global to the window, so declaration order only orders what has
      no `z` of its own.** Mounting the dropdown panel *after* the page stack
      puts it above every element that does not opt into a `z`, and nothing
      more: the instance list's toolbar (`z: 114`) and both scrollbars
      (`z: 500`) are exactly what a toolbar dropdown opens over, and at the
      default `z: 0` the panel was painted underneath the list it belongs to.
      The Vue's `.dropdown-list` is `z-index: 100000`, above the list's 114,
      so the fix is to carry that number's *position* rather than a number
      picked to clear the offenders — and then anything the Vue stacks above
      the dropdown (the dialogs, 11451419) needs one too, because it was
      relying on being declared later. The z's are sparse enough here that
      they have to be read as a scale, not as local tweaks.
    - **`box-sizing: border-box` is set globally, so padding and border do not
      grow a box.** `src/assets/styles/main.css` has `* { box-sizing:
      border-box }`, which makes every `padding`/`border` in the Vue *inside*
      the size it is applied to. Reading the footer's avatar as content-box —
      `:size="56"` plus `padding: 2px` plus a `border: 2px` — gave 64px and an
      8px-too-big circle, and dragged the 18px overhang to 22 and the account
      pill out to 76 with it, since both are measured from where the avatar
      ends. `BaseSliderBar.vue` opts back into `content-box`; almost nothing
      else does, and the difference is worth checking before doing arithmetic
      on a box.
    - **A child's `preferred-width` may not read its parent's width.** Stating
      "exactly half the row" as `preferred-width: (other-row.width -
      other-row.spacing) / 2` is a binding loop — the preferred size feeds the
      layout cache that decides the width it just read — and Slint reports it
      as one the moment anything else in the subtree asks the layout for its
      own size. `flex: 1` is a zero basis and a grow of 1, so the Slint
      spelling is `preferred-width: 0` plus `horizontal-stretch: 1`: equal
      floors and equal factors make the children exactly equal *whatever* the
      floor is, and no floor has to know the row's width to say so.
    - **A stretch factor of 0 is the one you have to write, and 1 is the
      default.** Not a niche setting: *every* layout child grows by default, so
      an element the CSS pins (`flex-shrink: 0`, a fixed width, a `width:
      fit-content`) silently starts competing for the row's free space the
      moment it is put in a layout, and the free space is split by the factors
      rather than given to the one element that wanted it. It cost three
      separate bugs before it was written down: `BaseButton` and `BaseInput`
      filling a row they were only meant to size themselves to, the sort/group
      dropdown's label taking half its own dropdown away from the selection and
      pushing the chevron off the right end, and both clock labels growing
      alongside the music player's progress bar until the duration sat on top of
      it. Two things make it worse: the factor reads as opt-*in* because
      `horizontal-stretch: 1` looks like an instruction rather than the
      default, and CSS expresses the same idea in the opposite direction — a
      child's `flex: 1` is something the *parent's* `justify-content` honours,
      whereas Slint's factor belongs to the child and defaults to growing.
    - **A main-axis alignment other than `stretch` cancels every stretch
      factor.** `horizontal-stretch: 1` only has meaning under the default
      `alignment: stretch`; write `alignment: start` (or `end`, `center`) and
      the layout hands each child its *preferred* size and distributes the free
      space to nobody. A row of two `horizontal-stretch: 1` buttons written that
      way packs both at the left of the row at their text widths instead of
      splitting it — which is also why ordering matters: only the element
      declared *last* can be pushed to the far end by a stretch, so a
      field-then-button row has to be written in that order and cannot be
      rescued with `alignment: end` (that packs the whole group at the end and
      leaves the row unfilled).
    - **A layout child under a non-stretch alignment is sized by its
      *preferred* size, and a bare `Rectangle`'s preferred size is 0.** This is
      the one that bit twice: the `cell` wrappers
      `views/game/instance-summary.slint` puts between the stack and each
      sliding row are plain `Rectangle`s, so `cross-axis-alignment: start`
      collapsed every cell to 0x0, the row inside it then took the *centring*
      default `x`/`y` of a zero-sized parent (half its own width off to the
      left), and `width: stack.preferred-width` on the component root went to
      zero as well. Each cell now binds `preferred-width`/`preferred-height` to
      the row it wraps. Keep the `x`-bearing slide *below* the cell, and give
      the cell the size.
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
      deliberately, not as its longest line. The same switch is why a row holding
      a `wrap: word-wrap` `Text` under `alignment: start` never wraps at all: the
      text's *preferred* width is the whole unwrapped line, which is what it is
      given. (`min-width` is the longest word, so `stretch` shrinks it and it
      wraps; a `Text` that neither wraps nor elides has
      `min-width == preferred-width` and does not care either way.) The fix
      several rows here use instead is `cross-axis-alignment: stretch` with
      `horizontal-alignment: center` on the text — the glyphs land in the same
      place as a shrink-to-fit box, and the wrap has a width to happen at.
    - **A child with no `x` or `y` is *centred* in its parent, not placed at
      its top-left.** `i-slint-core`'s default geometry is `x = (parent.width -
      width) / 2`, `y = (parent.height - height) / 2` — which reads as a
      deliberate choice for a lone child and is, in fact, how Slint marks "this
      element does not care". A `Rectangle` child with a fixed `y` is at that
      `y`; one without is in the middle. The palette's row put its title line in
      a plain `Rectangle` with no `y` and the line landed 8.5px low — exactly
      half of what the 30px column had spare, which is the tell — and a second
      time, inside a `FocusScope`, the whole input row sat in the middle of the
      panel. Every child of a non-layout parent needs an explicit `y`; the
      palette's says so where it lays them out.
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
- **A wrapping `FlexboxLayout` measures itself at its own "roughly square"
  width.** `flexbox_layout_info_main_axis` (`i-slint-core/layout.rs`) computes a
  wrapping layout's preferred width as √(total area) rather than its longest
  line, deliberately, so that a wrapped row comes out roughly square — which is
  fine when the layout is *given* the width it draws at, and wrong when a parent
  asks it for a height: the search panel's category row reserved two lines where
  it drew one, and every row below it sat a line too low. (The chips' own widths
  are exact — a chip reports what it measured — so the wrap is computed in Rust
  instead; `content.rs`'s `filter_row_height`.) The same trap catches CSS's
  `flex: 0 1 auto` chip run, which is why the Vue's rows do not have this
  problem: the browser wraps at the width the row was given.
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
    - The Microsoft **auth-code flow comes back to a loopback listener**, not to
      the `conic-launcher://` scheme the Tauri app registers with
      `tauri-plugin-deep-link` + `tauri-plugin-single-instance`. The screen is
      1:1 and "Log in" opens the browser; what changed is behind it, and both
      flows now complete — see `slint-authcode` under Migrated so far for the
      port, the `state` check, and the page the browser is shown. Two visible
      differences:
      - **A refusal comes back at all.** Microsoft's consent screen sends
        `error=access_denied&error_description=…` rather than no code at all,
        and the listener reads it. The Vue's `onOpenUrl` only ever read `code`,
        so a refusal there was silence. A refusal and a timeout now both return
        to the browser screen, which rebinds a fresh listener — the error screen
        has no buttons (the Vue's does not either), so it is a dead end, and the
        browser has just shown a page that says what happened. This is also what
        the device-code flow does with `DeviceCodeExpired`.
      - **The URL is not a constant.** It is filled in when the listener is
        bound, so the link box and the "copy the link" span show the one that is
        being served. The listener is bounded by `AUTH_CODE_TIMEOUT`, the life
        of Microsoft's authorization code, and released the moment the code
        arrives, the user switches to the device code, or the dialog closes.
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
      player's `pause_on_launch` is wired for real.
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
- **Music player deviations**, all deliberate:
    - **A track is decoded in full, on the worker, before it plays.** The element
      streamed, which is the right shape for a long file the user seeks around in,
      and the wrong one here: the analyser needs the signal the way the element fed
      it, the position has to be readable and seekable at any moment (it is
      persisted every five seconds and restored at startup), and the _duration_ is
      what the panel's right-hand clock shows — which for a VBR MP3 is only exact
      once the whole file has been read. The cost is a pause between tracks, and
      the pause is short: symphonia decodes these at 340–1750x realtime on this
      machine (a 2-minute MP3 in 0.35s in release, ~1.5s in a debug build), and the
      work happens on a worker, so the panel keeps running. The other cost is
      memory: the samples are held as `i16`, so a two-minute stereo track costs
      ~42 MiB and a long WAV its own file size. The webview buffered the whole
      file too, so this is the same shape at a size that can be named.
    - **The saved position is a file, not `localStorage`.** `{ path, currentTime }`
      goes to `music_session.json` next to `config.toml` in the shared data
      directory, which is the only app-level store a native app has. It is the one
      piece of the player the two frontends do not share: the Tauri app cannot read
      it, and this one cannot read the webview's storage.
    - **The OS media controls are not wired.** The store registered `play`,
      `pause`, `previoustrack`, `nexttrack` and `seekto` through the Media Session
      API, so the system's media keys and the macOS Now Playing widget drove the
      player. There is no equivalent in Slint; it needs MPRIS on Linux, SMTC on
      Windows and `MPNowPlayingInfoCenter` on macOS, and none of those is
      reachable from a Slint application. The player's own API is complete for it.
    - **The `title` tooltips are accessible labels.** Slint has no tooltip on an
      icon button, so the eight `@tr`'d titles of the transport row and the
      playlist's `title` became `accessible-label`s. They are read out; they are
      not drawn.
    - **The progress bar's drag stops at the bar.** The Vue registered
      `pointermove`/`pointerup` on `window` for the duration of the drag, so the
      bar kept following the pointer once it had left the bar itself. Slint cannot
      listen outside an element, so the drag is tracked inside it: a press still
      seeks, which is what a click does, but the pointer has to stay on the bar.
    - **`opus`, `wma` and `aiff` are listed but not decodable.** The extension list
      is the original's, all ten, and it still decides what the playlist shows.
      Symphonia has no decoder for those three, so a track in one of them selects,
      shows its name, reports an error and plays nothing — which is what the
      webview did with a file its engine could not play. `mp3`, `wav`, `ogg`,
      `flac`, `m4a`/`aac` and `alac` all decode. The extension list is not narrowed
      to hide this: whether a file decodes is a property of the file, not of its
      extension.
    - **A decode failure is a log line.** The store reported it through
      `console.error`, which had no counterpart; the crate logs it and the track
      stays selected, so the panel and the playlist still show it.
    - **The visualizer is drawn in logical pixels, not device pixels.** The Vue
      sized its canvas to the container times `devicePixelRatio` and let the CSS
      scale it back down, so its two absolute constants — the 1px floor on a bar's
      height and the 2px idle baseline — were _device_ pixels (0.5 and 1 CSS px on
      a Retina display). Slint has no device-pixel grid, so they are 1px and 2px
      in logical units, and the idle baseline is twice as thick on a Retina screen
      as the original drew it. Everything proportional is unaffected: the bar count
      comes from the CSS width, and the gap is a quarter of the bar's width, which
      lands on the same CSS value either way.
    - **The spectrum-to-bars maths is in Rust** (`music::BeatMap`), not in
      `beat-map.slint`. It cannot be a Slint binding: every bar is normalised
      against the loudest _bar of the frame_, which needs a pass over all of them
      before any of them can be drawn, and the decaying peak is state only a
      callback may write. What the component draws is what the Vue drew — the same
      bins, the same logarithmic spread, the same dB floor, the same 0.97 decay —
      from the levels it is handed. This is the same trade the content grid's
      `filter_row_height` makes.
    - **A gradient is relative to the element it fills, so each bar draws its own
      share of the canvas gradient.** The Vue built one
      `createLinearGradient(0, height, 0, 0)` for the whole canvas and every bar
      sampled the part of it the bar covered — which is what `level`, a fraction of
      the canvas height, is for: a bar's head is `lavender` mixed with `blue` by
      its own height, so the colour at a given height is the same in both.
    - The playlist popup's fade and slide are its own phase machine, not the
      panel's. `<Transition name="playlist-fade">` is a *second* transition, so a
      toggle while the panel stays open has to animate on its own — and a `v-if`
      would take the card away before anything could, which is why it stays
      mounted for as long as its phase is not "closed" (the same two rules as
      `BaseDialog`).
    - **The live and idle bars are two loops, not one.** A gradient cannot share a
      `background` ternary with a plain colour — the two do not unify into a brush,
      and every bar comes out invisible. The idle loop iterates `bar-count` (an
      integer _is_ a model in Slint, which is what it repeats over) because
      `levels` is empty while there is no spectrum to read.
    - **The playlist card's height is computed, not measured.** The list is built by
      a `for`, and a repeated element's height is measured at its own preferred
      width, so a measurement would not be the height the layout uses — the trap
      `filter_row_height` and the `SettingCollapse` clip work around. The row
      height is a constant in the component instead.
    - The footer's `backdrop-filter: blur(4px)` is dropped, like the toolbar's and
      the launch panel's (Slint 1.18 has no backdrop blur), so the bars show
      through the bar's own `surface0` at 40% unblurred.
    - **`showPlaylist` outlives the panel.** The Vue's is a `ref` inside
      `MusicPlayer.vue`, which stays mounted for the whole session; only the inner
      `v-if` on `panelOpen` comes and goes. So `closePanel()`, the title bar's
      button and the enable-switch watcher all leave it alone, and reopening the
      panel brings the playlist back as it was — with its own enter, because the
      overlay's `v-if` rebuilds the whole subtree. Rust used to clear it on all
      three, which silently lost the setting.
- **A percentage on a component's own root is not the parent's size**, which is
  why the visualizer is sized by its caller. `.beat-map.fill` is `position:
  absolute; inset: 0` — it fills the *footer* — and the obvious
  `width: 100%; height: 100%` in `BeatMap` came out as **840×100**: the width
  re-resolved during layout (something read it, so the dependency was
  registered), the height never was, and it kept the 100px it fell back to when
  the wrapper a conditional puts around a child — `if AppConfig.show-visualizer :
  BeatMap` — had no size of its own. The bars then measured themselves against
  100px and were drawn 22px above the footer. The size is given at the one place
  that knows the footer's box (`views/game/footer.slint`, as `parent.width` /
  `parent.height`), which is the honest translation of `inset: 0`. Worth
  remembering for any component mounted under an `if` that has to fill something.
- **`icons.slint` is generated, not written.** `slint/app/build.rs` reads every
  SVG under `src/assets/icons/` (plus the two brand marks in
  `src/assets/images/`) with `usvg` — the same crate Slint rasterizes them with,
  reached through `resvg` — and writes the `Icons` global the app imports. So
  the table cannot drift from the SVGs: attribute inheritance, `<rect rx>`,
  `<circle>`, `<ellipse>`, `<line>` and `<polyline>` are all resolved by the
  library that will draw them, rather than by a `d`-only reading of the file.
  **Adding an icon is dropping the `.svg` into `src/assets/icons/` and
  rebuilding** — the generator re-scans the directory (a `rerun-if-changed` on
  it, so a new file triggers it) and every icon in it becomes a valid
  `AppIcon { name: "..." }`. The file is gitignored; it is a build product.
  The one thing a generator cannot catch is a *name* that does not exist —
  `Icons.stroke-commands()` returns `""` and the icon is simply missing, with
  nothing logged — so `slint/tools/check-icon-names.py` asserts every name any
  `.slint` asks for has an SVG behind it.

  This replaced a hand-transcribed table, which had drifted: ten icons had lost
  their `<circle>` geometry (`gamepad`, `branch`, `palette`, …) and seven more
  had lost every `<rect>`/`<line>`/`<ellipse>`/`<polyline>` — `apps-outline`,
  `server`, `bell`, `badge-check`, `arrow-down-tray`, `package` and `image` drew
  *nothing at all*. `warning` was worse than missing: its `!` stroke begins with
  a lower-case `m` (a relative move from the origin) that the transcription read
  as a mid-path move, so the exclamation mark sat in the wrong place.

  Three glyphs stay hand-written because they are not icons: the
  minimize/maximize/close window controls in `title-bar.slint` (drawn to the
  platform's own metrics) and the placeholder artwork in `game-placeholder.slint`.
  `base-loading`'s arc and `item-loading-icon`'s three states are animation
  frames, not a set.
- **The `pcm` feature of symphonia is a codec of its own, not part of `wav`.**
  Without it symphonia reads a WAV header and then has no decoder for the samples
  it found, which fails every WAV in the folder with "unsupported codec" — the one
  thing a music folder is most likely to be full of.
- **Setup wizard deviations**, all deliberate:
  - **The card's padding is the scroll area's, not the card's.** `.body` is
    `padding: 24px 36px; overflow: hidden` around the wizard's own `ScrollView`,
    and `ScrollView.vue`'s `.scrollbar` is `position: absolute; right: 8px`
    against the nearest positioned ancestor — the card. So the Vue's scrollbar
    lands 8px from the *card's* edge, inside the 36px of padding. A Slint
    `ScrollView` pins its scrollbar to its own right edge, so the padding is put
    on the scroll area's content instead, which puts the scrollbar where the
    Vue's is. The card keeps the border-radius, the fill and the clip.
  - **The language grid is laid out by hand.** `grid-template-columns:
    repeat(auto-fill, minmax(100px, 1fr))` has no counterpart: `GridLayout`'s
    `col`/`row` bindings have to be compile-time constants, so an `auto-fill`
    track list cannot be expressed, and a wrapping `FlexboxLayout` measures
    itself at √(total area) rather than at the width the `ScrollView` gives it —
    which is exactly the trap the content panels' `filter_row_height` works
    around. The track count and the column width are computed instead
    (`floor((width + 8px) / 108px)`, and what is left over divided between
    them), which is what the browser does with the same two rules.
  - **The language screen's own intro is not played.** The screen exposes it with
    `defineExpose({ playIntro })` — a staggered `opacity: 0, scale: 0.8` over the
    title, the three paragraphs and the twelve buttons — and nothing ever calls
    it: `SetupView.vue` takes the ref and plays only its own header/body/footer
    timeline. Dead code is not carried over; `SetupView`'s intro is.
  - **The import button is a dead button, as it is in the Vue.**
    `.import-from-other-launcher` has no `@click` in the original, so importing
    from another launcher is a feature the app does not have yet. It is ported
    as the button it is rather than dropped, so the screen still looks the way
    it does.
  - **The Java screen's inner `ScrollView` is not reproduced, and nothing scrolls
    differently without it.** `.wrapper { height: 100% }` resolves against an
    auto-height block parent, so that scroll area grows with its content and
    never scrolls — the card's own `ScrollView` is what moves. One scroller is
    also the better arrangement, since a nested one would take the wheel.
  - **The add-account screen fills the card, and that changes nothing.** The
    wizard's `:deep()` rules `.account-add-container, .auth-code { height:
    100% }` and `.add-microsoft-account-container { flex: 1 }` only stretch a
    top-aligned block over the box it already starts at the top of, which is what
    `alignment: start` does. The two rules that do change something — the
    padding and the hidden "Cancel" buttons — are the `wizard-host` flag.
  - **`.wizard-message`'s 1.5 line height is 1.115× taller than the
    stylesheet's.** 1.18 has no CSS `line-height`, and `line-height-factor`
    multiplies the font's natural line box rather than the font size, so 19.5px
    comes out as 21.7px. The same trade the rest of the port makes; the
    paragraphs are one line taller per wrapped line than the Vue's.
  - **The profile card's avatar has no drop shadow.** The Vue's `.avatar
    { filter: drop-shadow(0 0 8px rgba(0, 0, 0, 0.5)) }` follows the element's
    rendered shape, so on a round avatar it is a round shadow. Slint's
    `drop-shadow-*` draws the shape of the element's *box* — the same four
    properties put a square 56×56 shadow behind a circle — and there is no mask
    to soften it with, so the shadow is dropped rather than drawn wrong.
  - **The profile card's type colour is the switch's other half.** The Vue has
    three mutually exclusive classes (`.microsoft`, `.yggdrasil`, `.offline`)
    and an account is always one of the three, so the colours are chosen from
    `GameState.current-account-kind` instead.
  - **The button bar has no backdrop blur** (the same as the toolbar's and the
    game view footer's): Slint 1.18 has none, so the card scrolling under it
    shows through its own `surface0` at 60% unblurred.

- **Icons**: the original uses Font Awesome Pro (`fa-pro`), which can't be
  shipped, so the set is the free subset the Vue app already ships in
  `src/assets/icons/`. `ui/icons.slint` is **generated** from those SVGs by
  `slint/app/build.rs` on every build (see the note on `icons.slint` above), so
  a call site is `AppIcon { name: "folder"; }` and the geometry follows the SVG.
  `music-folder` (a `fa-pro` glyph in a different style) and `icons.svg` (an
  unreferenced multi-glyph sprite) were removed rather than ported.- The placeholder view contains dev-only English strings; real localized text
  arrives with the actual views.
