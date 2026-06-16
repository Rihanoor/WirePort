use tauri::Manager;

#[derive(serde::Serialize)]
struct ParseResult {
    name: String,
    endpoint: String,
    dns: String,
    address: String,
    allowed_ips: String,
    source_path: String,
    config_content: String,
}

struct WgConfig {
    address: String,
    dns: String,
    endpoint: String,
    allowed_ips: String,
}

fn parse_wg_config(content: &str) -> Option<WgConfig> {
    let mut current_section = "";
    let mut private_key = None;
    let mut address = None;
    let mut dns = String::new();
    let mut public_key = None;
    let mut endpoint = None;
    let mut allowed_ips = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].trim();
            continue;
        }

        if let Some(eq_idx) = line.find('=') {
            let key = line[..eq_idx].trim().to_lowercase();
            let val = line[eq_idx + 1..].trim().to_string();

            match current_section.to_lowercase().as_str() {
                "interface" => {
                    match key.as_str() {
                        "privatekey" => private_key = Some(val),
                        "address" => address = Some(val),
                        "dns" => dns = val,
                        _ => {}
                    }
                }
                "peer" => {
                    match key.as_str() {
                        "publickey" => public_key = Some(val),
                        "endpoint" => endpoint = Some(val),
                        "allowedips" => allowed_ips = Some(val),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    match (private_key, address, public_key, endpoint, allowed_ips) {
        (Some(_), Some(addr), Some(_), Some(end), Some(aips)) => Some(WgConfig {
            address: addr,
            dns,
            endpoint: end,
            allowed_ips: aips,
        }),
        _ => None,
    }
}

#[tauri::command]
fn pick_parse_and_validate_file() -> Result<Option<ParseResult>, String> {
    let file = rfd::FileDialog::new()
        .add_filter("WireGuard Config", &["conf"])
        .pick_file();

    match file {
        Some(path) => {
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.replace(".conf", ""))
                .unwrap_or_else(|| "unnamed".to_string());
            
            let source_path = path.to_string_lossy().to_string();
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read file: {}", e))?;

            if let Some(parsed) = parse_wg_config(&content) {
                Ok(Some(ParseResult {
                    name,
                    endpoint: parsed.endpoint,
                    dns: parsed.dns,
                    address: parsed.address,
                    allowed_ips: parsed.allowed_ips,
                    source_path,
                    config_content: content,
                }))
            } else {
                Err("Invalid WireGuard configuration".to_string())
            }
        }
        None => Ok(None),
    }
}

#[tauri::command]
fn save_profiles(app_handle: tauri::AppHandle, profiles_json: String) -> Result<(), String> {
    let app_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    
    let file_path = app_dir.join("profiles.json");
    std::fs::write(&file_path, profiles_json)
        .map_err(|e| format!("Failed to write profiles: {}", e))?;
        
    Ok(())
}

#[tauri::command]
fn load_profiles(app_handle: tauri::AppHandle) -> Result<String, String> {
    let app_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let file_path = app_dir.join("profiles.json");
    if !file_path.exists() {
        return Ok("[]".to_string());
    }
    
    std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read profiles: {}", e))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            pick_parse_and_validate_file,
            save_profiles,
            load_profiles
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
