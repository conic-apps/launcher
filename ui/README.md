# ui

The Slint interface crate, `conic-ui`. `ui/**.slint` is the whole interface, and `ui/build.rs`
compiles it into the generated Rust module `app` re-exports as `slint_backend`, generating
`ui/icons.slint` from the SVGs under `ui/assets/icons/` on every build. `assets/` holds the icons,
artwork, skins and the two embedded fonts; `i18n/` the twelve `.po` catalogues, which
[CONTRIBUTING.md](../CONTRIBUTING.md#translations) covers.

## Layout

`app.slint` is the root `Window`, `theme.slint` the design tokens and `icons.slint` a `build.rs`
product. Everything else is one of four buckets, and the bucket — not the feature — is what decides
where a file goes:

- **`components/`** — reusable, feature-agnostic widgets, split by role:
  - `controls/` — form and display primitives, with **no prefix**: `Button`, `Checkbox`, `Input`,
    `Select`, `DropdownSelect`, `Switch`, `SliderBar`, `ListItem`, `Progress`, `Loading`.
  - `containers/` — layout/behaviour wrappers: `ScrollView`, `ScrollViewHorizontal`,
    `SlideTransition`, `ZoomTransition`.
  - `display/` — presentational: `AppIcon`, `AccountAvatar`, `InstanceCard`, `PaletteRow`, `Scrim`,
    `WindowBackground`.
  - `settings/` — `SettingItem`, `SettingGroup`, `SettingCollapse`, `SettingReveal`.
  - `markdown/` — the `.slint` half of `app/src/ui/components/markdown/`.
  - `title-bar.slint` plus a `title-bar/` folder of its private children (`title-bar-action-button`,
    `search-bar`). A component gets a same-named folder only when it has private children, and keeps
    them there rather than in `components/`.
- **`views/`** — the top-level pages: one `*-view.slint` each (`game-view`, `launch-view`,
  `setup-view`, `todo-placeholder`) with a same-named folder for its parts (`game/`, `setup/`).
- **`overlays/`** — everything drawn above a page. The app-root layers (`dialog-root`,
  `dropdown-overlay`, `instance-menu`, `description-tooltip`, `command-palette`, `music-player`,
  `settings-overlay`, `instance-settings`) sit here; a dialog _flow_ gets a folder
  (`dialogs/account-add/`, `dialogs/create-instance/`, `dialogs/multiplayer/`) with
  `dialogs/dialog.slint` as the shared modal shell, the content browser is `content/`, the news
  browser — a list flow over the Mojang news feed and the Java changelogs, with an in-panel
  changelog reader — is `news/`, and the settings drawer's section bodies (`settings-general`,
  `settings-jvm`, …) are `settings/`.
- **`globals/`** — one singleton per domain (`Navigation`, `AppConfig`, `ContentState`,
  `PageMotion`, `SettingsShell`, `NotificationState`, `TaskState`, `OverlayShadow`, …), and how
  cross-screen state is shared without prop drilling; the ones no single surface owns (`scroll`,
  `tasks`) live in `app/src/ui/services/`.

### Settings placement

Settings is one drawer, `overlays/settings-overlay.slint`, over the game view; there is no settings
page and no page/nav/content component split. The section list, the scroll-spy, the scrolling body
and the section imports all live in that file, while the bodies it imports live in `settings/` — the
setup wizard composes two of them (`settings-game`, `settings-jvm`) instead of duplicating them. The
drawer is declared before the other app-root panels, which is why news, music and notifications
cover it, while the settings controls' dropdowns (the `z: 1000` layer) stay above it.

The drawer is available at every window width. Its sidebar is fixed at 208px — with trimmed padding,
the narrowest width that still fits the longest translated section label — while the body scales
with the window as a clamped three fifths (520–680px), leaving at least 80px of game visible. That
keeps the body usable at the 840px minimum window without stretching the settings into a full-screen
sheet on a large display. `globals/settings-shell.slint` owns only the open flag and the page
mirror: the title bar's gear toggles it, opening it from another page first navigates home, and any
later navigation away from the game closes it. The title bar shows no current page: its icons are
plain buttons, and only an open overlay fills its tile.

The drawer's two parts animate independently: each starts with its right edge on the window's left
edge and at zero opacity, and settles at its own target (the sidebar at 0, the content after it).
The game view behind it is dimmed by the shared scrim, and the content's base colour and 24px shadow
live on a panel behind the scrolling body, so the body can stay transparent and keep its own
clipping. A fast live resize is kept out of the way by the background: the GPU framebuffer and the
sky keep their old size and stretch until the window has been still for 350ms, instead of recreating
a full-size framebuffer on every drag frame.

### Overlay dimming

Every overlay but the dialogs dims the window through one shared layer,
`components/display/scrim.slint`, which owns the single dim level. Each band of the window mounts
one instance — the game view's, the app root's and the palette's — and
`globals/overlay-shadow.slint` lets only the topmost open overlay's band ask for it: an overlay
opened over another takes the dim over, so two open overlays never darken the window twice. The
palette's band spans the title bar, like a dialog; the others stop under it.
`overlays/dialogs/dialog.slint` keeps its own scrim, drawn above all of them.
