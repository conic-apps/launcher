# Windows packaging payload

Everything the `.msi` installs that is **not** the executable itself, plus the
executable's icon. Both files are **committed**, and both are read by
`tools/package-windows.ps1`.

| Consumer                        | Reads                                        |
| ------------------------------- | -------------------------------------------- |
| `wix build` — the `.wxs`        | `conic-launcher.ico`, the preprocessor below |
| the standalone `.exe` artifact  | nothing of ours — it *is* the binary         |

The `.wxs` is the MSI's recipe, and there is no generator for it: `dpkg-deb`
reads `app/Cargo.toml`, `rpmbuild` reads `app/.rpm/conic-launcher.spec`, and
`wix` reads this file. The differences between the three are in the tools, not in
this directory.

## `conic-launcher.wxs`

One file for both architectures. Everything that differs per build arrives
through the WiX preprocessor, which `package-windows.ps1` passes as `-d`:

| Variable           | From                                | Why it is not in the file |
| ------------------ | ----------------------------------- | ------------------------- |
| `ProductVersion`   | the Cargo version, minus any suffix | see "Two versions" below  |
| `ProductCode`      | derived: UUIDv5 of version + arch   | see "Identities" below     |
| `ExeFile`          | `<target-dir>/release/conic-launcher.exe` | an absolute path, because it depends on the workspace |
| `LicenseFile`      | the repository's `LICENSE`          | an absolute path, ditto    |
| `IconFile`         | `conic-launcher.ico` next to this   | ditto, and it would be one more thing to keep in step |

The WiX v4+ schema (`http://wixtoolset.org/schemas/v4/wxs`), which is WiX 5, 6
and 7. **Not** the WiX v3 dialect: that variant is EOL, and `candle`/`light`
read a different XML language rather than an older spelling of this one. The
workflow installs the v6 tool with `dotnet tool install` on both runners — see
`AGENTS.md` for why not the v3 one the x64 runner image happens to carry.

Three spellings in here that are WiX 6's and not what the older documentation
suggests, each of which is a build error rather than a warning:

- `Package/@ProductCode`, not `@PackageCode` (which does not exist).
- `<SummaryInformation Description="…" Manufacturer="…" />`, not the
  `<Title>`/`<Description>` elements WiX v3 had as children of `<Package>`.
  `Description` here is the summary stream's `Subject` row.
- `<MajorUpgrade AllowSameVersionUpgrades="yes" DowngradeErrorMessage="…">`.
  WiX 5+ *requires* `DowngradeErrorMessage` unless `AllowDowngrades="yes"`, and
  then *rejects* `AllowSameVersionUpgrades` outright. The pair above is the only
  combination that both upgrades a same-version package and still refuses a
  downgrade.

## Two versions

An MSI has one version field and it is three integers: `0.1.0-alpha.2` is
`0.1.0`. This is the same constraint `CFBundleShortVersionString` imposes on the
macOS bundle, and it is why that key is split from `CFBundleVersion` there.

The pre-release suffix is not lost, it is just not *in* the installer:

- the file name carries it — `conic-launcher-0.1.0-alpha.2-x64.msi`;
- the `ProductCode` carries it, being derived from the full Cargo version, so
  `alpha.2` and `alpha.3` are different products to Windows Installer even
  though both report `0.1.0`.

What that buys and what it costs:

- `alpha.2` → `alpha.3` → `0.1.0` all upgrade each other in place, because
  `MajorUpgrade/@AllowSameVersionUpgrades` is set. Without it Windows Installer
  refuses to treat a same-version package as an upgrade and the user ends up with
  two installs of one app in one folder.
- An *older* pre-release over a *newer* one is accepted too, since Windows
  Installer cannot order two packages that both say `0.1.0`. With no
  self-updater there is nothing that does that by accident.

## Identities

Three GUIDs are decisions and are committed rather than derived:

| GUID                                   | What it identifies                                     |
| -------------------------------------- | ------------------------------------------------------ |
| `Package/@UpgradeCode`                 | the product line — shared by both architectures         |
| `Component/@Guid` ×3 (exe, licence, shortcut) | one installed file or shortcut each              |

Changing `UpgradeCode` makes a new, unrelated app that installs beside the old
one; changing a component GUID makes Windows Installer treat the file as
somebody else's and leave it behind on upgrade. Neither is recoverable from a
release, which is why they are written down.

The `ProductCode` *is* derived, because it has to change with every version and a
table of those is a table to forget to update. The derivation is UUID version 5
(SHA-1) inside the `UpgradeCode` as its namespace, over
`"conic-launcher|<cargo version>|<arch>"`, so every identity in this app comes
from one reviewed GUID and the same commit always produces the same one.

## Naming

The architecture is spelled the way Windows spells it — **`x64` and `arm64`**,
not `x86_64`/`amd64` and not `aarch64`/`arm64`. The `.msi`'s platform is `x64` or
`arm64`, `wix build -arch` takes those, and a user reading a download page is
reading a Windows architecture. `package-linux.sh` uses `x86_64`/`aarch64`
because that is what `uname -m` prints; `package-macos.sh` uses `arm64` because
that is what `lipo` reports. Same three concepts, three vocabularies.

The installed names are fixed: `conic-launcher.exe` and `LICENSE` in
`%ProgramFiles%\Conic Launcher`, and a `Conic Launcher` folder in the Start
menu. The `.wxs` spells all three out, so a rename is a change to one file.

The `%ProgramFiles%` above is `ProgramFiles64Folder` in the `.wxs`, and it has
to be: **`ProgramFilesFolder` is the 32-bit folder**. It is the `PFiles` short
folder name, so it resolves to `C:\Program Files (x86)` whatever the package's
platform says, in an x64 package as much as an x86 one, and the install succeeds
while putting everything in the wrong place. The `Directory` table row reads
`PFiles` in both cases, so no amount of reading the package finds it.

## `conic-launcher.ico`

Regenerate with `python3 tools/generate-ico.py`. It renders the ladder from
`app/ui/assets/logo.png` — not `app/ui/assets/images/app-icon.png`, which is
512px and is embedded in the binary for the *window* icon (see
`app/src/main.rs`); upscaling that for the 256px frame is the same mistake
`generate-icns.py` documents.

**Committed, because the frames have to be BMPs.** The MSI's `Icon` table holds
`BITMAPINFOHEADER` DIBs, and Pillow writes PNG frames by default. WiX does not
check: it copies the frames in as they are and the build succeeds, so the only
symptom is an installer that cannot draw its own icon. The generator uses
`bitmap_format="bmp"` and then verifies what it wrote.

## What the `.exe` still needs

`conic-launcher.exe` is one file and nothing of this project is installed beside
it: Slint's Skia and `cpal`'s audio stack are statically linked, so the
executable is the whole application. Its PE import table is 32 entries and 31 of
them are Windows itself — `kernel32`, `user32`, `dwrite`, `opengl32`, and the
`api-ms-win-*` API sets the Universal C Runtime resolves to.

The 32nd is `VCRUNTIME140.dll`, which is **not** a Windows component: it is the
Visual C++ Redistributable. Any Windows machine with Office, .NET or another
application built with MSVC has it; a bare one does not, and the executable then
fails to start with `VCRUNTIME140.dll was not found`. This applies to the `.msi`
identically — it installs the same executable — so it is a distribution question
rather than a quirk of the portable artifact. Three ways to close it:

1. **Link the CRT statically** — `-C target-feature=+crt-static` for
   `*-pc-windows-msvc`, in `.cargo/config.toml` under the two targets so nothing
   on Linux or macOS is affected. Nothing to redistribute and no prerequisite at
   all; the cost is a slightly larger executable and no CRT updates after
   install.
2. **Chain the redistributable** — a WiX Burn bundle that runs `vc_redist.x64.exe`
   or `vc_redist.arm64.exe` before the MSI, which is Microsoft's recommended way
   and keeps the CRT centrally updatable. It is a second package to sign and
   publish, and this repository cannot sign anything yet.
3. **Ship the DLL app-local**, next to the executable. Discouraged by Microsoft
   for servicing reasons, and it would break the "one file" property of the
   portable artifact.

Which of these to take is a distribution decision rather than a packaging one, so
nothing here picks one.

## Not in here

- **No signature.** There is no certificate in this repository, so the `.msi` and
  the `.exe` ship unsigned and SmartScreen warns on both. The macOS bundle has
  the same gap in a different shape (ad-hoc `codesign`, no notarization).
- **No resource in the executable itself.** The `.exe` carries no `.rc`, so it
  has no embedded icon and no version information: Explorer shows the default
  icon and an empty Properties tab, and Apps installed through the `.msi` are
  named by the installer rather than by the binary. The window icon is set at
  runtime from the embedded image, and the Start menu shortcut gets the icon out
  of the `Icon` table, so neither is affected. Adding this means a
  `winres`-style build step in `app/build.rs`, which is an app change rather than
  a packaging one.
- **No localised installer.** `Language="1033"`, English, like the desktop entry
  and `Info.plist`. A `.wxl` is what it would take.