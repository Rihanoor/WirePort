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

#[derive(serde::Serialize)]
struct ConfigMeta {
    content: String,
    path: String,
    #[serde(rename = "generatedAt")]
    generated_at: String,
    #[serde(rename = "proxyType")]
    proxy_type: String,
    port: u16,
}

fn parse_meta_comments(content: &str) -> (String, String, u16) {
    let mut generated_at = String::new();
    let mut proxy_type = "socks5".to_string();
    let mut port = 1080;
    
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("# GeneratedAt:") {
            generated_at = line["# GeneratedAt:".len()..].trim().to_string();
        } else if line.starts_with("# ProxyType:") {
            proxy_type = line["# ProxyType:".len()..].trim().to_string();
        } else if line.starts_with("# Port:") {
            if let Ok(p) = line["# Port:".len()..].trim().parse::<u16>() {
                port = p;
            }
        }
    }
    
    (generated_at, proxy_type, port)
}

#[tauri::command]
fn generate_wireproxy_config(
    app_handle: tauri::AppHandle,
    profile_id: String,
    proxy_type: String,
    port: u16,
    config_content: String,
    generated_at: String,
) -> Result<ConfigMeta, String> {
    // Validation: Port range 1024-65535
    if port < 1024 {
        return Err("Port must be between 1024 and 65535".to_string());
    }
    if proxy_type != "socks5" && proxy_type != "http" {
        return Err(format!("Invalid proxy type: {}", proxy_type));
    }
    if parse_wg_config(&config_content).is_none() {
        return Err("Invalid WireGuard configuration".to_string());
    }

    let app_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let gen_dir = app_dir.join("generated");
    std::fs::create_dir_all(&gen_dir)
        .map_err(|e| format!("Failed to create generated directory: {}", e))?;

    let file_path = gen_dir.join(format!("{}.conf", profile_id));
    let path_str = file_path.to_string_lossy().to_string();

    // Generate config content with comment metadata at the top
    let mut generated = format!(
        "# GeneratedAt: {}\n# ProxyType: {}\n# Port: {}\n\n",
        generated_at, proxy_type, port
    );
    generated.push_str(config_content.trim());
    generated.push_str("\n\n");
    
    if proxy_type == "socks5" {
        generated.push_str("[Socks5]\n");
        generated.push_str(&format!("BindAddress = 127.0.0.1:{}\n", port));
    } else {
        generated.push_str("[HTTP]\n");
        generated.push_str(&format!("BindAddress = 127.0.0.1:{}\n", port));
    }

    std::fs::write(&file_path, &generated)
        .map_err(|e| format!("Failed to write generated config: {}", e))?;

    Ok(ConfigMeta {
        content: generated,
        path: path_str,
        generated_at,
        proxy_type,
        port,
    })
}

#[tauri::command]
fn load_generated_config(
    app_handle: tauri::AppHandle,
    profile_id: String,
) -> Result<Option<ConfigMeta>, String> {
    let app_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let file_path = app_dir.join("generated").join(format!("{}.conf", profile_id));
    
    if !file_path.exists() {
        return Ok(None);
    }
    
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read generated config: {}", e))?;
    
    let path_str = file_path.to_string_lossy().to_string();
    let (generated_at, proxy_type, port) = parse_meta_comments(&content);
    
    Ok(Some(ConfigMeta {
        content,
        path: path_str,
        generated_at,
        proxy_type,
        port,
    }))
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    wireproxy_binary_path: String,
}

#[tauri::command]
fn load_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, String> {
    let app_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    let file_path = app_dir.join("settings.json");
    if !file_path.exists() {
        return Ok(AppSettings {
            wireproxy_binary_path: String::new(),
        });
    }
    
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read settings: {}", e))?;
    
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse settings: {}", e))
}

#[tauri::command]
fn save_settings(app_handle: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let app_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| format!("Failed to create directory: {}", e))?;
    
    let file_path = app_dir.join("settings.json");
    let content = serde_json::to_string(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
        
    Ok(())
}

#[tauri::command]
fn pick_wireproxy_binary() -> Result<Option<String>, String> {
    let file = rfd::FileDialog::new()
        .set_title("Select WireProxy Binary")
        .pick_file();

    match file {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

use std::collections::HashMap;
use std::process::Child;
use std::sync::Mutex;

pub struct ProcessManager {
    processes: Mutex<HashMap<String, Child>>,
}

fn get_bundled_sidecar_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let target_triple = if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        "x86_64-unknown-linux-gnu"
    } else {
        "aarch64-apple-darwin"
    };

    let bin_name = format!("wireproxy-{}", target_triple);

    // 1. Production bundle lookup: sidecar binary should be alongside the current executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let prod_path = exe_dir.join(&bin_name);
            if prod_path.exists() && prod_path.is_file() {
                return Ok(prod_path);
            }
        }
    }

    // 2. Development lookup: sidecar is located in src-tauri/binaries/ relative to resource_dir()
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let dev_path = resource_dir.join("binaries").join(&bin_name);
        if dev_path.exists() && dev_path.is_file() {
            return Ok(dev_path);
        }
    }

    // 3. Last resort fallback to current working directory
    let fallback_path = std::path::PathBuf::from("binaries").join(&bin_name);
    if fallback_path.exists() && fallback_path.is_file() {
        return Ok(fallback_path);
    }

    Err("Could not find bundled WireProxy sidecar binary".to_string())
}

#[tauri::command]
fn start_wireproxy(
    app_handle: tauri::AppHandle,
    profile_id: String,
    config_path: String,
    binary_path: String,
    port: u16,
    state: tauri::State<'_, ProcessManager>,
) -> Result<(), String> {
    // 1. Resolve binary path
    let bin_path = if !binary_path.trim().is_empty() {
        std::path::PathBuf::from(&binary_path)
    } else {
        get_bundled_sidecar_path(&app_handle)?
    };

    // Verify binary path exists and is a file
    if !bin_path.exists() || !bin_path.is_file() {
        return Err(format!("WireProxy binary not found at: {}", bin_path.display()));
    }

    // 2. Verify generated config exists
    let conf_path = std::path::Path::new(&config_path);
    if !conf_path.exists() || !conf_path.is_file() {
        return Err(format!("Configuration file not found at: {}", config_path));
    }

    // 3. Verify port is valid
    if port < 1024 {
        return Err("Port must be between 1024 and 65535".to_string());
    }

    // 4. Verify no other process is running
    let mut map = state.processes.lock().unwrap();
    
    // Clean up dead processes first
    let mut dead_keys = Vec::new();
    for (k, child) in map.iter_mut() {
        if child.try_wait().map(|status| status.is_some()).unwrap_or(true) {
            dead_keys.push(k.clone());
        }
    }
    for k in dead_keys {
        map.remove(&k);
    }

    if !map.is_empty() {
        if map.contains_key(&profile_id) {
            return Err("This profile is already running".to_string());
        } else {
            return Err("Another profile is already running. Only one profile can run at a time in V1.".to_string());
        }
    }

    // 5. Spawn the child process
    let mut child = std::process::Command::new(&bin_path)
        .arg("-c")
        .arg(&config_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn WireProxy process: {}", e))?;

    // 6. Validation: Sleep 200ms and check status
    std::thread::sleep(std::time::Duration::from_millis(200));

    match child.try_wait() {
        Ok(None) => {
            // Still running! Success.
            map.insert(profile_id, child);
            Ok(())
        }
        Ok(Some(status)) => {
            // Exited immediately. Capture stderr.
            let mut stderr_content = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                use std::io::Read;
                let _ = stderr.read_to_string(&mut stderr_content);
            }
            let stderr_msg = stderr_content.trim();
            if stderr_msg.is_empty() {
                Err(format!("WireProxy exited immediately with status: {}", status))
            } else {
                Err(format!("WireProxy failed to start: {}", stderr_msg))
            }
        }
        Err(e) => {
            Err(format!("Failed to check WireProxy status after spawn: {}", e))
        }
    }
}

#[tauri::command]
fn stop_wireproxy(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<(), String> {
    let mut map = state.processes.lock().unwrap();
    if let Some(mut child) = map.remove(&profile_id) {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

#[tauri::command]
fn get_profile_status(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<String, String> {
    let mut map = state.processes.lock().unwrap();
    if let Some(child) = map.get_mut(&profile_id) {
        match child.try_wait() {
            Ok(None) => Ok("running".to_string()),
            Ok(Some(status)) => {
                map.remove(&profile_id);
                if status.success() {
                    Ok("stopped".to_string())
                } else {
                    Ok("error".to_string())
                }
            }
            Err(_) => {
                map.remove(&profile_id);
                Ok("error".to_string())
            }
        }
    } else {
        Ok("stopped".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ProcessManager {
            processes: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            pick_parse_and_validate_file,
            save_profiles,
            load_profiles,
            generate_wireproxy_config,
            load_generated_config,
            load_settings,
            save_settings,
            pick_wireproxy_binary,
            start_wireproxy,
            stop_wireproxy,
            get_profile_status
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.try_state::<ProcessManager>() {
                    let mut map = state.processes.lock().unwrap();
                    for (_, mut child) in map.drain() {
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
