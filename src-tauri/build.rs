fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().plugin(
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
            "test_proxy_connection",
            "get_local_public_ip",
            "get_profile_logs",
            "clear_profile_logs",
            "get_proxy_stats",
            "get_usage_overview",
            "get_usage_history",
            "reset_usage",
            "set_selected_profile",
        ]),
    ))
    .expect("failed to run tauri-build");
}
