// Conic Launcher
// Copyright 2022-2026 ConicMC developers. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use usvg::tiny_skia_path::PathSegment;
use usvg::{Node, Tree};

/// Where the icon SVGs live, relative to this crate (`app`).
///
/// `assets/icons/` holds the icon set proper and is read whole; `assets/images/`
/// is the loose artwork (`@image-url` targets, the app icon, the default skins'
/// siblings) and only the two brand marks are taken from it, named explicitly.
/// Everything else in there is a bitmap, or a logo drawn by an `@image-url`
/// rather than by `AppIcon`, and must not reach the table.
const ICON_SOURCES: &[(&str, &str)] = &[
    ("ui/assets/icons", ""),
    ("ui/assets/images", "modrinth,curseforge"),
];

/// The stack `slint-build`'s compiler is given.
///
/// Its stack use grows with the whole `ui/` tree, and the build script's **main
/// thread** only gets the linker's default: 1 MiB on `*-pc-windows-msvc`,
/// where rustc does not raise the PE stack reserve for a build script. The
/// tree has outgrown that, and the symptom is a bare
/// `STATUS_STACK_OVERFLOW` from the build script -- the compiler dies in the
/// middle of a file and names no file, so it reads like a corrupt `.slint`.
///
/// 64 MiB is ~30x what the tree needs today (it fits in 2 MiB), which leaves the
/// next few screens off the same cliff. A thread stack is *reserved*, not
/// committed, so the headroom costs nothing until the compiler touches it.
const COMPILER_STACK_BYTES: usize = 64 * 1024 * 1024;

fn main() {
    // `slint-build` compiles the UI files below `ui/` into a generated Rust
    // module that `slint::include_modules!()` pulls into `main`.
    //
    // Custom fonts are embedded via `import "*.ttf"` in `ui/theme.slint`;
    // translations are bundled from `i18n/<lang>/LC_MESSAGES/<crate>.po` and
    // selected at runtime with `slint::select_bundled_translation()`.
    //
    // `ui/icons.slint` is generated first: it is a build product, and the
    // compiler reads it like any other `.slint` file.
    check_embedded_font();
    generate_icons();
    std::thread::Builder::new()
        .name("slint-compile".into())
        .stack_size(COMPILER_STACK_BYTES)
        .spawn(compile_ui)
        .expect("failed to spawn the UI compiler thread")
        .join()
        .expect("the UI compiler thread panicked");
}

/// Compile the UI, on the thread [`main`] set up for it.
fn compile_ui() {
    let config = slint_build::CompilerConfiguration::new().with_bundled_translations("i18n");
    slint_build::compile_with_config("ui/app.slint", config).expect("failed to compile the app UI");
}

/// Reject a font Slint cannot parse, at build time.
///
/// Slint's `register_font_from_memory` accepts an unparsable blob without
/// reporting an error, so a WOFF2 payload under a `.ttf` name drops the family
/// and every text run falls back to the system font -- with the app still
/// running and nothing logged. Failing the build is the only loud signal.
fn check_embedded_font() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/fonts/ComfortaaNunito.ttf");
    let data = fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    // TrueType outlines, the Apple `true`/`typ1` variants, and `OTTO` (CFF).
    let head = data.get(..4).unwrap_or_default();
    if !matches!(head, b"\x00\x01\x00\x00" | b"true" | b"typ1" | b"OTTO") {
        panic!(
            "{} is not an sfnt font (first bytes: {head:02x?}). Slint cannot decode WOFF2 and \
             silently falls back to the system font -- regenerate it with `python3 \
             tools/merge-digit-font.py`.",
            path.display()
        );
    }
}

/// One icon's geometry, split into the two passes `AppIcon` draws.
struct Icon {
    /// Sub-paths with a stroke, concatenated. Drawn with `stroke` over
    /// `fill: transparent`.
    stroke: String,
    /// Sub-paths with a fill, concatenated. Drawn with `fill` and
    /// `stroke: transparent`.
    fill: String,
    /// The stroke width in view box units, or `None` when nothing is stroked.
    /// One per icon because `AppIcon` scales a single value; the generator
    /// rejects an icon whose strokes disagree rather than picking one.
    stroke_width: Option<f32>,
    /// The view box edge, assumed square (see `check_square`).
    viewbox: i32,
}

/// Generate `ui/icons.slint` from the SVGs `AppIcon` renders.
///
/// **Why generated rather than written by hand.** A transcribed table drifts:
/// shapes are easy to mistype, and every SVG shape type (`<rect rx>`,
/// `<circle>`, `<ellipse>`, `<line>`, `<polyline>`) would have to be
/// re-implemented by hand. Generating from the SVGs keeps the geometry exact.
///
/// So the SVGs are read with `usvg` -- the same crate Slint rasterizes them
/// with, reached through `resvg` -- and the geometry comes back already
/// resolved: attribute inheritance applied, `<rect rx>`, `<circle>`,
/// `<ellipse>`, `<line>` and `<polyline>` all converted to path segments by
/// the library that will draw them. The generated data cannot disagree with
/// the rendering, and an icon is repaired by editing its SVG.
///
/// Adding an icon is dropping the file into `app/ui/assets/icons/`; there is
/// nothing else to update.
fn generate_icons() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut icons: BTreeMap<String, Icon> = BTreeMap::new();

    for (directory, only) in ICON_SOURCES {
        let directory = manifest.join(directory);
        // A directory watch covers files added *and* removed, which a
        // per-file `rerun-if-changed` would miss.
        println!("cargo:rerun-if-changed={}", directory.display());
        let only: Vec<&str> = only.split(',').filter(|name| !name.is_empty()).collect();

        let entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()));
        for entry in entries {
            let path = entry.expect("failed to read a directory entry").path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("svg") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if !only.is_empty() && !only.contains(&name) {
                continue;
            }
            if icons.contains_key(name) {
                // `only` lists the two brand marks, which are in `images/`
                // precisely so they cannot collide with the `icons/` set.
                continue;
            }
            let icon = parse_icon(&path);
            assert!(
                icons.insert(name.to_owned(), icon).is_none(),
                "{name}: found twice in {}",
                ICON_SOURCES
                    .iter()
                    .map(|(directory, _)| *directory)
                    .collect::<Vec<_>>()
                    .join(" and ")
            );
        }
    }

    assert!(
        !icons.is_empty(),
        "no icon SVGs found; the table would be empty"
    );
    let source = render_icons_table(&icons);
    let out = manifest.join("ui/icons.slint");
    // Write only when the table actually changed. `slint-build` registers every
    // file the compiler imports as a `rerun-if-changed` path, and that includes
    // this one -- it is read like any other `.slint` file. Rewriting it with
    // identical content still moves its mtime, the build script therefore
    // always looks stale, and every `cargo build` recompiles the whole
    // five-minute app crate instead of being a no-op.
    if fs::read_to_string(&out).is_ok_and(|existing| existing == source) {
        return;
    }
    fs::write(&out, source)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", out.display()));
}

/// Read one SVG into the two passes `AppIcon` draws.
fn parse_icon(path: &Path) -> Icon {
    let data =
        fs::read(path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let tree = Tree::from_data(&data, &usvg::Options::default()).unwrap_or_else(|error| {
        panic!("failed to parse {}: {error}", path.display());
    });
    check_square(path, &tree);

    let mut icon = Icon {
        stroke: String::new(),
        fill: String::new(),
        stroke_width: None,
        viewbox: viewbox_edge(&tree),
    };
    collect_group(tree.root(), &mut icon, path);
    icon
}

/// The view box edge, which every icon in the set happens to make square.
///
/// `AppIcon` passes one number as both `viewbox-width` and `viewbox-height`, so
/// a non-square view box would be drawn stretched. Nothing in the set is, and
/// an icon that arrives that way is a mistake worth hearing about rather than
/// a silent distortion.
fn check_square(path: &Path, tree: &Tree) {
    let size = tree.size();
    if (size.width() - size.height()).abs() > f32::EPSILON {
        panic!(
            "{}: view box is {}x{}, but `AppIcon` assumes a square one (it passes a single \
             number as both `viewbox-width` and `viewbox-height`, so the glyph would be drawn \
             stretched).",
            path.display(),
            size.width(),
            size.height()
        );
    }
}

fn viewbox_edge(tree: &Tree) -> i32 {
    // `Tree::size` is the view box in user units, which is the unit the path
    // data is in and the unit `stroke-width` is expressed against.
    (tree.size().width().round() as i32).max(1)
}

/// Walk the group's children in document order and collect every shape.
fn collect_group(group: &usvg::Group, icon: &mut Icon, path: &Path) {
    for node in group.children() {
        match node {
            Node::Group(child) => collect_group(child, icon, path),
            Node::Path(shape) => collect_shape(shape, icon, path),
            // Nothing in the icon set is an embedded raster, and `<text>` would
            // need a font to outline. Neither is a shape `AppIcon` can draw,
            // so skipping them is right -- but silently is not, because an
            // icon that is *only* one of them would generate an empty table.
            Node::Image(image) => {
                println!(
                    "cargo:warning={}: ignoring <image> ({}); an icon must be vector geometry",
                    path.display(),
                    image.id()
                );
            }
            Node::Text(text) => {
                println!(
                    "cargo:warning={}: ignoring <text id={}>; an icon must be vector geometry",
                    path.display(),
                    text.id()
                );
            }
        }
    }
}

/// Split one shape into the pass its own paint calls for.
///
/// The split is `usvg`'s, not a guess: it has already applied the CSS
/// inheritance that decides it, so a shape reads as filled or stroked exactly
/// as it will be rasterized. A shape with `fill="none"` is therefore the only
/// kind that is *not* filled.
fn collect_shape(shape: &usvg::Path, icon: &mut Icon, path: &Path) {
    if !shape.is_visible() {
        return;
    }
    // `abs_transform` is the element's own transform composed with those of
    // its ancestors; `data` is the geometry without it. `transform` returns
    // `None` for a degenerate (zero-area) transform, which leaves nothing to
    // draw either way.
    let Some(data) = shape.data().clone().transform(shape.abs_transform()) else {
        return;
    };
    let commands = serialize(&data);
    if commands.is_empty() {
        return;
    }
    if shape.fill().is_some() {
        icon.fill.push_str(&commands);
    }
    if let Some(stroke) = shape.stroke() {
        icon.stroke.push_str(&commands);
        let width = stroke.width().get();
        match icon.stroke_width {
            // A file's shapes share one `stroke-width` (SVG would let them
            // differ, and `AppIcon` cannot express that), so a second value
            // means the icon needs a different renderer, not a coin flip.
            Some(existing) if (existing - width).abs() > f32::EPSILON => {
                panic!(
                    "{}: stroked shapes disagree on stroke-width ({existing} and {width}), which \
                     `AppIcon` cannot express -- it scales a single value.",
                    path.display()
                );
            }
            _ => icon.stroke_width = Some(width),
        }
    }
}

/// `tiny-skia-path` segments back into an SVG path data string.
///
/// Slint's `Path.commands` takes SVG syntax, and `usvg` hands back the same
/// geometry as `tiny-skia-path` segments (move / line / quad / cubic / close),
/// so this is a change of spelling rather than of shape. Numbers are printed
/// with `{v}` -- Rust's shortest round-trip form -- which keeps the generated
/// file small and the numbers exactly round-trippable.
fn serialize(path: &usvg::tiny_skia_path::Path) -> String {
    let mut out = String::new();
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(point) => {
                let _ = write!(out, "M{} {}", point.x, point.y);
            }
            PathSegment::LineTo(point) => {
                let _ = write!(out, "L{} {}", point.x, point.y);
            }
            PathSegment::QuadTo(control, point) => {
                let _ = write!(out, "Q{} {} {} {}", control.x, control.y, point.x, point.y);
            }
            PathSegment::CubicTo(from, control, point) => {
                let _ = write!(
                    out,
                    "C{} {} {} {} {} {}",
                    from.x, from.y, control.x, control.y, point.x, point.y
                );
            }
            PathSegment::Close => out.push('Z'),
        }
    }
    out
}

/// Write out the `Icons` global that `AppIcon` and the direct `Icons.*()` call
/// sites (`markdown-body.slint`, `search-bar.slint`, `title-bar.slint`) read.
///
/// The four functions and their fallback values keep the call sites stable:
/// every icon in the directory is a key, so a `name:` binding and a direct
/// `Icons.*("...")` lookup both resolve.
fn render_icons_table(icons: &BTreeMap<String, Icon>) -> String {
    let mut out = String::new();
    out.push_str(
        "// Conic Launcher\n\
         // Copyright 2022-2026 ConicMC developers. All rights reserved.\n\
         // SPDX-License-Identifier: GPL-3.0-only\n\
         \n\
         // GENERATED by `app/build.rs` from the SVGs under `ui/assets/icons/` -- do not edit.\n\
         // Edit the `.svg` (or add one to `app/ui/assets/icons/`) and rebuild.\n\
         //\n\
         // The geometry below is `usvg`'s, i.e. Slint's own: attribute inheritance applied and\n\
         // every shape type (`<rect rx>`, `<circle>`, `<ellipse>`, `<line>`, `<polyline>`) already\n\
         // converted to path data.\n\
         //\n\
         // An icon is described by four lookups:\n\
         //   * `stroke-commands` -- outline sub-paths, drawn with `stroke` over a transparent fill;\n\
         //   * `fill-commands`   -- solid sub-paths, drawn with `fill` and a transparent stroke.\n\
         //                         Several icons are both, which is why the two lists are separate;\n\
         //   * `viewbox`         -- the square view box edge (16, 24, 512 or 640);\n\
         //   * `stroke-width`    -- stroke width in view box units, absent when nothing is stroked.\n\
         //\n\
         // The icon set is Font Awesome Free and Ionicons; the two brand marks (`modrinth`,\n\
         // `curseforge`) are the SVGs under `app/ui/assets/images/`, inlined here as path data so\n\
         // that `AppIcon` can tint them.\n\
         export global Icons {\n",
    );

    let mut viewbox = String::new();
    for (name, icon) in icons {
        let _ = writeln!(
            viewbox,
            "        if name == \"{name}\" {{ return {}; }}",
            icon.viewbox
        );
    }
    // A 512 view box is what an unlisted name falls back to, so an icon added
    // without a regenerated table would at least be drawn at a plausible scale
    // rather than at 1:1.
    let _ = writeln!(viewbox, "        return 512;");

    let mut stroke_width = String::new();
    for (name, icon) in icons {
        if let Some(width) = icon.stroke_width {
            let _ = writeln!(
                stroke_width,
                "        if name == \"{name}\" {{ return {}; }}",
                trim_float(width)
            );
        }
    }
    // The Ionicons half of the set is authored at `stroke-width: 32` on a 512
    // view box, so 32 is both the most common value and the right thing for an
    // icon this generator has not seen (a name reaching the table from a
    // dynamic binding) to fall back to.
    let _ = writeln!(stroke_width, "        return 32.0;");

    for (signature, body) in [
        (
            "public pure function viewbox(name: string) -> int",
            &viewbox,
        ),
        (
            "public pure function stroke-width(name: string) -> float",
            &stroke_width,
        ),
        (
            "public pure function stroke-commands(name: string) -> string",
            &render_commands(icons, |icon| &icon.stroke),
        ),
        (
            "public pure function fill-commands(name: string) -> string",
            &render_commands(icons, |icon| &icon.fill),
        ),
    ] {
        let _ = writeln!(out, "    {signature} {{\n{body}\n    }}\n");
    }
    out.push_str("}\n");
    out
}

/// The body of a `commands` lookup: one `if` per icon that has that pass, and
/// a final `return ""` so an unknown or unstroked name yields an empty path
/// rather than a parse error.
fn render_commands(icons: &BTreeMap<String, Icon>, select: impl Fn(&Icon) -> &String) -> String {
    let mut out = String::new();
    for (name, icon) in icons {
        let commands = select(icon);
        if commands.is_empty() {
            continue;
        }
        let _ = writeln!(
            out,
            "        if name == \"{name}\" {{ return \"{commands}\"; }}"
        );
    }
    out.push_str("        return \"\";\n");
    out
}

/// Print a float without a trailing `.0` where an integer will do, so the
/// generated table reads like the SVG path data it came from.
fn trim_float(value: f32) -> String {
    if (value.fract()).abs() < f32::EPSILON && value.abs() < 1.0e9 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}
