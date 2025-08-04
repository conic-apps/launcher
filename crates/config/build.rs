const COMMANDS: &[&str] = &["load_config_file"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
