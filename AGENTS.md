# Conic Launcher — Agent Guide

## Quick start

```bash
pnpm install          # install frontend deps
pnpm dev              # Vite dev server (Tauri's beforeDevCommand)
pnpm tauri dev        # full Tauri app in dev mode
pnpm build            # vue-tsc --noEmit && vite build (runs as Tauri beforeBuildCommand)
pnpm tauri build      # production build
```

## Verification commands (run in this order)

```bash
pnpm lint             # eslint "src/**" --ignore-pattern "src/assets" --max-warnings=0
pnpm format:check     # prettier --check .
pnpm ts:check         # tsc --noEmit
```

`pnpm format` rewrites files with Prettier.

## Package manager

Use **pnpm** only. README says `yarn` — it's stale. Engine is `pnpm@11.7.0`; `.npmrc` has `pnpm@10.14.0` (also stale — ignore). If you get engine errors, update `.npmrc` to match `package.json`.

## Architecture

- **`core/`** — Tauri v2 app crate. Entrypoint: `core/src/main.rs`. Registers all plugins (config, account, install, instance, launch, folder, platform) and their invoke handlers in `core/build.rs`.
- **`crates/*/`** — Rust library crates. Each has a corresponding `crates/<name>/index.ts` that wraps `@tauri-apps/api/core` `invoke()` calls. The TypeScript frontend imports these as `@conic/<name>` (Vite alias resolves `@conic` → `./crates`).
- **`src/`** — Vue 3 + Pinia + vue-i18n frontend. Entrypoint: `src/main.ts`. Uses `@tauri-apps/api` for window and invoke.
- **`src/views/`** — Screens (osu!-style): full-page views switched by `navigationStore` (`GameView`, `LaunchView`, `SettingsView`, `AccountsView`, `SetupWizard`). Screen-local subcomponents live in their subdirectories (`views/game/*`, `views/settings/*`, `views/setup/*`).
- **`src/overlays/`** — Global overlays (osu!-style): persistent layers mounted above all screens. `DialogRoot.vue` mounts everything under `overlays/dialogs/*`; content panels live in `overlays/content/*`; `InstanceSetting.vue`, `CommandPalette.vue`, `MusicPlayer.vue` sit at the overlays root. Visibility state for instance settings: `overlays/useInstanceSettings.ts`; for content panels: `overlays/content/useContent.ts`.
- **`core/capabilities/main.json`** — Tauri v2 capability/permission file. Adding a new Tauri command requires adding a permission entry here and in `core/build.rs`.

## Crate map

| Crate       | Purpose                                                              |
| ----------- | -------------------------------------------------------------------- |
| `account`   | Microsoft/offline/Authlib/Yggdrasil account management               |
| `config`    | App config load/save                                                 |
| `download`  | Generic file downloader                                              |
| `folder`    | Data directory layout                                                |
| `game_data` | Minecraft resource pack / world parsing                              |
| `install`   | Minecraft + mod loader installation (Forge, Fabric, Quilt, NeoForge) |
| `instance`  | Instance CRUD                                                        |
| `launch`    | Game launch with progress reporting                                  |
| `modrinth`  | Modrinth API client                                                  |
| `platform`  | OS detection                                                         |
| `shared`    | Common types/utilities                                               |
| `version`   | Minecraft version metadata                                           |

## Rust conventions

- Edition 2024, MSRV 1.88
- `#![deny(clippy::unwrap_used)]` in the main crate
- Release profile: `panic = "abort"`, `lto = true`, `codegen-units = 1`, `strip = true`

## Frontend conventions

- **No semicolons** in `.ts` files; **semicolons required** in `.vue` `<script>` blocks
- Indent: 4 spaces (TS/JS), 2 spaces (Vue templates, HTML, YAML)
- `vue/multi-word-component-names` ESLint rule is **off**
- Naming: `camelCase` for variables/consts, `UPPER_CASE` for constants, `PascalCase` for types/interfaces
- Components use `<AppIcon>` globally registered — no import needed
- **Never** add `cursor: pointer` on your own — this is a desktop app; interactive elements don't use the web link-hand cursor

## Internationalization (i18n)

The app uses `vue-i18n` for internationalization. Locale files are in `src/locales/`.

### Locale file structure

12 locales, all kept in sync (406+ keys each). `zh_cn` is the primary/source language; `en_us` is the fallback.

| File       | Language           | Notes                                     |
| ---------- | ------------------ | ----------------------------------------- |
| `zh_cn.ts` | 简体中文           | Source language                           |
| `en_us.ts` | English            | Fallback locale                           |
| `zh_tw.ts` | 繁體中文           |                                           |
| `ja_jp.ts` | 日本語             |                                           |
| `ko_kr.ts` | 한국어             |                                           |
| `de_de.ts` | Deutsch            |                                           |
| `fr_fr.ts` | Français           |                                           |
| `es_es.ts` | Español            |                                           |
| `pt_br.ts` | Português (Brasil) |                                           |
| `ru_ru.ts` | Русский            | Slavic plural rules registered in main.ts |
| `tr_tr.ts` | Türkçe             |                                           |
| `pl_pl.ts` | Polski             | Slavic plural rules registered in main.ts |

The supported locale list must match the UI language pickers in `src/views/settings/SettingsGeneral.vue` and `src/views/setup/SetupWizardLanguage.vue`.

Top-level keys: `app`, `game`, `overlays`, `setup`, `settings`. Locale namespaces mirror the component structure.

- `app.*` — App-wide strings (titlebar search, update indicator)
- `game.*` — Game view, instance list, launch view, footer, time formatting (`game.time.*`)
- `overlays.dialogs.*` — Dialog overlays (e.g. `overlays.dialogs.createInstance.*`, `overlays.dialogs.minecraftChoose.*`)
- `overlays.content.*` — Content panels (mods, resource packs, modpacks, saves, screenshots)
- `setup.*` — Setup wizard (all steps)
- `settings.*` — Settings view (all tabs)

### Pluralization

vue-i18n pipe syntax is used where grammar requires it:

- Two-form languages: `"1 hour ago | {count} hours ago"` — call with `t("game.time.hoursAgo", hours)` (number as second arg; `{count}` is bound automatically)
- Russian/Polish need FOUR forms `zero | one | few | many` (e.g. `"час назад"`) and rely on the `slavicPluralRules` function registered under `pluralRules` in `src/main.ts` for both `ru_ru` and `pl_pl`
- Languages without numeral agreement (zh, ja, ko, tr) use a single form
- Playtime units (`game.time.seconds/minutes/hours`) deliberately avoid plurals because values can be decimal (e.g. "4.1 min")

### Relative-time formatting

`formatLastPlayed(timestamp, timeFormatter)` and `formatPlayTime(seconds, playTimeFormatter)` in `crates/instance/index.ts` require formatter objects built from `t()`. Build them per-component like in `InstanceSummary.vue` / `InstancesList.vue` / `ContentSaves.vue` / `ConfirmDeleteInstance.vue` (plain object whose methods call `t()` so re-renders pick up locale changes).

### Current i18n coverage

**Fully internationalized:**

- `src/views/game/*` (InstancesList, InstanceSummary, Footer, InstancesListToolBar)
- `src/views/LaunchView.vue`
- `src/overlays/content/*` (all content panel files)
- `src/overlays/dialogs/CreateInstance.vue` and its subcomponents (`create/MinecraftChoose.vue`)
- `src/views/SetupWizard.vue` and `src/views/setup/*`
- `src/views/SettingsView.vue` and `src/views/settings/*`
- `src/overlays/MusicPlayer.vue`
- `src/overlays/CommandPalette.vue`
- `src/components/TitleBarUpdateIndicator.vue`
- `src/components/BaseSearchBar.vue`
- `src/App.vue` (titlebar search placeholder)

**Not yet internationalized (accounts — pending new UI):**

- `src/views/accounts/*`
- Remaining dialogs in `src/overlays/dialogs/*` other than the create-instance flow
- `src/overlays/InstanceSetting.vue` (uses `game.instance.*` keys; namespace may move under `overlays.*` later)

### How to add/update i18n strings

1. Add the key to both `src/locales/zh_cn.ts` and `src/locales/en_us.ts`
2. Use `t('key.path')` in templates (requires `const { t } = useI18n()` in `<script setup>`)
3. For parameterized messages, use `t('key', { param: value })` — e.g. `t('game.launch.progress.downloadFiles', { current, total })`
4. **Do NOT** use Chinese strings directly in templates/components for internationalized sections
5. After UI text changes, both locale files must be kept in sync

### Adding a new locale

1. Create `src/locales/<locale>.ts` following the same structure as `zh_cn.ts`
2. Import it in `src/main.ts` and add to the `messages` object
3. Update the locale list in `src/views/settings/SettingsGeneral.vue` and `src/views/setup/SetupWizardLanguage.vue`

## Testing

**No tests exist** — neither Rust tests (`mod tests {}`) nor frontend tests (no `__tests__/` or `*.spec.*` files). Vitest is configured in `vitest.config.ts` (jsdom environment) but has no tests to run.

## Linux-specific

`WEBKIT_DISABLE_DMABUF_RENDERER=1` is set at startup to work around WebKit rendering issues on Linux.

## Generated files

`core/gen/schemas/` is generated by Tauri build and gitignored.
`core/.gitignore` also ignores `META-INF/` (Forge installer artifact).

## License

GPL-3.0-only with additional terms (see LICENSE — distribution must rename the software, keep copyright, no joint liability).
