# Conic Launcher
# Copyright 2022-2026 ConicMC developers. All rights reserved.
# SPDX-License-Identifier: GPL-3.0-only

# The spec `cargo rpm` builds. It is **required**, not optional: `cargo rpm init`
# generates one, and `cargo rpm` then reads *this* file back and only substitutes
# the two placeholders below. There is no fallback to an embedded template, so a
# missing spec fails the build rather than silently producing a package.
#
# It exists as a file rather than being generated because cargo-rpm's own default
# template lists only the executable in `%files`. Everything this package
# installs outside `/usr/bin` -- the desktop entry, the icon theme entries, the
# licence -- would then land in the buildroot unpackaged, and rpmbuild fails the
# build on exactly that ("Installed (but unpackaged) file(s) found"). So `%files`
# below is the part that has to be right.
#
# The file *contents* are not listed here one by one: rpmbuild's `%files`
# accepts globs, and the icon theme ladder is a set of sibling directories that a
# tenth size would add to without touching this file. The paths come from
# `[package.metadata.rpm.files]` in `app/Cargo.toml`, which is staged into
# `app/.rpm/usr/` by `tools/package-linux.sh` -- the spec and that table have to
# agree on the layout, and the table is where the layout is actually written
# down.
#
# `@@VERSION@@` and `@@RELEASE@@` are replaced with the workspace version before
# rpmbuild is invoked.

Name:           conic-launcher
Version:        @@VERSION@@
Release:        @@RELEASE@@%{?dist}
Summary:        A small, fast, and nimble Minecraft launcher

License:        GPL-3.0-only
URL:            https://github.com/conic-apps/launcher
Source0:        %{name}-%{version}.tar.gz

# The upstream licence is GPL-3.0-only *with* the section 7 additional terms in
# `LICENSE` (rename on modification, keep the copyright notices, no joint
# liability). RPM's `License:` field cannot express that: cargo-rpm reads it from
# `package.license`, which Cargo restricts to an SPDX identifier, and has no
# override. So the additional terms travel as the licence text itself, which is
# installed below and is what a redistribution has to carry.

# rpmbuild's automatic dependency generator (`find-requires`) reads the ELF's
# `NEEDED` entries, which is why nothing below has to name `libc`, `libgcc_s`,
# `libm`, `libfontconfig` or `libasound`. Those are the *only* five it finds, and
# they are not the only five the binary needs:
#
# Everything listed here is `dlopen`ed rather than linked, so no dependency
# scanner can see it, and an rpm installed without it dies at startup with
# `error while loading shared libraries: …`:
#
#   glutin / winit (x11-dl, glutin_glx_sys)   libX11, libXi, libGL
#   winit           (x11-dl)                   libXcursor, libXrender
#   x11rb           (pure Rust, libxcb)        libxcb
#   winit           (xkbcommon-dl)             libxkbcommon, libxkbcommon-x11
#   winit           (wayland-backend)          libwayland-client, libwayland-egl
#   i-slint-renderer-skia / glutin            libGL, libEGL
#
# The sonames are required rather than the distribution's package names, which is
# both what `find-requires` emits and the only spelling that survives a rename:
# `libGL.so.1` is `libglvnd` on Fedora and `libgl1` on Debian, and this package
# is built for both.
#
# X11 and Wayland are both required even though a session uses one of them:
# every current desktop has the other through Xwayland, and "which one is this
# package installed on" is not something a `Requires:` can be conditional on.
#
# One line, because rpmbuild has no line-continuation for tags: a trailing
# backslash is a parse error ("Dependency tokens must begin with alpha-numeric,
# '_' or '/'"), not a continuation.
Requires: ca-certificates xdg-utils libEGL.so.1()(64bit) libGL.so.1()(64bit) libX11.so.6()(64bit) libXi.so.6()(64bit) libXcursor.so.1()(64bit) libXrender.so.1()(64bit) libxcb.so.1()(64bit) libxkbcommon-x11.so.0()(64bit) libxkbcommon.so.0()(64bit) libwayland-client.so.0()(64bit) libwayland-egl.so.1()(64bit)

# Recommends are the two *executables* rather than libraries, and both have a
# working fallback: `reveal_in_dir` opens the parent directory when there is no
# `FileManager1` service, and the image picker (`zenity`) backs one button and
# has no fallback but is never reached otherwise.
Recommends: dbus, zenity

%description
A small, fast, and nimble Minecraft launcher.

Multi-instance management, a mod and resource pack marketplace, cross-LAN
multiplayer and multi-theme support, with 12 interface languages.

Instances, accounts (Microsoft, offline, Authlib Injector and Yggdrasil),
saves, datapacks and resourcepacks are all managed in place. A suitable Java
runtime is located on the system or downloaded on demand.

# No BuildRequires: the executable is built by `cargo rpm`'s own `cargo build`
# before rpmbuild runs, and there is nothing compiled in `%build`. `Requires` is
# the block above.

%prep
%setup -q

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a * %{buildroot}

%clean
rm -rf %{buildroot}

%files
%defattr(-,root,root,-)
%license %{_usr}/share/licenses/%{name}/LICENSE
%{_bindir}/%{name}
%{_datadir}/applications/%{name}.desktop
# The hicolor ladder `tools/generate-icons.py` writes. Matching the directory
# shape rather than naming the sizes means adding one is a change to the
# generator, not to this file. Written without RPM macros on purpose: rpmbuild
# expands macros inside comments too, and warns about the ones that look like
# tags.
%{_datadir}/icons/hicolor/*/apps/%{name}.png
%{_datadir}/icons/hicolor/scalable/apps/%{name}.svg