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
    src/main.rs                     # Rust host: platform setup, window chrome, locale
    ui/
      app.slint                     # root `App` Window (mirrors src/App.vue)
      theme.slint                   # palette + typography tokens, embeds fonts
      icons.slint                   # icon path data (mirrors src/assets/icons/*.svg)
      fonts/Comfortaa.ttf           # converted from src/assets/fonts/*.woff2
      fonts/Nunito.ttf
      assets/                       # palette previews + about logos (png/svg)
      globals/
        navigation.slint            # global page navigation (src/store/navigation.ts)
        settings.slint              # global config state (the "config store")
        game.slint                  # game view state (src/store/instance.ts + …)
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
  crates/
    platform/                       # Tauri-free mirror of crates/platform
    window/                         # window-control service (min/max/fullscreen)
    config/                         # Tauri-free mirror of crates/config
    folder/                         # Tauri-free mirror of crates/folder
    account/                        # Tauri-free mirror of crates/account
    instance/                       # Tauri-free mirror of crates/instance
    content/                        # Tauri-free mirror of crates/content (counts)
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
numbers render in Nunito and everything else falls back to Comfortaa. Both were
variable `.woff2` fonts. Slint embeds fonts at compile time but only accepts
`.ttf`/`.ttc`/`.otf`, so they were converted with `fonttools`:

- `Comfortaa.ttf` — variable (wght 300–700), family `Comfortaa`.
- `Nunito.ttf` — instanced at weight 400, family `Nunito`.

They are imported in `ui/theme.slint` (`import "fonts/*.ttf";`), and the root
window sets `default-font-family: Theme.font-family`. Slint has no
`unicode-range` fallback, so numeric text must opt in explicitly with
`font-family: Theme.digits-font-family`.

Typography tokens live in the `Theme` global (`font-family`,
`digits-font-family`, `font-size-base`, `font-weight-normal/bold`).

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
  a `GameState.now-ms()` callback because Slint has no wall clock). A `Timer`
  drives `scroll-y` instead of a `Flickable`, which would layer its own
  fixed-duration wheel animation on top and fight the programmatic scrolling the
  view needs (centring on open, gliding to a selection, freezing during a group
  collapse). Wheel events are captured by the `TouchArea` wrapping the content:
  the cards' own `TouchArea`s reject scroll events, so the wheel bubbles to it.
  The same curve carries every list change — the rows' `y` is animated and their
  `x` is a pure function of it, so a reorder travels along the rail exactly like
  the gsap `railKeyframes` sampling `parallaxX`. Rows are reconciled in place by
  `uid` in `game.rs`, so their items survive a relayout; a collapsed group keeps
  its cards parked on their header's slot at `opacity: 0` so they fade and glide
  instead of being destroyed. The custom scrollbar, the toolbar and the
  `GameIntro` timeline (the Vue `onMounted`/`playIntro` GSAP entrance, staggered
  through the `delay` of each `animate`) are ported with it.
- `slint-platform` (OS detection), `slint-window` (window controls),
  `slint-config`, `slint-folder`, `slint-account`, `slint-instance` and
  `slint-content`.

Not yet migrated: `LaunchView`, `AccountsView`, the setup wizard, the overlay
layer (dialogs, content panels, command palette, music player, instance
settings), the background renderer, and the full Java runtime scanner (settings
use a lightweight subset). The game view's click-to-open content overlays and
the account skin/audio-visualizer rendering are stubbed; `views/game-placeholder.slint`
stands in for the not-yet-migrated views.

## Conventions

- `.slint` files use **kebab-case** names (`title-bar.slint`); exported
  components use **PascalCase** (`TitleBar`).
- Organize by feature folder to mirror the Vue tree: `components/` (shared),
  `views/` (screens), and — later — `overlays/` and `globals/`.
- The macOS window is Chrome-style (transparent, title-hidden, full-size content
  view), configured in `src/main.rs`; the native traffic lights are aligned to
  the custom title bar.

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
      notch; a trackpad's pixel deltas arrive in the same units and are therefore
      also doubled — that constant is the knob if it feels fast on a trackpad.
    - The toolbar and the footer have no backdrop blur: the original relies on
      `backdrop-filter: blur(4px)`, and Slint 1.18 has no backdrop blur (its only
      blur is `drop-shadow-blur`). They stay translucent, so the rows scrolling
      under the toolbar show through it unblurred.
- **Slint notes** learned the hard way while porting the list, kept here because
  both cost real debugging time:
    - `z` on a _component's root_ is ignored when the parent instantiates it
      outside a `for` loop: the compiler's z-order pass only falls back to the
      component root's `z` for repeated children, so the value has to be set on the
      instance (`InstancesList` sets the toolbar's `z: 114` there).
    - `TouchArea.pointer-event` reports a _move_ with `PointerEventButton.other`,
      never `left`, so a drag has to be tracked with its own flag rather than by
      filtering on the button.

- **macOS traffic lights**: on startup the native buttons can briefly appear at
  the stock position before settling. Several approaches were tried (moving the
  container, hiding/revealing, Electron-style positioning); see git history.
- **Icons**: the original uses Font Awesome Pro (`fa-pro`), which can't be
  shipped. The search glyph is currently a hand-embedded path; a proper icon
  strategy (e.g. the SVGs in `src/assets/icons/`) is still to be decided.
- The placeholder view contains dev-only English strings; real localized text
  arrives with the actual views.
