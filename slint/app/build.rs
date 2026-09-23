fn main() {
    // `slint-build` compiles the UI files below `ui/` into `gen` and emits Rust
    // source that `slint::include_modules!()` pulls into `main`.
    //
    // Custom fonts are embedded via `import "*.ttf"` in `ui/theme.slint`;
    // translations are bundled from `i18n/<lang>/LC_MESSAGES/<crate>.po` and
    // selected at runtime with `slint::select_bundled_translation()`.
    let config = slint_build::CompilerConfiguration::new().with_bundled_translations("i18n");
    slint_build::compile_with_config("ui/app.slint", config).expect("failed to compile the app UI");
}
