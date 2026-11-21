# ui

The Slint interface crate, `conic-ui`. `ui/**.slint` is the whole interface, and `ui/build.rs`
compiles it into the generated Rust module `app` re-exports as `slint_backend`, generating
`ui/graphics/icons.slint` from the SVGs under `ui/assets/icons/` on every build. `assets/` holds the
icons, artwork, skins and the two embedded fonts; `i18n/` the twelve `.po` catalogues, which
[CONTRIBUTING.md](../CONTRIBUTING.md#translations) covers.

## Layout

`app.slint` is the root `Window`. Everything else is one of five buckets, and the bucket — not the
feature — is what decides where a file goes:

- **`graphics/`** — the design system, split by role: the tokens, and the reusable,
  feature-agnostic widgets that read them. `theme.slint` (the Catppuccin palettes, the typography tokens and the
  embedded fonts) and `icons.slint` (a `build.rs` product) sit at its root. A file may join this
  bucket only if it is a widget — something a surface can put in a layout — and needs nothing but
  those two, the cross-surface infrastructure under `globals/` and its own siblings. A widget that
  reads one surface's state belongs to that surface; a shell that owns a scrim, a transition or a
  `@children` sizing protocol belongs to `overlays/shell/`.
  - `controls/` — form and display primitives, with **no prefix**: `Button`, `Checkbox`, `Input`,
    `Select`, `DropdownSelect`, `Switch`, `SliderBar`, `ListItem`, `Progress`, `Loading`.
  - `containers/` — layout/behaviour wrappers: `ScrollView`, `ScrollViewHorizontal`,
    `SlideTransition`, `ZoomTransition`.
  - `display/` — presentational: `AppIcon`, `AccountAvatar`, `InstanceCard`, `Scrim`, `EmptyState`,
    `HeaderBar`.
  - `settings/` — `SettingItem`, `SettingGroup`, `SettingCollapse`, `SettingReveal`.
  - `markdown/` — the `.slint` half of `app/src/ui/components/markdown/`.
- **`window/`** — the window's own chrome, not a widget. A file belongs here when the platform
  decides its geometry or how it is drawn (macOS's traffic-light inset, the Windows caption
  buttons), when the platform owns the input (the caption buttons have no `TouchArea` and no
  callback, because the system hit-tests and clicks them), or when Rust's native layer produces or
  consumes the type (`WindowControl`, built by `app/src/support/native/windows/caption.rs`). It
  holds `title-bar.slint` with a `title-bar/` folder of its private children
  (`title-bar-action-button`, `search-bar`), `window-background.slint` — the window's back layer,
  drawn under everything else — and `window-control.slint`, the platform-control contract the title
  bar draws from. This is the one bucket allowed to hold app-specific chrome, which is what
  distinguishes it from `graphics/`: the title bar's search, gear, news, music and notification
  buttons are fixed parts of this app's window frame, not reusable widgets.
- **`screens/`** — the destinations `Navigation` can navigate to: one `*-screen.slint` each
  (`game-screen`, `launch-screen`, `setup-screen`) with a same-named folder for its parts (`game/`,
  `setup/`). `screens.slint` is the stack that mounts them and is the only screen file the app root
  imports, so which screen is showing is decided in one place. A screen is exactly a navigation
  destination, and nothing else is: a wizard's or dialog's inner step is a step (or phase), never a
  screen.
- **`overlays/`** — everything drawn above a screen. The app-root layers (`dialog-root`,
  `dropdown-overlay`, `instance-menu`, `description-tooltip`, `command-palette`, `music-player`,
  `settings.slint`, `instance-settings`) sit here, so which layer covers which is read off
  `app.slint`'s declaration order and nowhere else. `shell/` holds the two shells a layer draws into
  — `overlay-panel` (the sliding full-window panel over its scrim) and `dialog` (the centred modal
  panel) — and nothing else belongs there. A flow gets a folder: `dialogs/*` for the dialogs
  (`dialogs/account-add/`, `dialogs/create-instance/`, `dialogs/multiplayer/`); `content/` for the
  content browser's panels and, in `content/components/`, the parts they share (the card, the card
  grid, the pager, the filter panel, the README body and the save's world map); `news/` for the news browser — a list flow over
  the Mojang news feed and the Java changelogs, with an in-panel changelog reader; and `settings/`
  for the settings drawer's section bodies (`settings-general`, `settings-jvm`, …).
- **`globals/`** — one singleton per domain (`Navigation`, `AppConfig`, `ContentState`,
  `ScreenMotion`, `SettingsShell`, `NotificationState`, `TaskState`, `OverlayShadow`, …), and how
  cross-screen state is shared without prop drilling; the ones no single surface owns (`scroll`,
  `tasks`) live in `app/src/ui/services/`.

### Naming

`*Screen` is a destination `Navigation` can navigate to, which is the only thing a screen is;
`*View` may name any other drawn thing (an overlay's contents, the Markdown renderer, a scroller).
`*Overlay`/`*Dialog`/`*Panel`/`*Card` is a surface, `*Root` is an overlay layer host. There is **no
`base-` family** — the control itself is the base. Imports are relative paths, so moving a file
means updating every importer; `cargo check` compiles the `.slint` tree and is what catches a missed
one.

### Settings placement

Settings is one drawer, `overlays/settings.slint`, over the game screen; there is no
settings screen and no screen/nav/content component split. The section list, the scroll-spy, the scrolling body
and the section imports all live in that file, while the bodies it imports live in `settings/` — the
setup wizard composes two of them (`settings-game`, `settings-jvm`) instead of duplicating them. The
drawer is declared before the other app-root panels, which is why news, music and notifications
cover it, while the settings controls' dropdowns (the `z: 1000` layer) stay above it.

The drawer is available at every window width. Its sidebar is fixed at 208px — with trimmed padding,
the narrowest width that still fits the longest translated section label — while the body scales
with the window as a clamped three fifths (520–680px), leaving at least 80px of game visible. That
keeps the body usable at the 840px minimum window without stretching the settings into a full-screen
sheet on a large display. `globals/settings-shell.slint` owns only the open flag and the screen
mirror: the title bar's gear toggles it, opening it from another screen first navigates home, and any
later navigation away from the game closes it. The title bar shows no current screen: its icons are
plain buttons, and only an open overlay fills its tile.

The drawer's two parts animate independently: each starts with its right edge on the window's left
edge and at zero opacity, and settles at its own target (the sidebar at 0, the content after it).
The game screen behind it is dimmed by the shared scrim, and the content's base colour and 24px shadow
live on a panel behind the scrolling body, so the body can stay transparent and keep its own
clipping. A fast live resize is kept out of the way by the background: the GPU framebuffer and the
sky keep their old size and stretch until the window has been still for 350ms, instead of recreating
a full-size framebuffer on every drag frame.

### Overlay dimming

Every overlay but the dialogs dims the window through one shared layer,
`graphics/display/scrim.slint`, which owns the single dim level. Each band of the window mounts
one instance — the game screen's, the app root's and the command palette's — and
`globals/overlay-shadow.slint` lets only the topmost open overlay's band ask for it: an overlay
opened over another takes the dim over, so two open overlays never darken the window twice. The
band spans the title bar, like a dialog; the others stop under it.
`overlays/shell/dialog.slint` keeps its own scrim, drawn above all of them.
