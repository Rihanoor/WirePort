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

use std::collections::{HashMap, VecDeque};
use std::process::Child;
use std::sync::Mutex;
use std::io::BufRead;

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

struct LocalIpCache {
    ip: String,
    fetched_at: std::time::Instant,
}

pub struct ProcessManager {
    processes: Mutex<HashMap<String, Child>>,
    local_ip_cache: Mutex<Option<LocalIpCache>>,
    logs: Mutex<HashMap<String, VecDeque<LogEntry>>>,
}

fn classify_stderr_line(line: &str) -> &'static str {
    let lower = line.to_lowercase();
    if lower.contains("error")
        || lower.contains("fatal")
        || lower.contains("panic")
        || lower.contains("failed")
        || lower.contains("unable")
        || lower.contains("cannot")
    {
        "ERROR"
    } else if lower.contains("warn") || lower.contains("warning") {
        "WARN"
    } else {
        "DEBUG"
    }
}

fn strip_wireproxy_prefix(line: &str) -> &str {
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() >= 4 {
        let lvl = parts[0].to_uppercase();
        let date = parts[1];
        let time = parts[2];
        
        let is_valid_level = lvl == "DEBUG:" || lvl == "INFO:" || lvl == "WARN:" || lvl == "ERROR:";
        let is_valid_date = date.len() == 10 && date.chars().nth(4) == Some('/') && date.chars().nth(7) == Some('/');
        let is_valid_time = time.len() == 8 && time.chars().nth(2) == Some(':') && time.chars().nth(5) == Some(':');
        
        if is_valid_level && is_valid_date && is_valid_time {
            return parts[3];
        }
    }
    line
}

fn append_log(state: &ProcessManager, profile_id: &str, level: &str, message: &str) {
    let mut logs = state.logs.lock().unwrap();
    let entries = logs.entry(profile_id.to_string()).or_insert_with(VecDeque::new);
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let clean_message = strip_wireproxy_prefix(message);
    entries.push_back(LogEntry {
        timestamp,
        level: level.to_string(),
        message: clean_message.to_string(),
    });
    if entries.len() > 1000 {
        entries.pop_front();
    }
}

fn handle_unexpected_exit(app_handle: &tauri::AppHandle, profile_id: &str) {
    let state = app_handle.state::<ProcessManager>();
    let child_opt = {
        let mut map = state.processes.lock().unwrap();
        map.remove(profile_id)
    };
    
    if let Some(mut child) = child_opt {
        let status = child.wait();
        append_log(&state, profile_id, "INFO", "WireProxy process stopped");
        append_log(&state, profile_id, "ERROR", "Process exited unexpectedly");
        let exit_code_str = match status {
            Ok(s) => match s.code() {
                Some(code) => code.to_string(),
                None => "unknown".to_string(),
            },
            Err(_) => "unknown".to_string(),
        };
        append_log(&state, profile_id, "ERROR", &format!("WireProxy exited unexpectedly (exit code: {})", exit_code_str));
    }
}

#[tauri::command]
fn get_profile_logs(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<Vec<LogEntry>, String> {
    let logs = state.logs.lock().unwrap();
    if let Some(entries) = logs.get(&profile_id) {
        Ok(entries.iter().cloned().collect())
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn clear_profile_logs(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<(), String> {
    let mut logs = state.logs.lock().unwrap();
    if let Some(entries) = logs.get_mut(&profile_id) {
        entries.clear();
    }
    Ok(())
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
    append_log(&state, &profile_id, "INFO", "Starting WireProxy");

    // 1. Resolve binary path
    let bin_path = if !binary_path.trim().is_empty() {
        std::path::PathBuf::from(&binary_path)
    } else {
        match get_bundled_sidecar_path(&app_handle) {
            Ok(p) => p,
            Err(e) => {
                append_log(&state, &profile_id, "ERROR", &e);
                append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
                append_log(&state, &profile_id, "ERROR", "WireProxy exited unexpectedly (exit code: unknown)");
                return Err(e);
            }
        }
    };

    // Verify binary path exists and is a file
    if !bin_path.exists() || !bin_path.is_file() {
        let err_msg = format!("WireProxy binary not found at: {}", bin_path.display());
        append_log(&state, &profile_id, "ERROR", &err_msg);
        append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
        append_log(&state, &profile_id, "ERROR", "WireProxy exited unexpectedly (exit code: unknown)");
        return Err(err_msg);
    }

    // 2. Verify generated config exists
    let conf_path = std::path::Path::new(&config_path);
    if !conf_path.exists() || !conf_path.is_file() {
        let err_msg = format!("Configuration file not found at: {}", config_path);
        append_log(&state, &profile_id, "ERROR", &err_msg);
        append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
        append_log(&state, &profile_id, "ERROR", "WireProxy exited unexpectedly (exit code: unknown)");
        return Err(err_msg);
    }

    // 3. Verify port is valid
    if port < 1024 {
        let err_msg = "Port must be between 1024 and 65535".to_string();
        append_log(&state, &profile_id, "ERROR", &err_msg);
        append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
        append_log(&state, &profile_id, "ERROR", "WireProxy exited unexpectedly (exit code: unknown)");
        return Err(err_msg);
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
        let err_msg = if map.contains_key(&profile_id) {
            "This profile is already running".to_string()
        } else {
            "Another profile is already running. Only one profile can run at a time in V1.".to_string()
        };
        append_log(&state, &profile_id, "ERROR", &err_msg);
        append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
        append_log(&state, &profile_id, "ERROR", "WireProxy exited unexpectedly (exit code: unknown)");
        return Err(err_msg);
    }

    // 5. Spawn the child process
    let mut child = match std::process::Command::new(&bin_path)
        .arg("-c")
        .arg(&config_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn() {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Failed to spawn WireProxy process: {}", e);
                append_log(&state, &profile_id, "ERROR", &err_msg);
                append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
                append_log(&state, &profile_id, "ERROR", "WireProxy exited unexpectedly (exit code: unknown)");
                return Err(err_msg);
            }
        };

    // 6. Validation: Sleep 200ms and check status
    std::thread::sleep(std::time::Duration::from_millis(200));

    match child.try_wait() {
        Ok(None) => {
            // Still running! Success.
            append_log(&state, &profile_id, "INFO", "WireProxy process started");

            let mut child_to_insert = child;
            let stdout = child_to_insert.stdout.take();
            let stderr = child_to_insert.stderr.take();

            map.insert(profile_id.clone(), child_to_insert);
            drop(map);

            if let Some(stdout_pipe) = stdout {
                let app_handle_clone = app_handle.clone();
                let profile_id_clone = profile_id.clone();
                std::thread::spawn(move || {
                    let reader = std::io::BufReader::new(stdout_pipe);
                    for line in reader.lines() {
                        if let Ok(msg) = line {
                            let state = app_handle_clone.state::<ProcessManager>();
                            append_log(&state, &profile_id_clone, "INFO", &msg);
                        }
                    }
                    handle_unexpected_exit(&app_handle_clone, &profile_id_clone);
                });
            }

            if let Some(stderr_pipe) = stderr {
                let app_handle_clone = app_handle.clone();
                let profile_id_clone = profile_id.clone();
                std::thread::spawn(move || {
                    let reader = std::io::BufReader::new(stderr_pipe);
                    for line in reader.lines() {
                        if let Ok(msg) = line {
                            let state = app_handle_clone.state::<ProcessManager>();
                            let level = classify_stderr_line(&msg);
                            append_log(&state, &profile_id_clone, level, &msg);
                        }
                    }
                    handle_unexpected_exit(&app_handle_clone, &profile_id_clone);
                });
            }

            Ok(())
        }
        Ok(Some(status)) => {
            // Exited immediately. Capture stdout and stderr.
            let mut stdout_content = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                use std::io::Read;
                let _ = stdout.read_to_string(&mut stdout_content);
            }
            let mut stderr_content = String::new();
            if let Some(mut stderr) = child.stderr.take() {
                use std::io::Read;
                let _ = stderr.read_to_string(&mut stderr_content);
            }

            // Log stdout and stderr
            for line in stdout_content.lines() {
                append_log(&state, &profile_id, "INFO", line);
            }
            for line in stderr_content.lines() {
                let level = classify_stderr_line(line);
                append_log(&state, &profile_id, level, line);
            }

            append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
            let exit_code_str = match status.code() {
                Some(code) => code.to_string(),
                None => "unknown".to_string(),
            };
            append_log(&state, &profile_id, "ERROR", &format!("WireProxy exited unexpectedly (exit code: {})", exit_code_str));

            let stderr_msg = stderr_content.trim();
            if stderr_msg.is_empty() {
                Err(format!("WireProxy exited immediately with status: {}", status))
            } else {
                Err(format!("WireProxy failed to start: {}", stderr_msg))
            }
        }
        Err(e) => {
            let err_msg = format!("Failed to check WireProxy status after spawn: {}", e);
            append_log(&state, &profile_id, "ERROR", &err_msg);
            append_log(&state, &profile_id, "ERROR", "Process exited unexpectedly");
            append_log(&state, &profile_id, "ERROR", "WireProxy exited unexpectedly (exit code: unknown)");
            Err(err_msg)
        }
    }
}

#[tauri::command]
fn stop_wireproxy(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<(), String> {
    let child_opt = {
        let mut map = state.processes.lock().unwrap();
        map.remove(&profile_id)
    };

    if let Some(mut child) = child_opt {
        let _ = child.kill();
        let _ = child.wait();
        append_log(&state, &profile_id, "INFO", "WireProxy process stopped");
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
                if status.success() {
                    Ok("stopped".to_string())
                } else {
                    Ok("error".to_string())
                }
            }
            Err(_) => Ok("error".to_string()),
        }
    } else {
        Ok("stopped".to_string())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionHealthResult {
    success: bool,
    tunnel_active: bool,
    exit_ip: String,
    local_ip: String,
    latency_ms: u64,
    error: String,
}

async fn get_local_public_ip_internal(
    state: &ProcessManager,
) -> Result<String, String> {
    // Check if cached (5 min = 300 seconds)
    {
        let cache = state.local_ip_cache.lock().unwrap();
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < std::time::Duration::from_secs(300) {
                return Ok(cached.ip.clone());
            }
        }
    }

    // Fetch new
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build client for local IP lookup: {}", e))?;

    let providers = [
        "https://api.ipify.org",
        "https://api64.ipify.org",
        "https://ifconfig.me/ip",
    ];

    let mut last_error = String::new();
    for provider in &providers {
        match client.get(*provider).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    if let Ok(body) = res.text().await {
                        let ip = body.trim().to_string();
                        if !ip.is_empty() {
                            let mut cache = state.local_ip_cache.lock().unwrap();
                            *cache = Some(LocalIpCache {
                                ip: ip.clone(),
                                fetched_at: std::time::Instant::now(),
                            });
                            return Ok(ip);
                        }
                    }
                }
            }
            Err(e) => {
                last_error = format!("Request to {} failed: {}", provider, e);
            }
        }
    }

    Err(format!(
        "Failed to fetch local public IP. Last error: {}",
        last_error
    ))
}

#[tauri::command]
async fn get_local_public_ip(
    state: tauri::State<'_, ProcessManager>,
) -> Result<String, String> {
    get_local_public_ip_internal(&state).await
}

#[tauri::command]
async fn test_proxy_connection(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, ProcessManager>,
    profile_id: String,
) -> Result<ConnectionHealthResult, String> {
    // 1. Fetch local public IP first
    let local_ip = match get_local_public_ip_internal(&state).await {
        Ok(ip) => ip,
        Err(e) => return Ok(ConnectionHealthResult {
            success: false,
            tunnel_active: false,
            exit_ip: String::new(),
            local_ip: String::new(),
            latency_ms: 0,
            error: format!("Failed to fetch local public IP: {}", e),
        }),
    };

    // 2. Get proxy settings
    let app_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let file_path = app_dir.join("generated").join(format!("{}.conf", profile_id));
    if !file_path.exists() {
        return Ok(ConnectionHealthResult {
            success: false,
            tunnel_active: false,
            exit_ip: String::new(),
            local_ip,
            latency_ms: 0,
            error: "Configuration file not found. Please regenerate configuration.".to_string(),
        });
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read generated config: {}", e))?;

    let (_, proxy_type, port) = parse_meta_comments(&content);

    // 3. Build reqwest proxy client
    let proxy_url = if proxy_type.to_lowercase() == "http" {
        format!("http://127.0.0.1:{}", port)
    } else {
        format!("socks5://127.0.0.1:{}", port)
    };

    let proxy = match reqwest::Proxy::all(&proxy_url) {
        Ok(p) => p,
        Err(e) => return Ok(ConnectionHealthResult {
            success: false,
            tunnel_active: false,
            exit_ip: String::new(),
            local_ip,
            latency_ms: 0,
            error: format!("Failed to configure proxy URL: {}", e),
        }),
    };

    let client = match reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(5))
        .build() {
            Ok(c) => c,
            Err(e) => return Ok(ConnectionHealthResult {
                success: false,
                tunnel_active: false,
                exit_ip: String::new(),
                local_ip,
                latency_ms: 0,
                error: format!("Failed to build HTTP client: {}", e),
            }),
        };

    // 4. Make HTTP request with fallbacks
    let providers = [
        "https://api.ipify.org",
        "https://api64.ipify.org",
        "https://ifconfig.me/ip",
    ];

    let start_time = std::time::Instant::now();
    let mut last_error = String::new();

    for provider in &providers {
        match client.get(*provider).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    match res.text().await {
                        Ok(body) => {
                            let exit_ip = body.trim().to_string();
                            if !exit_ip.is_empty() {
                                let latency_ms = start_time.elapsed().as_millis() as u64;
                                // True tunnel verification logic
                                let tunnel_active = !exit_ip.is_empty() && !local_ip.is_empty() && exit_ip != local_ip;

                                return Ok(ConnectionHealthResult {
                                    success: true,
                                    tunnel_active,
                                    exit_ip,
                                    local_ip: local_ip.clone(),
                                    latency_ms,
                                    error: "None".to_string(),
                                });
                            }
                        }
                        Err(e) => {
                            last_error = format!("Failed to read body from {}: {}", provider, e);
                        }
                    }
                } else {
                    last_error = format!("HTTP error from {}: status {}", provider, res.status());
                }
            }
            Err(e) => {
                last_error = format!("Request to {} failed: {}", provider, e);
            }
        }
    }

    // If all providers failed
    Ok(ConnectionHealthResult {
        success: false,
        tunnel_active: false,
        exit_ip: String::new(),
        local_ip,
        latency_ms: 0,
        error: if last_error.is_empty() {
            "All connection providers failed to respond.".to_string()
        } else {
            last_error
        },
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ProcessManager {
            processes: Mutex::new(HashMap::new()),
            local_ip_cache: Mutex::new(None),
            logs: Mutex::new(HashMap::new()),
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
            get_profile_status,
            test_proxy_connection,
            get_local_public_ip,
            get_profile_logs,
            clear_profile_logs
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
