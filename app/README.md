# app

The application crate, `conic-launcher` — the binary every package ships. `conic-ui` generates the
Slint bindings and `app/src/main.rs` re-exports them as `slint_backend`, the name the rest of the
crate uses. The layer chain and the `app/src/ui/` ↔ `ui/` mirror are drawn in
[ARCHITECTURE.md](../ARCHITECTURE.md#architecture-map); this file says what each layer holds.

## Layering

- **`support/`** — process and OS plumbing with no UI counterpart. `native/` is the window-shell
  driver: the winit backend hook, the macOS traffic lights and Dock icon, the Windows caption.
- **`usecases/`** — the app's use cases: UI-neutral orchestration over the domain crates. A use case
  reports progress through a `shared::Sink<T>` port and staleness through
  `usecases::generation::{Gate, Token}`.
- **`ui/`** — the Slint adapter. The mirror is by surface/component, not per `.slint` file, and
  `ui/components/` does not repeat the `controls`/`display`/… role split. `app/src/ui/services/`
  holds the cross-surface concerns: `report` (delivery onto the event loop), `app_config`, `scroll`,
  `notifications`, `tasks`, `sessions`.

The Markdown/HTML renderer lives at `app/src/ui/components/markdown/` (its `.slint` half at
`ui/components/markdown/`): `comrak` + `parley`/`fontique` render a content panel's body and push a
display list into the Slint model, and the companion views draw it. It sits in `app/` rather than
under `crates/` because it is not independent of the UI.
