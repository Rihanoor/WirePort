fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .plugin(
                "wireport",
                tauri_build::InlinedPlugin::new().commands(&[
                    "pick_parse_and_validate_file",
                    "save_profiles",
                    "load_profiles",
                ]),
            ),
    )
    .expect("failed to run tauri-build");
}
