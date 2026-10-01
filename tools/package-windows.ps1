#!/usr/bin/env pwsh
# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

# Build the Windows artifacts: the single-file executable and the `.msi`.
#
# Usage:
#   tools/package-windows.ps1              # the .exe and the .msi
#   tools/package-windows.ps1 exe          # the standalone executable only
#   tools/package-windows.ps1 msi          # the installer only
#
# Everything lands in `<target-dir>/package/`.
#
# PowerShell rather than bash, unlike `package-linux.sh` and
# `package-macos.sh`. Not a preference: the MSI is assembled by `wix`, which is a
# Windows program that only runs on Windows, and a bash script driving it would
# spend its length translating paths that have no business being translated —
# `Git-Bash`'s automatic `/c/Program Files` rewriting turns a `Target=` the
# installer needs into something else. `msiexec`, `dotnet` and the WiX tool are all
# native here for the same reason.
#
# No `--target`, for the reason `package-macos.sh` gives at length: the runner *is*
# the architecture. A `--target` build compiles for the guest while every build
# script runs on the host, so Skia and `aws-lc-rs` (both built through `cc`) come
# out subtly wrong rather than failing. CI builds each architecture on a native
# runner — `windows-2025` and `windows-11-arm` — for the same reason `macos-15`
# and `macos-15-intel` are two jobs.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$Crate = 'conic-launcher'
$Payload = Join-Path $Root 'packaging/windows'
$Wxs = Join-Path $Payload 'conic-launcher.wxs'
$Icon = Join-Path $Payload 'conic-launcher.ico'
$License = Join-Path $Root 'LICENSE'

Set-Location $Root

function Die([string]$Message) {
    # stderr rather than stdout, and a plain exit rather than a `throw`: a
    # PowerShell exception prints a stack trace through every frame of the
    # script, which buries the one line that says what went wrong.
    [Console]::Error.WriteLine("error: $Message")
    exit 1
}

function Step([string]$Text) { Write-Host "==> $Text" -ForegroundColor Blue }
function Note([string]$Text) { Write-Host "    $Text" }

function Require([string]$Command, [string]$Hint) {
    # `Get-Command` rather than `command -v`, and it is checked by name so the
    # message can say what to install. Both tools are installed by the workflow
    # (`.github/workflows/build.yml`) and neither is a build dependency of the
    # crate.
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        Die "$Command is not on PATH. $Hint"
    }
}

# ---------------------------------------------------------------------------
# Options
# ---------------------------------------------------------------------------

# `-h` before anything is parsed as a format, for the reason it is before
# everything in `package-linux.sh`: a script that answers `--help` by complaining
# about an unknown format has not answered it.
if ($args -contains '-h' -or $args -contains '--help') {
    # The header comment is the usage, printed rather than duplicated. The first
    # comment line is dropped because it is the shebang, which
    # `package-macos.sh` drops the same way with `sed -n '2,20p'`.
    Get-Content $PSCommandPath |
        Where-Object { $_ -match '^#' } |
        Select-Object -Skip 1 |
        ForEach-Object { $_.Substring(1).TrimStart() }
    exit 0
}

# Same shape as `package-linux.sh`'s `deb rpm appimage`: no arguments means every
# format, and naming one restricts the run to it.
$Wanted = if ($args.Count -eq 0) { @('exe', 'msi') } else { $args }
foreach ($format in $Wanted) {
    if ($format -notin @('exe', 'msi')) {
        Die "unknown format '$format' (expected: exe, msi)"
    }
}
function Wants([string]$Format) { $Format -in $Wanted }

# ---------------------------------------------------------------------------
# Preconditions
# ---------------------------------------------------------------------------

$HostTriple = rustc -vV | Select-String -Pattern '^host: ' | ForEach-Object { $_.Line }
if (-not $HostTriple) {
    Die 'could not read rustc''s host triple; is cargo on PATH?'
}
$HostTriple = $HostTriple -replace '^host: ', ''
if ($HostTriple -notlike '*-pc-windows-msvc') {
    Die "this only runs on Windows (rustc's host is '$HostTriple', not a pc-windows-msvc target)"
}

# The architecture is read out of `rustc -vV` rather than taken as an argument,
# because the build below has no `--target`: the executable this packages is the
# one this machine just built, so its architecture is a fact rather than a
# request. The two names are *Windows'* (`x64`, `arm64`) rather than Rust's
# (`x86_64`, `aarch64`) because they name an `.msi`'s platform as well as a
# build, and `wix build -arch` wants them.
switch -regex ($HostTriple) {
    '^x86_64-pc-windows-msvc$' { $Arch = 'x64' }
    '^aarch64-pc-windows-msvc$' { $Arch = 'arm64' }
    default {
        Die "$HostTriple is not a packaged architecture (expected x86_64-pc-windows-msvc or aarch64-pc-windows-msvc)"
    }
}

if (Wants 'msi') {
    Require 'wix' "It is a .NET tool: dotnet tool install --global wix --version 6.0.2"
}

foreach ($required in @($Wxs, $Icon, $License)) {
    if (-not (Test-Path $required)) { Die "$required is missing" }
}

# ---------------------------------------------------------------------------
# What to build it from
# ---------------------------------------------------------------------------

# Both facts below come from Cargo rather than being assumed. `target/release/` is
# only cargo's default: a `.cargo/config.toml` `build.target-dir` or an exported
# `CARGO_TARGET_DIR` moves it, and a script that guessed wrong would go looking
# for an executable that is not there. One `cargo metadata` call answers both.
#
# `ConvertFrom-Json` rather than `sed` over the JSON, unlike the two shell
# scripts: this is PowerShell, and the parser is built in, so there is no second
# tool to depend on and no escaping to get wrong.
$Metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { Die 'cargo metadata failed' }

$TargetDir = $Metadata.target_directory
$Package = $Metadata.packages | Where-Object { $_.name -eq $Crate } | Select-Object -First 1
if (-not $Package) { Die "cargo metadata does not know a package called $Crate" }
$Version = $Package.version

if (-not $TargetDir) { Die 'could not read target_directory from cargo metadata' }
if (-not $Version) { Die "could not read the version of $Crate from cargo metadata" }

# `0.1.0-alpha.2` -> `0.1.0`. An MSI's `ProductVersion` is three integers and
# nothing else — there is no field for a pre-release suffix, which is the same
# constraint `CFBundleShortVersionString` imposes on the macOS bundle, and the
# reason that key is split from `CFBundleVersion` there. The suffix is not lost
# here: it is in the file name, and in the `ProductCode` below, so `0.1.0-alpha.2`
# and `0.1.0-alpha.3` are still different products to Windows Installer even
# though both are "0.1.0".
$ShortVersion = $Version -replace '-.*$', ''
$Parts = @($ShortVersion.Split('.'))
if ($Parts.Count -lt 2 -or $Parts.Count -gt 3) {
    Die "cannot derive an MSI ProductVersion from '$Version': an MSI wants two or three numeric fields"
}
while ($Parts.Count -lt 3) { $Parts += '0' }
$Bounds = @(255, 255, 65535) # major, minor, build
for ($i = 0; $i -lt 3; $i++) {
    if ($Parts[$i] -notmatch '^\d+$') {
        Die "cannot derive an MSI ProductVersion from '$Version': '$($Parts[$i])' is not a number"
    }
    if ([int]$Parts[$i] -gt $Bounds[$i]) {
        Die "cannot derive an MSI ProductVersion from '$Version': $($Parts[$i]) exceeds $($Bounds[$i])"
    }
}
$ProductVersion = $Parts -join '.'

# The MSI's own identity, derived rather than written down. `wix build` will mint a
# random one if it is left out, and a package whose identity changes on every
# build cannot be repaired, reinstalled or upgraded from a copy the user already
# has — which is the whole point of publishing one file per release.
#
# UUID version 5 (SHA-1 of a name inside a namespace), so the same commit always
# produces the same GUID: no table to keep in step with the version history, and a
# rebuild of an unreleased version is recognisably the same product rather than a
# second one. The namespace is the `UpgradeCode` in `conic-launcher.wxs`, so every
# identity this app has comes from one reviewed GUID.
#
# `Version` and `Arch` are both in the name because the two architectures share an
# `UpgradeCode`: on an ARM64 machine both run, and they are different products
# that must not claim each other's identity.
$Namespace = [guid]'FD94E4AF-6B94-40EF-ABD0-6ED38AFA2F39'
function New-DeterministicGuid([guid]$Space, [string]$Name) {
    # `[guid]::ToByteArray()` is *not* the RFC 4122 byte order — it puts the first
    # three fields little-endian, so hashing it directly would make these GUIDs
    # irreproducible by any other UUIDv5 implementation. The hex string is in the
    # right order already.
    $Hex = $Space.ToString('N')
    $Bytes = [System.Collections.Generic.List[byte]]::new()
    for ($i = 0; $i -lt 16; $i++) {
        $Bytes.Add([Convert]::ToByte($Hex.Substring($i * 2, 2), 16))
    }
    foreach ($Byte in [System.Text.Encoding]::UTF8.GetBytes($Name)) { $Bytes.Add($Byte) }

    # `SHA1.Create()` rather than the static `SHA1.HashData`, which does not exist
    # before .NET 5 and would put a runtime requirement on a script whose only
    # other dependency is a build of Rust.
    $Hash = [System.Security.Cryptography.SHA1]::Create().ComputeHash($Bytes.ToArray())
    $Out = $Hash[0..15]
    # Version 5 in the high nibble of octet 6, and the RFC 4122 variant in the
    # high bits of octet 8. Without the variant bits the result is not a v5 GUID at
    # all — it is just 16 bytes that happen to be formatted like one.
    $Out[6] = ($Out[6] -band 0x0F) -bor 0x50
    $Out[8] = ($Out[8] -band 0x3F) -bor 0x80

    $Hex = -join ($Out | ForEach-Object { $_.ToString('x2') })
    return '{0}-{1}-{2}-{3}-{4}' -f $Hex.Substring(0, 8), $Hex.Substring(8, 4),
        $Hex.Substring(12, 4), $Hex.Substring(16, 4), $Hex.Substring(20, 12)
}
$ProductCode = (New-DeterministicGuid $Namespace "$Crate|$Version|$Arch").ToUpperInvariant()

# ---------------------------------------------------------------------------
# Build the executable
# ---------------------------------------------------------------------------

# `--locked`, for the reason `package-linux.sh` gives: a release pipeline has to be
# reproducible from a commit, and a packaging run that silently updated
# `Cargo.lock` would not be.
Step "Building $Crate $Version (release, $Arch)"
cargo build --release --locked -p $Crate
if ($LASTEXITCODE -ne 0) { Die 'cargo build failed' }

$Bin = Join-Path $TargetDir "release/$Crate.exe"
if (-not (Test-Path $Bin)) { Die "$Bin was not produced" }

# `CURSEFORGE_API_KEY` is read by `crates/curseforge/build.rs` at compile time.
# Unset is not an error — the official CurseForge API is simply unauthenticated
# — but it is worth saying out loud, because the symptom otherwise shows up much
# later as an empty search.
if (-not $env:CURSEFORGE_API_KEY) {
    Note 'CURSEFORGE_API_KEY is unset; the official CurseForge API will be unauthenticated.'
}

$Out = Join-Path $TargetDir 'package'
New-Item -ItemType Directory -Force $Out | Out-Null

# The other two scripts print `packages in target/package/` by cutting the
# repository root off the front. That is a `Substring` here, and a `Substring` of
# a path that does not start with the root throws rather than printing something
# odd — which is what an exported `CARGO_TARGET_DIR` gives. So the absolute path
# is the fallback, not an exception.
function Note-Destination([string]$Path) {
    $Prefix = "$Root\"
    if ($Path.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        Note "packages in $($Path.Substring($Prefix.Length))"
    } else {
        Note "packages in $Path"
    }
}

# `$Crate-$Version-$Arch`, which is the same name the `.exe` and the `.msi` get and
# the same shape `package-macos.sh` gives its `.dmg`. The version is the Cargo one
# verbatim: the `.msi` inside says `0.1.0`, and the file it is in says
# `0.1.0-alpha.2`, so the pre-release is visible in the one place a user looks
# before running it.
$Artifact = "$Crate-$Version-$Arch"

if (Wants 'exe') {
    Step 'Copying the executable'
    Copy-Item $Bin (Join-Path $Out "$Artifact.exe") -Force
    # One file, with nothing of ours beside it: Slint's Skia and `cpal`'s audio
    # stack are statically linked, so this is the whole app. That is also why there
    # is no zip around it.
    #
    # One thing it does need is not ours either: the MSVC runtime. The executable
    # imports `VCRUNTIME140.dll`, which is the Visual C++ Redistributable rather
    # than a Windows component — every Windows machine that has Office, .NET or
    # any other Rust/VC++ app has it, and a bare one does not. See
    # `packaging/windows/README.md` for the three ways out of that and why this
    # script does not pick one.
    Get-ChildItem (Join-Path $Out "$Artifact.exe") |
        ForEach-Object { Note ("{0}  {1:N1} MiB" -f $_.Name, ($_.Length / 1MB)) }
}

if (-not (Wants 'msi')) {
    Step 'Done'
    Note-Destination $Out
    exit 0
}

# ---------------------------------------------------------------------------
# The .msi
# ---------------------------------------------------------------------------

Step 'Building .msi'

# `wix build` reads the payload through preprocessor variables rather than having
# it copied into a staging directory under a name of its choosing: the file names
# in the package are authored in the `.wxs`, and the paths here are absolute, so
# the archive says `conic-launcher.exe` and installs that, whatever this checkout
# is called and wherever it lives.
#
# `EmbedCab` is in the `.wxs`; `-arch` is not, because it is a property of the
# machine rather than of the source.
#
# `WixToolset.UI.wixext` is the extension that brings the dialogs. Without it the
# package still builds and installs — silently, with no interface at all, which for
# a double-clicked `.msi` means a window that flashes and an install that happens
# behind it.
#
# `-pdbtype none` because WiX otherwise drops a `.wixpdb` next to the output, and
# that is a debugging artifact rather than something to publish: it is half the
# size of the `.msi`, it is only meaningful to somebody stepping through the
# build, and it would sit in `target/package/` beside the two artifacts.
$WixArgs = @(
    'build', $Wxs,
    '-arch', $Arch,
    '-ext', 'WixToolset.UI.wixext',
    '-pdbtype', 'none',
    '-d', "ProductVersion=$ProductVersion",
    '-d', "ProductCode=$ProductCode",
    '-d', "ExeFile=$Bin",
    '-d', "LicenseFile=$License",
    '-d', "IconFile=$Icon",
    '-o', (Join-Path $Out "$Artifact.msi")
)

& wix @WixArgs
if ($LASTEXITCODE -ne 0) { Die "wix build failed ($LASTEXITCODE)" }

$Msi = Join-Path $Out "$Artifact.msi"
if (-not (Test-Path $Msi)) { Die "$Msi was not produced" }
Note "ProductVersion $ProductVersion (from $Version)"
Note "ProductCode    $ProductCode"
Get-ChildItem $Msi | ForEach-Object { Note ("{0}  {1:N1} MiB" -f $_.Name, ($_.Length / 1MB)) }

Step 'Done'
Note-Destination $Out