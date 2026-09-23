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
      components/                   # shared/reusable pieces
        title-bar.slint
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
        game-placeholder.slint      # stand-in for GameView
  crates/
    platform/                       # Tauri-free mirror of crates/platform
    window/                         # window-control service (min/max/fullscreen)
    config/                         # Tauri-free mirror of crates/config
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
font-family: 'Nunito', 'Comfortaa', system-ui, …;   /* body */
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
- `src/main.rs` picks a language at startup with
  `slint::select_bundled_translation(&lang)`: the language saved in the config,
  or the system locale when unset ("follow system"). An unknown code falls back
  to `en_US`.
- Changing the language in Settings → General calls
  `select_bundled_translation` again from the settings `changed` handler; Slint
  re-evaluates every `@tr` binding, so **no restart is needed**.
- `CONIC_LOCALE=fr_FR` forces a locale and disables runtime switching (useful
  for testing).
- The **default translation context is the Slint component name**, so entries
  use e.g. `msgctxt "App"` / `msgctxt "TitleBar"`.

All 12 launcher languages ship a catalog (`en_US` is the fallback):
`zh_CN`, `zh_TW`, `ja_JP`, `ko_KR`, `de_DE`, `fr_FR`, `es_ES`, `pt_BR`, `ru_RU`,
`tr_TR`, `pl_PL`. Catalogs were seeded from the Vue `src/locales/*.ts` settings
strings; the `msgid`s are the `@tr()` source strings and the `msgctxt` is the
component name. To add or refresh a language, add/update its directory under
`app/i18n/` (and the `bundled_locale()` mapping in `src/main.rs`). Catalogs can
be (re)generated with `slint-tr-extractor` (not currently installed).

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
- Icon rendering for the migrated screens (`icons.slint` + `AppIcon`).
- Config load/save (`slint-config`) wired to the `AppConfig` global: every
  setting is persisted to the same `~/.conic[-debug]/config.toml` as the Tauri
  app. Editing text fields saves on a 400 ms debounce.
- Language switching across all 12 bundled locales, applied at runtime without a
  restart (see Internationalization).
- `slint-platform` (OS detection), `slint-window` (window controls) and
  `slint-config`.

Not yet migrated: the other real views (`GameView`, `LaunchView`,
`AccountsView`, the setup wizard), the overlay layer (dialogs, content panels,
command palette, music player, instance settings), the background renderer, and
the full Java runtime scanner (settings use a lightweight subset). The settings
entry animations are also intentionally not ported this round;
`views/game-placeholder.slint` stands in for `GameView`.

## Conventions

- `.slint` files use **kebab-case** names (`title-bar.slint`); exported
  components use **PascalCase** (`TitleBar`).
- Organize by feature folder to mirror the Vue tree: `components/` (shared),
  `views/` (screens), and — later — `overlays/` and `globals/`.
- The macOS window is Chrome-style (transparent, title-hidden, full-size content
  view), configured in `src/main.rs`; the native traffic lights are aligned to
  the custom title bar.

## Known issues / notes

- **macOS traffic lights**: on startup the native buttons can briefly appear at
  the stock position before settling. Several approaches were tried (moving the
  container, hiding/revealing, Electron-style positioning); see git history.
- **Icons**: the original uses Font Awesome Pro (`fa-pro`), which can't be
  shipped. The search glyph is currently a hand-embedded path; a proper icon
  strategy (e.g. the SVGs in `src/assets/icons/`) is still to be decided.
- The placeholder view contains dev-only English strings; real localized text
  arrives with the actual views.
