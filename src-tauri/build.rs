fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .plugin(
                "wireport",
                tauri_build::InlinedPlugin::new().commands(&[
                    "pick_parse_and_validate_file",
                    "save_profiles",
                    "load_profiles",
                    "generate_wireproxy_config",
                    "load_generated_config",
                    "load_settings",
                    "save_settings",
                    "pick_wireproxy_binary",
                    "start_wireproxy",
                    "stop_wireproxy",
                    "get_profile_status",
                ]),
            ),
    )
    .expect("failed to run tauri-build");
}
