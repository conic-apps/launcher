# Conic Launcher — Agent Guide

## Quick start

```bash
pnpm install
pnpm dev              # Vite dev server (Tauri beforeDevCommand)
pnpm tauri dev        # full Tauri app in dev mode
pnpm build            # vue-tsc --noEmit -p tsconfig.app.json && vite build
pnpm tauri build      # production bundle
```

`pnpm clean-app-data` deletes `~/.conic-debug` (debug builds use that data dir; release builds use the app identifier).

## Verification

`pnpm check` (what contributors are asked to run) = `ts:check -> format:check -> lint -> cargo clippy -> cargo fmt`.

Frontend-only, matching CI order (`check-frontend.yml`): `pnpm ts:check`, `pnpm lint`, `pnpm format:check`, `pnpm build`.

- `pnpm ts:check` is `vue-tsc -p tsconfig.app.json` — it typechecks `crates/**/*.ts` too, not just `src/`.
- `pnpm lint` runs ESLint over **`src/**`only** (not`crates/\*/index.ts`).
- `pnpm format` rewrites with Prettier.
- `check-rust.yml`: `cargo fmt --all -- --check`, `cargo check`, then `cargo clippy --all-targets --release -- -D warnings` (clippy warnings fail CI). Its "test" job actually runs `cargo check --all --all-targets`, not `cargo test`.
- `deny.yml` runs cargo-deny (bans/licenses/sources) when `Cargo.toml`/`Cargo.lock`/`deny.toml` change.

## Package manager

pnpm only (README's `yarn` is stale). `package.json`: `packageManager: pnpm@11.18.0`, engines `node ^24.16.0`, `pnpm ^11.7.0`. `.npmrc` still pins `pnpm@10.14.0` — ignore it unless corepack throws, then align it.

## Architecture

- **`core/`** — Tauri v2 app crate. Entrypoint `core/src/main.rs`, which registers every plugin from `crates/*`. **Adding a Tauri command needs two edits**: list it in `core/build.rs` (`InlinedPlugin::new().commands(...)`) and add `"<plugin>:allow-<cmd>"` to `core/capabilities/main.json` — otherwise the invoke is denied at runtime.
- **`crates/*/`** — Rust domain crates, one per capability, each with an `index.ts` wrapping `invoke()`. Frontend imports them as `@conic/<name>`. Vite aliases `@conic` → `./crates`, but **TypeScript requires an explicit path entry per crate in `tsconfig.app.json`** — add one when adding a crate.
- **`src/`** — Vue 3 + Pinia + vue-i18n. Entry `src/main.ts` (awaits platform/data-location, then mounts `#window`). Screens switch via `src/store/navigation.ts`: `Page = "setup" | "game" | "launch" | "settings" | "accounts"`.
- **`src/views/`** — full-page screens (`GameView`, `LaunchView`, `SettingsView`, `AccountsView`, `SetupView` + `views/setup/SetupWizard*.vue`).
- **`src/overlays/`** — persistent layers above screens. `DialogRoot.vue` mounts `overlays/dialogs/*`; content panels live in `overlays/content/*`; `InstanceSetting.vue`, `CommandPalette.vue`, `MusicPlayer.vue` at the root. State: `useInstanceSettings.ts`, `content/useContent.ts`.
- **`slint/`** — **in-progress parallel migration of the UI to Slint 1.18**; read `slint/README.md` before touching it. Copy-and-adapt: the Tauri/Vue code is deliberately not modified, so both frontends coexist. Build with `cargo run -p conic-launcher-slint`. Its `.po`/`@tr()` i18n is separate from the Vue `src/locales` system — don't mix or auto-sync them.

## Crate map

| Crate          | Purpose                                               |
| -------------- | ----------------------------------------------------- |
| `account`      | Microsoft / offline / Authlib / Yggdrasil accounts    |
| `beat_this`    | Audio parsing / beat detection (`beat` plugin)        |
| `config`       | App config load/save, background image                |
| `content`      | Saves, datapacks, resourcepacks, screenshots, mods    |
| `curseforge`   | CurseForge API client                                 |
| `download`     | Generic downloader tasks                              |
| `folder`       | Data directory layout (`DATA_LOCATION`)               |
| `install`      | Minecraft + loader installation                       |
| `instance`     | Instance CRUD, playtime                               |
| `java-runtime` | Java scanning/parsing                                 |
| `launch`       | Game launch with progress reporting                   |
| `modrinth`     | Modrinth API client                                   |
| `multiplayer`  | Conic Nexus cross-LAN multiplayer                     |
| `music`        | Local music file listing                              |
| `platform`     | OS detection                                          |
| `shared`       | Common types/utilities (Rust-only, no `index.ts`)     |
| `statistics`   | Playtime statistics                                   |
| `update`       | Launcher self-update                                  |
| `version`      | Minecraft version metadata (Rust-only, no `index.ts`) |

## Rust conventions

- Workspace edition 2024, `rust-version = "1.88"`, resolver 3.
- `#![deny(clippy::unwrap_used)]` applies to `core/src/main.rs` only.
- All external deps live in root `Cargo.toml` `[workspace.dependencies]`; local crates are declared there and are not published.
- Do **not** enable Tauri's `devtools` feature in workspace deps (it force-enables devtools in release). Opt in via core's own `devtools` feature when needed.
- Release profile: `panic = "abort"`, `lto`, `codegen-units = 1`, `opt-level = "z"`, `strip`.
- File header convention: `// Conic Launcher` / copyright / `// SPDX-License-Identifier: GPL-3.0-only`.

## Frontend conventions

- Prettier is the source of truth: no semicolons in `.ts`, **semicolons in `.vue` scripts**; tabWidth 4 (2 for `.vue`/`.html`/`.yml`); `printWidth 100`.
- `vue/multi-word-component-names` is **off**; `@typescript-eslint/naming-convention` warns on `src/**` (const camelCase/UPPER_CASE, types/interface PascalCase).
- `<AppIcon>` is globally registered — no import needed.
- **Never** add `cursor: pointer`; this is a desktop app.
- `window.__PLATFORM__` and `window.__DATA_LOCATION__` are populated before the app mounts.

## i18n

12 locales in `src/locales/`; `zh_cn` is the default/source, `en_us` the fallback. Add every key to both `zh_cn.ts` and `en_us.ts` and keep all 12 in sync. The locale list must match the pickers in `src/views/settings/SettingsGeneral.vue` and `src/views/setup/SetupWizardLanguage.vue`. Never hardcode Chinese in components.

- Pluralization uses vue-i18n pipe syntax: two-form languages call `t("game.time.hoursAgo", count)` (the number binds `{count}`). `ru_ru`/`pl_pl` need four forms `zero | one | few | many` and rely on `slavicPluralRules` registered in `src/main.ts`. zh/ja/ko/tr use a single form. Playtime units (`game.time.seconds/minutes/hours`) intentionally avoid plurals because values are decimal.
- `formatLastPlayed` / `formatPlayTime` in `crates/instance/index.ts` take a formatter object built from `t()`; build it per component so locale changes re-render (see `InstanceSummary.vue`, `InstancesList.vue`, `ContentSaves.vue`, `ConfirmDeleteInstance.vue`).
- Not yet internationalized: `src/views/accounts/*`, `src/overlays/account/*`, most `src/overlays/dialogs/*` (except the create-instance flow), and `src/overlays/InstanceSetting.vue` (still uses `game.instance.*`).

## Testing

No frontend tests: Vitest is configured (jsdom) but there are no `*.spec.*`/`__tests__`. Rust unit tests exist only in a few crates (`install`, `java-runtime`); run with `cargo test`.

## Versioning & releases

The app version comes from `core/tauri.conf.json` (Windows WiX additionally has `bundle.windows.wix.version`). `build.yml` builds on push to `master` and publishes a release only when that version differs from the previous commit. README asks contributors to target the `dev` branch; releases are cut from `master`.

## Generated / ignored

`core/gen/schemas/` (Tauri build output) and `core/META-INF/` (Forge installer artifact) are gitignored. `vite.config.ts` ignores `target/**` in watch; `.prettierignore` skips `core/`, `**/assets`, and `packaging/arch/{src,pkg}`.

## Linux

`WEBKIT_DISABLE_DMABUF_RENDERER=1` is set in `core/src/main.rs` on Linux. Arch packaging lives in `packaging/arch` (`makepkg -si`).

## License

GPL-3.0-only with GPLv3 §7 additional terms: modified distributions must rename the software, keep copyright notices, and not hold the authors jointly liable.
