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
      fonts/Comfortaa.ttf           # converted from src/assets/fonts/*.woff2
      fonts/Nunito.ttf
      components/                   # shared/reusable pieces
        title-bar.slint
        title-bar/navigation-button.slint
        title-bar/title-bar-action-button.slint
        search-bar.slint
      views/placeholder.slint       # stand-in for the view stack (GameView, …)
  crates/
    platform/                       # Tauri-free mirror of crates/platform
    window/                         # window-control service (min/max/fullscreen)
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
  `slint::select_bundled_translation(&lang)` (from the system locale).
  `CONIC_LOCALE=zh_CN` overrides it (useful for testing).
- The **default translation context is the Slint component name**, so entries
  use e.g. `msgctxt "App"` / `msgctxt "TitleBar"`.

Currently bundled: `en_US` (fallback) and `zh_CN`. To add a language, copy a
catalog directory and register the mapping in `select_locale()`. Catalogs can be
(re)generated with `slint-tr-extractor` (not currently installed).

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
- Theme palette + typography, embedded fonts, i18n framework.
- `slint-platform` (OS detection) and `slint-window` (window controls).

Not yet migrated: the real views (`GameView`, `SettingsView`, `LaunchView`,
`AccountsView`, setup wizard), the overlay layer (dialogs, content panels,
command palette, music player, instance settings), the icon set, and the
Rust controllers/state. `views/placeholder.slint` stands in for the view stack.

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
