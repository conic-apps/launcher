use std::fs;
use std::path::Path;

fn main() {
    // `slint-build` compiles the UI files below `ui/` into `gen` and emits Rust
    // source that `slint::include_modules!()` pulls into `main`.
    //
    // Custom fonts are embedded via `import "*.ttf"` in `ui/theme.slint`;
    // translations are bundled from `i18n/<lang>/LC_MESSAGES/<crate>.po` and
    // selected at runtime with `slint::select_bundled_translation()`.
    check_embedded_font();
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
    let data =
        fs::read(&path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    // TrueType outlines, the Apple `true`/`typ1` variants, and `OTTO` (CFF).
    let head = data.get(..4).unwrap_or_default();
    if !matches!(head, b"\x00\x01\x00\x00" | b"true" | b"typ1" | b"OTTO") {
        panic!(
            "{} is not an sfnt font (first bytes: {head:02x?}). Slint cannot decode WOFF2 and \
             silently falls back to the system font -- regenerate it with `python3 \
             slint/tools/merge-digit-font.py`.",
            path.display()
        );
    }
}
