# instance-import

Turns something dropped on the window into a new instance: a modpack archive in one of five pack
formats, or another launcher's instance folder. The frontend only has to word it —
`ImportSource::kind()` says which of the two it is.

- `source.rs` resolves where a pack's files are — a zip or an already extracted folder, with the
  pack root at the top or one folder down — and every reader goes through it.
- `pack/` holds the five formats and `launcher/` the instance layouts; each returns the common
  `Metadata` plus its own half, which `resolve` turns into downloads and file trees.
- `import.rs` creates the instance and fills it in one call: the game install, the downloads, then
  the trees. The new id is reported through `Progress::InstanceCreated` before the install starts,
  so the task card can point at it and the instance list can select it; no caller sequences "create"
  before "install". `crates/install` keeps its own meaning — it installs Minecraft, Java and the
  loader into an instance that already exists.
- `files.rs` writes the trees and reports the bytes it copies, so an instance import shows a live
  bar instead of a spinner.

| Source                    | Marker                                                        | Files                                                                                                                         | Copied                                                                                        |
| ------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| CurseForge                | `manifest.json`                                               | `projectID`/`fileID` pairs, resolved through the `curseforge` client                                                          | `overrides/`                                                                                  |
| Modrinth                  | `modrinth.index.json`                                         | the index's own URLs and hashes; a SHA-1 lookup only for an entry whose `downloads` is empty                                  | `overrides/`, then `client-overrides/`                                                        |
| Prism/MultiMC             | `mmc-pack.json`, `instance.cfg`                               | none — the pack carries everything                                                                                            | `.minecraft/`, minus `logs/` and `crash-reports/`                                             |
| MCBBS                     | `mcbbs.packmeta`, or a `manifest.json` with an `addons` array | `addon` entries the archive does not carry come from the pack's `fileApi`; `curse` entries go through the CurseForge resolver | `overrides/`                                                                                  |
| HMCL                      | `modpack.json`                                                | none                                                                                                                          | `minecraft/`, minus `pack.json` and the content the launcher installs itself                  |
| Version-isolated instance | `<folder>/<folder>.json`                                      | none                                                                                                                          | the folder, minus `versions/`, `libraries/`, `assets/`, the logs and the launcher's own files |

A Prism/MultiMC pack is an exported instance, and a live instance folder has the same layout, so
both are the same reader and both are an `Instance` import. `manifest.json` is shared, and a pack
converted from another launcher can keep a stale one: the `addons` array is what makes a manifest
MCBBS rather than CurseForge, and `mmc-pack.json` and `modrinth.index.json` are checked before it.

A source may also carry settings for the instance it becomes — mostly launch settings
(Prism/MultiMC's `instance.cfg` carries `JvmArgs`, `MaxMemAlloc`, the auto-join server and the
window size; MCBBS's `launchInfo` the JVM and game arguments), but nothing stops a source from
naming something else. `settings.rs` keeps them off the format modules, `Plan::settings` carries
them, and the import writes them into the new instance's configuration; a source that asks for
nothing leaves the instance on the launcher's defaults, and one that names a heap turns the
automatic allocation off so its choice survives. Prism/MultiMC's `patches/` and `jarmods/` patch the
game jar itself; they are skipped with a log line rather than half-applied.
