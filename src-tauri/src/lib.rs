use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

static TRAY_NOTIFIED: AtomicBool = AtomicBool::new(false);

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct WindowState {
    width: f64,
    height: f64,
    x: i32,
    y: i32,
}

fn save_window_state(window: &tauri::Window) {
    if let (Ok(is_minimized), Ok(is_maximized)) = (window.is_minimized(), window.is_maximized()) {
        if is_minimized || is_maximized {
            return;
        }
    }

    if let (Ok(physical_size), Ok(physical_pos), Ok(scale_factor)) = (
        window.outer_size(),
        window.outer_position(),
        window.scale_factor(),
    ) {
        let logical_size = physical_size.to_logical::<f64>(scale_factor);
        let logical_pos = physical_pos.to_logical::<f64>(scale_factor);

        // Basic sanity check to avoid invalid coordinates or zero/negative size
        if logical_size.width <= 100.0 || logical_size.height <= 100.0 {
            return;
        }

        let state = WindowState {
            width: logical_size.width,
            height: logical_size.height,
            x: logical_pos.x.round() as i32,
            y: logical_pos.y.round() as i32,
        };

        if let Ok(app_dir) = window.app_handle().path().app_data_dir() {
            let file_path = app_dir.join("window-state.json");
            // Ensure directory exists
            let _ = std::fs::create_dir_all(&app_dir);
            if let Ok(json) = serde_json::to_string(&state) {
                let _ = std::fs::write(file_path, json);
            }
        }
    }
}

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
                "interface" => match key.as_str() {
                    "privatekey" => private_key = Some(val),
                    "address" => address = Some(val),
                    "dns" => dns = val,
                    _ => {}
                },
                "peer" => match key.as_str() {
                    "publickey" => public_key = Some(val),
                    "endpoint" => endpoint = Some(val),
                    "allowedips" => allowed_ips = Some(val),
                    _ => {}
                },
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
            let name = path
                .file_name()
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
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    std::fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    let file_path = app_dir.join("profiles.json");
    std::fs::write(&file_path, profiles_json)
        .map_err(|e| format!("Failed to write profiles: {}", e))?;

    // Keep in-memory cache in sync with disk so the tray/connect handlers
    // reflect the latest profile list without re-reading the file.
    if let Some(state) = app_handle.try_state::<ProcessManager>() {
        invalidate_profiles_cache(&state);
    }

    Ok(())
}

#[tauri::command]
fn load_profiles(app_handle: tauri::AppHandle) -> Result<String, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let file_path = app_dir.join("profiles.json");
    if !file_path.exists() {
        return Ok("[]".to_string());
    }

    std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read profiles: {}", e))
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
        if let Some(value) = line.strip_prefix("# GeneratedAt:") {
            generated_at = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("# ProxyType:") {
            proxy_type = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("# Port:") {
            if let Ok(p) = value.trim().parse::<u16>() {
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

    let app_dir = app_handle
        .path()
        .app_data_dir()
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
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let file_path = app_dir
        .join("generated")
        .join(format!("{}.conf", profile_id));

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
    hide_dock_icon: Option<bool>,
    disable_logs: Option<bool>,
}

#[tauri::command]
fn load_settings(app_handle: tauri::AppHandle) -> Result<AppSettings, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let file_path = app_dir.join("settings.json");
    if !file_path.exists() {
        return Ok(AppSettings {
            wireproxy_binary_path: String::new(),
            hide_dock_icon: Some(false),
            disable_logs: Some(false),
        });
    }

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read settings: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))
}

#[tauri::command]
fn save_settings(app_handle: tauri::AppHandle, settings: AppSettings) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let file_path = app_dir.join("settings.json");
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }

    let content = serde_json::to_string(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(&file_path, content).map_err(|e| format!("Failed to write settings: {}", e))?;

    // Update disable_logs in ProcessManager
    let disable = settings.disable_logs.unwrap_or(false);
    if let Some(state) = app_handle.try_state::<ProcessManager>() {
        state
            .disable_logs
            .store(disable, std::sync::atomic::Ordering::Relaxed);
        if disable {
            let mut logs_map = state.logs.lock();
            logs_map.clear();
        }
    }

    // Dynamically toggle activation policy on macOS
    #[cfg(target_os = "macos")]
    {
        let hide = settings.hide_dock_icon.unwrap_or(false);
        let policy = if hide {
            tauri::ActivationPolicy::Accessory
        } else {
            tauri::ActivationPolicy::Regular
        };
        let _ = app_handle.set_activation_policy(policy);
    }

    Ok(())
}

use std::collections::{HashMap, VecDeque};

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

#[derive(Clone, Default)]
pub struct WorkerStartupCount {
    pub encryption: usize,
    pub decryption: usize,
    pub handshake: usize,
}

struct RunningProcess {
    child: std::process::Child,
    info_port: u16,
    started_at: std::time::Instant,
}

struct StatsCacheEntry {
    last_tx_bytes: u64,
    last_rx_bytes: u64,
    last_polled_at: std::time::Instant,
}

// ============================================================
// Persistent data-usage tracking
// ============================================================

/// One calendar day's accumulated bytes. Serialized as `{uploaded, downloaded}`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Default, Copy)]
pub struct UsageDay {
    pub uploaded: u64,
    pub downloaded: u64,
}

/// On-disk shape of `usage.json`. `days` is keyed by local `YYYY-MM-DD`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct UsageStore {
    #[serde(default)]
    pub days: std::collections::BTreeMap<String, UsageDay>,
    #[serde(default)]
    pub last_updated: Option<String>,
}

/// Aggregated view handed to the frontend: each window is an uploaded/downloaded pair.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsageBucket {
    pub uploaded: u64,
    pub downloaded: u64,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UsageOverview {
    pub today: UsageBucket,
    pub last_24h: UsageBucket,
    pub week: UsageBucket,
    pub month: UsageBucket,
    /// All-time total across every recorded day.
    pub total: UsageBucket,
}

/// Local calendar-day key, e.g. `2026-06-21`. Uses the machine's local timezone
/// via chrono's `Local`, matching how a user thinks about "today".
fn today_key() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Prune days older than `retain_days` to bound `usage.json` growth.
fn prune_old_days(store: &mut UsageStore, retain_days: i64) {
    let cutoff = chrono::Local::now() - chrono::Duration::days(retain_days);
    let cutoff_key = cutoff.format("%Y-%m-%d").to_string();
    store.days.retain(|k, _| k.as_str() >= cutoff_key.as_str());
}

/// Read `usage.json` from app_data_dir, or return an empty store if missing/invalid.
fn load_usage_store(app_handle: &tauri::AppHandle) -> UsageStore {
    let app_dir = match app_handle.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return UsageStore::default(),
    };
    let file_path = app_dir.join("usage.json");
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(_) => return UsageStore::default(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// Write `usage.json` atomically: write to a temp file then rename, so a crash
/// mid-write can't corrupt the existing store.
fn write_usage_store(app_handle: &tauri::AppHandle, store: &UsageStore) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    std::fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create dir: {}", e))?;

    let file_path = app_dir.join("usage.json");
    let tmp_path = app_dir.join("usage.json.tmp");

    let mut to_write = store.clone();
    to_write.last_updated =
        Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    prune_old_days(&mut to_write, 90);

    let json = serde_json::to_string(&to_write)
        .map_err(|e| format!("Failed to serialize usage: {}", e))?;
    std::fs::write(&tmp_path, json)
        .map_err(|e| format!("Failed to write usage temp file: {}", e))?;
    std::fs::rename(&tmp_path, &file_path)
        .map_err(|e| format!("Failed to rename usage file: {}", e))?;
    Ok(())
}

/// Add a byte delta to today's bucket in the in-memory store.
fn record_usage_delta(store: &mut UsageStore, uploaded_delta: u64, downloaded_delta: u64) {
    if uploaded_delta == 0 && downloaded_delta == 0 {
        return;
    }
    let key = today_key();
    let day = store.days.entry(key).or_default();
    day.uploaded = day.uploaded.saturating_add(uploaded_delta);
    day.downloaded = day.downloaded.saturating_add(downloaded_delta);
}

/// Compute the {today, last24h, week, month} overview from the day map.
/// `week`/`month` roll up by local calendar-day membership.
fn compute_usage_overview(store: &UsageStore) -> UsageOverview {
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();

    // Last 24h: strictly, a rolling 24-hour window. Day-granularity data can't
    // represent sub-day slices, so we approximate by counting today + yesterday
    // for the rolling window (the best a day-keyed store can do without timestamps).
    let yesterday = (now - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    let week_start = (now - chrono::Duration::days(6))
        .format("%Y-%m-%d")
        .to_string();
    // First day of the current month.
    let month_start = {
        use chrono::Datelike;
        now.date_naive()
            .with_day(1)
            .unwrap_or_else(|| now.date_naive())
            .format("%Y-%m-%d")
            .to_string()
    };

    let mut today_b = UsageBucket {
        uploaded: 0,
        downloaded: 0,
    };
    let mut last24_b = UsageBucket {
        uploaded: 0,
        downloaded: 0,
    };
    let mut week_b = UsageBucket {
        uploaded: 0,
        downloaded: 0,
    };
    let mut month_b = UsageBucket {
        uploaded: 0,
        downloaded: 0,
    };
    // All-time total: every recorded day, regardless of window.
    let mut total_b = UsageBucket {
        uploaded: 0,
        downloaded: 0,
    };

    for (key, day) in &store.days {
        let bucket = UsageBucket {
            uploaded: day.uploaded,
            downloaded: day.downloaded,
        };
        // All-time accumulator.
        total_b = UsageBucket {
            uploaded: total_b.uploaded + bucket.uploaded,
            downloaded: total_b.downloaded + bucket.downloaded,
        };
        if key == &today {
            today_b = UsageBucket {
                uploaded: today_b.uploaded + bucket.uploaded,
                downloaded: today_b.downloaded + bucket.downloaded,
            };
        }
        if key == &today || key == &yesterday {
            last24_b = UsageBucket {
                uploaded: last24_b.uploaded + bucket.uploaded,
                downloaded: last24_b.downloaded + bucket.downloaded,
            };
        }
        if key.as_str() >= week_start.as_str() {
            week_b = UsageBucket {
                uploaded: week_b.uploaded + bucket.uploaded,
                downloaded: week_b.downloaded + bucket.downloaded,
            };
        }
        if key.as_str() >= month_start.as_str() {
            month_b = UsageBucket {
                uploaded: month_b.uploaded + bucket.uploaded,
                downloaded: month_b.downloaded + bucket.downloaded,
            };
        }
    }

    UsageOverview {
        today: today_b,
        last_24h: last24_b,
        week: week_b,
        month: month_b,
        total: total_b,
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStats {
    pub status: String,
    pub uploaded_bytes_total: u64,
    pub downloaded_bytes_total: u64,
    pub upload_speed_bytes_per_sec: u64,
    pub download_speed_bytes_per_sec: u64,
    pub last_handshake: String,
    pub last_handshake_age_secs: Option<u64>,
    pub connected_for_secs: u64,
}

/// Fetch the final byte totals for a live profile, record the tail delta into
/// the usage store, and persist `usage.json`. Call this BEFORE killing the
/// child or removing the stats cache entry — wireproxy's counters die with the
/// process, so this is the only chance to capture traffic since the last poll.
///
/// `info_port` must come from a still-running `RunningProcess`.
fn flush_profile_usage_blocking(
    app_handle: &tauri::AppHandle,
    state: &ProcessManager,
    profile_id: &str,
    info_port: u16,
) {
    // Best-effort final /metrics fetch (short timeout — the process is about to die).
    let url = format!("http://127.0.0.1:{}/metrics", info_port);
    let body = match reqwest::blocking::get(&url) {
        Ok(res) if res.status().is_success() => res.text().unwrap_or_default(),
        _ => String::new(),
    };

    let mut tx_bytes: u64 = 0;
    let mut rx_bytes: u64 = 0;
    for line in body.lines() {
        if let Some(eq_idx) = line.find('=') {
            let key = line[..eq_idx].trim();
            let val = line[eq_idx + 1..].trim();
            match key {
                "tx_bytes" => tx_bytes = val.parse().unwrap_or(0),
                "rx_bytes" => rx_bytes = val.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    // Diff against the last recorded baseline and accumulate.
    let (up_delta, down_delta) = {
        let mut stats_map = state.stats_cache.lock();
        if let Some(entry) = stats_map.get_mut(profile_id) {
            let up = tx_bytes.saturating_sub(entry.last_tx_bytes);
            let down = rx_bytes.saturating_sub(entry.last_rx_bytes);
            entry.last_tx_bytes = tx_bytes;
            entry.last_rx_bytes = rx_bytes;
            (up, down)
        } else {
            (0, 0)
        }
    };

    let mut dirty = false;
    {
        let mut usage = state.usage_store.lock();
        if up_delta > 0 || down_delta > 0 {
            record_usage_delta(&mut usage, up_delta, down_delta);
            dirty = true;
        }
    }
    if dirty {
        let snapshot = state.usage_store.lock().clone();
        let _ = write_usage_store(app_handle, &snapshot);
    }
}

pub struct ProcessManager {
    processes: Mutex<HashMap<String, RunningProcess>>,
    stats_cache: Mutex<HashMap<String, StatsCacheEntry>>,
    local_ip_cache: Mutex<Option<LocalIpCache>>,
    logs: Mutex<HashMap<String, VecDeque<LogEntry>>>,
    worker_counts: Mutex<HashMap<String, WorkerStartupCount>>,
    selected_profile_id: Mutex<Option<String>>,
    /// In-memory cache of profiles.json so the tray and connect handlers
    /// don't re-read the file on every status update.
    profiles_cache: Mutex<Option<Vec<serde_json::Value>>>,
    /// Shared HTTP client reused across stats polling and health checks.
    http_client: reqwest::Client,
    /// Persistent data-usage store, mirrored in memory and flushed to usage.json.
    usage_store: Mutex<UsageStore>,
    pub disable_logs: std::sync::atomic::AtomicBool,
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
        let is_valid_date = date.len() == 10
            && date.chars().nth(4) == Some('/')
            && date.chars().nth(7) == Some('/');
        let is_valid_time =
            time.len() == 8 && time.chars().nth(2) == Some(':') && time.chars().nth(5) == Some(':');

        if is_valid_level && is_valid_date && is_valid_time {
            return parts[3];
        }
    }
    line
}

enum WorkerType {
    Encryption,
    Decryption,
    Handshake,
}

fn parse_worker_log(message: &str) -> Option<WorkerType> {
    if !message.starts_with("Routine: ") || !message.ends_with(" - started") {
        return None;
    }

    if message.contains(" encryption worker ") {
        Some(WorkerType::Encryption)
    } else if message.contains(" decryption worker ") {
        Some(WorkerType::Decryption)
    } else if message.contains(" handshake worker ") {
        Some(WorkerType::Handshake)
    } else {
        None
    }
}

fn append_log(state: &ProcessManager, profile_id: &str, level: &str, message: &str) {
    if state
        .disable_logs
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return;
    }
    let clean_message = strip_wireproxy_prefix(message);
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    if let Some(worker_type) = parse_worker_log(clean_message) {
        let mut counts_map = state.worker_counts.lock();
        let counts = counts_map.entry(profile_id.to_string()).or_default();
        match worker_type {
            WorkerType::Encryption => counts.encryption += 1,
            WorkerType::Decryption => counts.decryption += 1,
            WorkerType::Handshake => counts.handshake += 1,
        }
        let aggregated_msg = format!(
            "WireGuard workers initialized: {} encryption, {} decryption, {} handshake",
            counts.encryption, counts.decryption, counts.handshake
        );
        drop(counts_map);

        let mut logs = state.logs.lock();
        let entries = logs.entry(profile_id.to_string()).or_default();

        let mut found_idx = None;
        let len = entries.len();
        let lookback = if len > 10 { 10 } else { len };
        for i in 0..lookback {
            let idx = len - 1 - i;
            if entries[idx]
                .message
                .starts_with("WireGuard workers initialized:")
            {
                found_idx = Some(idx);
                break;
            }
        }

        if let Some(idx) = found_idx {
            entries[idx].message = aggregated_msg;
            entries[idx].timestamp = timestamp;
            entries[idx].level = "DEBUG".to_string();
        } else {
            entries.push_back(LogEntry {
                timestamp,
                level: "DEBUG".to_string(),
                message: aggregated_msg,
            });
            if entries.len() > 1000 {
                entries.pop_front();
            }
        }
    } else {
        let mut logs = state.logs.lock();
        let entries = logs.entry(profile_id.to_string()).or_default();
        entries.push_back(LogEntry {
            timestamp,
            level: level.to_string(),
            message: clean_message.to_string(),
        });
        if entries.len() > 1000 {
            entries.pop_front();
        }
    }
}

/// Record a startup failure for a profile in a consistent shape and return it
/// as the command's `Err`. Emits the same trio of log lines every start-time
/// failure path in `start_wireproxy` expects, so each call site stays one line.
fn fail_start(state: &ProcessManager, profile_id: &str, message: String) -> String {
    append_log(state, profile_id, "ERROR", &message);
    append_log(state, profile_id, "ERROR", "Process exited unexpectedly");
    append_log(
        state,
        profile_id,
        "ERROR",
        "WireProxy exited unexpectedly (exit code: unknown)",
    );
    message
}

fn handle_unexpected_exit(app_handle: &tauri::AppHandle, profile_id: &str) {
    let state = app_handle.state::<ProcessManager>();

    // Check if the process is actually dead before removing it from the map
    let is_dead = {
        let mut map = state.processes.lock();
        if let Some(proc) = map.get_mut(profile_id) {
            match proc.child.try_wait() {
                Ok(None) => false, // Still running!
                _ => true,         // Dead or error
            }
        } else {
            false
        }
    };

    if !is_dead {
        return;
    }

    let child_opt = {
        let mut map = state.processes.lock();
        map.remove(profile_id)
    };

    if let Some(mut proc) = child_opt {
        let status = proc.child.wait();
        append_log(&state, profile_id, "INFO", "WireProxy process stopped");
        append_log(&state, profile_id, "ERROR", "Process exited unexpectedly");
        let exit_code_str = match status {
            Ok(s) => match s.code() {
                Some(code) => code.to_string(),
                None => "unknown".to_string(),
            },
            Err(_) => "unknown".to_string(),
        };
        append_log(
            &state,
            profile_id,
            "ERROR",
            &format!(
                "WireProxy exited unexpectedly (exit code: {})",
                exit_code_str
            ),
        );
        let _ = update_tray_menu(app_handle, "Error");

        let profile_name = get_profile_info(app_handle, profile_id)
            .map(|info| info.name)
            .unwrap_or_else(|| "Profile".to_string());
        let _ = app_handle
            .notification()
            .builder()
            .title("WirePort Connection Lost")
            .body(format!("{} stopped unexpectedly.", profile_name))
            .show();
    }
}

#[tauri::command]
fn get_profile_logs(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<Vec<LogEntry>, String> {
    let logs = state.logs.lock();
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
    let mut logs = state.logs.lock();
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

    let platform_bin_name = format!("wireproxy-{}", target_triple);
    let bundled_bin_name = if cfg!(target_os = "windows") {
        "wireproxy.exe".to_string()
    } else {
        "wireproxy".to_string()
    };
    let candidate_names = [bundled_bin_name, platform_bin_name];

    // 1. Production bundle lookup: sidecar binary should be alongside the current executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            for bin_name in &candidate_names {
                let prod_path = exe_dir.join(bin_name);
                if prod_path.exists() && prod_path.is_file() {
                    return Ok(prod_path);
                }
            }
        }
    }

    // 2. Development lookup: sidecar is located in src-tauri/binaries/ relative to resource_dir()
    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        for bin_name in &candidate_names {
            let dev_path = resource_dir.join("binaries").join(bin_name);
            if dev_path.exists() && dev_path.is_file() {
                return Ok(dev_path);
            }
        }
    }

    // 3. Last resort fallback to current working directory
    for bin_name in &candidate_names {
        let fallback_path = std::path::PathBuf::from("binaries").join(bin_name);
        if fallback_path.exists() && fallback_path.is_file() {
            return Ok(fallback_path);
        }
    }

    Err(format!(
        "Could not find bundled WireProxy sidecar binary. Tried: {}",
        candidate_names.join(", ")
    ))
}

fn find_free_port() -> Option<u16> {
    std::net::TcpListener::bind("127.0.0.1:0")
        .ok()
        .and_then(|l| l.local_addr().ok())
        .map(|addr| addr.port())
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
    // Clear worker counts for this profile
    {
        let mut counts = state.worker_counts.lock();
        counts.insert(profile_id.clone(), WorkerStartupCount::default());
    }

    // Clear stats cache entry when starting
    {
        let mut stats_map = state.stats_cache.lock();
        stats_map.remove(&profile_id);
    }

    append_log(&state, &profile_id, "INFO", "Starting WireProxy");

    // 1. Resolve binary path
    let bin_path = if !binary_path.trim().is_empty() {
        std::path::PathBuf::from(&binary_path)
    } else {
        match get_bundled_sidecar_path(&app_handle) {
            Ok(p) => p,
            Err(e) => return Err(fail_start(&state, &profile_id, e)),
        }
    };

    // Verify binary path exists and is a file
    if !bin_path.exists() || !bin_path.is_file() {
        let err_msg = format!("WireProxy binary not found at: {}", bin_path.display());
        return Err(fail_start(&state, &profile_id, err_msg));
    }

    // 2. Verify generated config exists
    let conf_path = std::path::Path::new(&config_path);
    if !conf_path.exists() || !conf_path.is_file() {
        let err_msg = format!("Configuration file not found at: {}", config_path);
        return Err(fail_start(&state, &profile_id, err_msg));
    }

    // 3. Verify port is valid
    if port < 1024 {
        let err_msg = "Port must be between 1024 and 65535".to_string();
        return Err(fail_start(&state, &profile_id, err_msg));
    }

    // 4. Verify no other process is running
    let mut map = state.processes.lock();

    // Clean up dead processes first
    let mut dead_keys = Vec::new();
    for (k, proc) in map.iter_mut() {
        if proc
            .child
            .try_wait()
            .map(|status| status.is_some())
            .unwrap_or(true)
        {
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
            "Another profile is already running. Only one profile can run at a time in V1."
                .to_string()
        };
        return Err(fail_start(&state, &profile_id, err_msg));
    }

    // Allocate dynamic info port
    let info_port = match find_free_port() {
        Some(p) => p,
        None => {
            let err_msg = "Failed to allocate dynamic info port".to_string();
            return Err(fail_start(&state, &profile_id, err_msg));
        }
    };

    // 5. Spawn the child process
    let mut child = match std::process::Command::new(&bin_path)
        .arg("-c")
        .arg(&config_path)
        .arg("-i")
        .arg(format!("127.0.0.1:{}", info_port))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let err_msg = format!("Failed to spawn WireProxy process: {}", e);
            return Err(fail_start(&state, &profile_id, err_msg));
        }
    };

    // 6. Validation: Sleep 200ms and check status
    std::thread::sleep(std::time::Duration::from_millis(200));

    match child.try_wait() {
        Ok(None) => {
            // Still running! Success.
            append_log(&state, &profile_id, "INFO", "WireProxy process started");
            let _ = update_last_connected_at(&app_handle, &profile_id);

            let mut child_to_insert = child;
            let stdout = child_to_insert.stdout.take();
            let stderr = child_to_insert.stderr.take();

            map.insert(
                profile_id.clone(),
                RunningProcess {
                    child: child_to_insert,
                    info_port,
                    started_at: std::time::Instant::now(),
                },
            );
            drop(map); // Release lock before updating tray menu

            let _ = update_tray_menu(&app_handle, "Connected");

            let profile_name = get_profile_info(&app_handle, &profile_id)
                .map(|info| info.name)
                .unwrap_or_else(|| "Profile".to_string());
            let _ = app_handle
                .notification()
                .builder()
                .title("WirePort Connected")
                .body(format!(
                    "{} is now running on 127.0.0.1:{}",
                    profile_name, port
                ))
                .show();

            if let Some(stdout_pipe) = stdout {
                let app_handle_clone = app_handle.clone();
                let profile_id_clone = profile_id.clone();
                std::thread::spawn(move || {
                    let reader = std::io::BufReader::new(stdout_pipe);
                    for msg in reader.lines().map_while(Result::ok) {
                        let state = app_handle_clone.state::<ProcessManager>();
                        append_log(&state, &profile_id_clone, "INFO", &msg);
                    }
                    handle_unexpected_exit(&app_handle_clone, &profile_id_clone);
                });
            }

            if let Some(stderr_pipe) = stderr {
                let app_handle_clone = app_handle.clone();
                let profile_id_clone = profile_id.clone();
                std::thread::spawn(move || {
                    let reader = std::io::BufReader::new(stderr_pipe);
                    for msg in reader.lines().map_while(Result::ok) {
                        let state = app_handle_clone.state::<ProcessManager>();
                        let level = classify_stderr_line(&msg);
                        append_log(&state, &profile_id_clone, level, &msg);
                    }
                    handle_unexpected_exit(&app_handle_clone, &profile_id_clone);
                });
            }

            Ok(())
        }
        Ok(Some(status)) => {
            drop(map); // Release lock immediately
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
            append_log(
                &state,
                &profile_id,
                "ERROR",
                &format!(
                    "WireProxy exited unexpectedly (exit code: {})",
                    exit_code_str
                ),
            );

            let stderr_msg = stderr_content.trim();
            let _ = update_tray_menu(&app_handle, "Error");
            if stderr_msg.is_empty() {
                Err(format!(
                    "WireProxy exited immediately with status: {}",
                    status
                ))
            } else {
                Err(format!("WireProxy failed to start: {}", stderr_msg))
            }
        }
        Err(e) => {
            drop(map); // Release lock immediately
            let err_msg = format!("Failed to check WireProxy status after spawn: {}", e);
            let logged = fail_start(&state, &profile_id, err_msg);
            let _ = update_tray_menu(&app_handle, "Error");
            Err(logged)
        }
    }
}

#[tauri::command]
fn stop_wireproxy(
    app_handle: tauri::AppHandle,
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<(), String> {
    // Capture the live process first so we can flush its final byte totals
    // to usage.json BEFORE killing it (wireproxy's counters die with the process).
    let proc_opt = {
        let mut map = state.processes.lock();
        let info_port = map.get(&profile_id).map(|p| p.info_port);
        if let Some(port) = info_port {
            flush_profile_usage_blocking(&app_handle, &state, &profile_id, port);
        }
        // Clear stats cache when stopping
        {
            let mut stats_map = state.stats_cache.lock();
            stats_map.remove(&profile_id);
        }
        map.remove(&profile_id)
    };

    if let Some(mut proc) = proc_opt {
        let _ = proc.child.kill();
        let _ = proc.child.wait();
        append_log(&state, &profile_id, "INFO", "WireProxy process stopped");
        let _ = update_tray_menu(&app_handle, "Disconnected");

        let profile_name = get_profile_info(&app_handle, &profile_id)
            .map(|info| info.name)
            .unwrap_or_else(|| "Profile".to_string());
        let _ = app_handle
            .notification()
            .builder()
            .title("WirePort Disconnected")
            .body(format!("{} has been stopped.", profile_name))
            .show();
    }
    Ok(())
}

#[tauri::command]
fn get_profile_status(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<String, String> {
    let mut map = state.processes.lock();
    if let Some(proc) = map.get_mut(&profile_id) {
        match proc.child.try_wait() {
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

#[tauri::command]
async fn get_proxy_stats(
    profile_id: String,
    state: tauri::State<'_, ProcessManager>,
) -> Result<ProxyStats, String> {
    let (info_port, started_at) = {
        let mut map = state.processes.lock();
        let proc = match map.get_mut(&profile_id) {
            Some(p) => p,
            None => {
                return Ok(ProxyStats {
                    status: "stopped".to_string(),
                    uploaded_bytes_total: 0,
                    downloaded_bytes_total: 0,
                    upload_speed_bytes_per_sec: 0,
                    download_speed_bytes_per_sec: 0,
                    last_handshake: "Never".to_string(),
                    last_handshake_age_secs: None,
                    connected_for_secs: 0,
                });
            }
        };

        // Check if child is running
        let is_running = matches!(proc.child.try_wait(), Ok(None));

        if !is_running {
            let status = match proc.child.try_wait() {
                Ok(Some(status)) if status.success() => "stopped".to_string(),
                _ => "error".to_string(),
            };
            return Ok(ProxyStats {
                status,
                uploaded_bytes_total: 0,
                downloaded_bytes_total: 0,
                upload_speed_bytes_per_sec: 0,
                download_speed_bytes_per_sec: 0,
                last_handshake: "Never".to_string(),
                last_handshake_age_secs: None,
                connected_for_secs: 0,
            });
        }

        (proc.info_port, proc.started_at)
    };

    let client = &state.http_client;

    let url = format!("http://127.0.0.1:{}/metrics", info_port);
    let response = client.get(&url).send().await;

    let body = match response {
        Ok(res) => {
            if res.status().is_success() {
                res.text().await.unwrap_or_default()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    };

    // Parse the metrics body
    let mut tx_bytes = 0;
    let mut rx_bytes = 0;
    let mut last_handshake_sec = 0;
    let mut last_handshake_nsec = 0;

    for line in body.lines() {
        if let Some(eq_idx) = line.find('=') {
            let key = line[..eq_idx].trim();
            let val = line[eq_idx + 1..].trim();
            match key {
                "tx_bytes" => tx_bytes = val.parse().unwrap_or(0),
                "rx_bytes" => rx_bytes = val.parse().unwrap_or(0),
                "last_handshake_time_sec" => last_handshake_sec = val.parse().unwrap_or(0),
                "last_handshake_time_nsec" => last_handshake_nsec = val.parse().unwrap_or(0),
                _ => {}
            }
        }
    }

    let now = std::time::Instant::now();

    // Re-lock to check cache and compute rate
    let mut stats_map = state.stats_cache.lock();
    let cache_entry = stats_map.get_mut(&profile_id);

    let (upload_speed, download_speed) = match cache_entry {
        Some(entry) => {
            let time_delta = now.duration_since(entry.last_polled_at).as_secs_f64();
            let up = if time_delta > 0.0 && tx_bytes >= entry.last_tx_bytes {
                let diff = tx_bytes - entry.last_tx_bytes;
                (diff as f64 / time_delta) as u64
            } else {
                0
            };
            let down = if time_delta > 0.0 && rx_bytes >= entry.last_rx_bytes {
                let diff = rx_bytes - entry.last_rx_bytes;
                (diff as f64 / time_delta) as u64
            } else {
                0
            };

            // Persist this poll's byte delta into the usage store. The deltas
            // (tx_bytes - last_tx_bytes / rx_bytes - last_rx_bytes) are the
            // actual traffic since the last poll — non-negative because we
            // already checked >= above; clamp to 0 otherwise.
            let up_delta = tx_bytes.saturating_sub(entry.last_tx_bytes);
            let down_delta = rx_bytes.saturating_sub(entry.last_rx_bytes);
            if up_delta > 0 || down_delta > 0 {
                drop(stats_map); // release before taking the usage lock
                {
                    let mut usage = state.usage_store.lock();
                    record_usage_delta(&mut usage, up_delta, down_delta);
                }
                stats_map = state.stats_cache.lock();
            }

            // Re-fetch the entry (the borrow was invalidated by the lock dance above).
            if let Some(entry) = stats_map.get_mut(&profile_id) {
                entry.last_tx_bytes = tx_bytes;
                entry.last_rx_bytes = rx_bytes;
                entry.last_polled_at = now;
            }

            (up, down)
        }
        None => {
            // First stats poll: create cache and return speed = 0
            stats_map.insert(
                profile_id.clone(),
                StatsCacheEntry {
                    last_tx_bytes: tx_bytes,
                    last_rx_bytes: rx_bytes,
                    last_polled_at: now,
                },
            );
            (0, 0)
        }
    };

    let last_handshake = if last_handshake_sec == 0 {
        "Never".to_string()
    } else {
        if let Some(dt) =
            chrono::DateTime::from_timestamp(last_handshake_sec, last_handshake_nsec as u32)
        {
            dt.to_rfc3339()
        } else {
            "Never".to_string()
        }
    };

    let last_handshake_age_secs = if last_handshake_sec == 0 {
        None
    } else {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Some(now_unix.saturating_sub(last_handshake_sec as u64))
    };

    let connected_for_secs = started_at.elapsed().as_secs();

    Ok(ProxyStats {
        status: "running".to_string(),
        uploaded_bytes_total: tx_bytes,
        downloaded_bytes_total: rx_bytes,
        upload_speed_bytes_per_sec: upload_speed,
        download_speed_bytes_per_sec: download_speed,
        last_handshake,
        last_handshake_age_secs,
        connected_for_secs,
    })
}

/// Returns the {today, last24h, week, month} aggregate usage overview, computed
/// from the in-memory day map. Read-only and lockless on disk.
#[tauri::command]
fn get_usage_overview(state: tauri::State<'_, ProcessManager>) -> Result<UsageOverview, String> {
    let store = state.usage_store.lock();
    Ok(compute_usage_overview(&store))
}

/// Returns the raw per-day usage history (`{days: {"2026-06-21": {...}}, lastUpdated}`),
/// for an optional future chart. Ordered oldest-first because BTreeMap sorts keys.
#[tauri::command]
fn get_usage_history(state: tauri::State<'_, ProcessManager>) -> Result<UsageStore, String> {
    Ok(state.usage_store.lock().clone())
}

/// Clears all accumulated usage. Persists the empty store immediately.
#[tauri::command]
fn reset_usage(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, ProcessManager>,
) -> Result<(), String> {
    {
        let mut store = state.usage_store.lock();
        store.days.clear();
        store.last_updated =
            Some(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    }
    let snapshot = state.usage_store.lock().clone();
    write_usage_store(&app_handle, &snapshot)
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

async fn get_local_public_ip_internal(state: &ProcessManager) -> Result<String, String> {
    // Check if cached (5 min = 300 seconds)
    {
        let cache = state.local_ip_cache.lock();
        if let Some(ref cached) = *cache {
            if cached.fetched_at.elapsed() < std::time::Duration::from_secs(300) {
                return Ok(cached.ip.clone());
            }
        }
    }

    // Fetch new using the shared client
    let client = &state.http_client;

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
                            let mut cache = state.local_ip_cache.lock();
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
async fn get_local_public_ip(state: tauri::State<'_, ProcessManager>) -> Result<String, String> {
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
        Err(e) => {
            return Ok(ConnectionHealthResult {
                success: false,
                tunnel_active: false,
                exit_ip: String::new(),
                local_ip: String::new(),
                latency_ms: 0,
                error: format!("Failed to fetch local public IP: {}", e),
            })
        }
    };

    // 2. Get proxy settings
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let file_path = app_dir
        .join("generated")
        .join(format!("{}.conf", profile_id));
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
        Err(e) => {
            return Ok(ConnectionHealthResult {
                success: false,
                tunnel_active: false,
                exit_ip: String::new(),
                local_ip,
                latency_ms: 0,
                error: format!("Failed to configure proxy URL: {}", e),
            })
        }
    };

    let client = match reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(ConnectionHealthResult {
                success: false,
                tunnel_active: false,
                exit_ip: String::new(),
                local_ip,
                latency_ms: 0,
                error: format!("Failed to build HTTP client: {}", e),
            })
        }
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
                                let tunnel_active = !exit_ip.is_empty()
                                    && !local_ip.is_empty()
                                    && exit_ip != local_ip;

                                if tunnel_active {
                                    let _ = update_tray_menu(&app_handle, "Connected");
                                } else {
                                    let _ = update_tray_menu(&app_handle, "Error");
                                }

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
    let _ = update_tray_menu(&app_handle, "Error");
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

struct ProfileInfo {
    name: String,
    proxy_type: String,
    port: u16,
}

/// Read profiles.json as a JSON array, returning an empty Vec if missing/invalid.
fn read_profiles_from_disk(app_handle: &tauri::AppHandle) -> Vec<serde_json::Value> {
    let app_dir = match app_handle.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let file_path = app_dir.join("profiles.json");
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str::<Vec<serde_json::Value>>(&content).unwrap_or_default()
}

/// Return cached profiles, lazily populating the cache from disk on first use.
/// Falls back to a disk read (and refreshes the cache) if the cache is empty.
fn get_cached_profiles(
    app_handle: &tauri::AppHandle,
    state: &ProcessManager,
) -> Vec<serde_json::Value> {
    {
        let guard = state.profiles_cache.lock();
        if let Some(ref cached) = *guard {
            return cached.clone();
        }
    }
    let from_disk = read_profiles_from_disk(app_handle);
    *state.profiles_cache.lock() = Some(from_disk.clone());
    from_disk
}

/// Invalidate the in-memory profiles cache. Call after any mutation
/// (save/delete/update_last_connected_at) so the next read sees fresh data.
fn invalidate_profiles_cache(state: &ProcessManager) {
    *state.profiles_cache.lock() = None;
}

fn get_profile_info(app: &tauri::AppHandle, profile_id: &str) -> Option<ProfileInfo> {
    let state = app.state::<ProcessManager>();
    let profiles = get_cached_profiles(app, &state);
    let profile_obj = profiles
        .iter()
        .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(profile_id))?;

    let name = profile_obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unnamed")
        .to_string();
    let proxy_type = profile_obj
        .get("proxyType")
        .and_then(|v| v.as_str())
        .unwrap_or("socks5")
        .to_string();
    let port = profile_obj
        .get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(1080) as u16;

    Some(ProfileInfo {
        name,
        proxy_type,
        port,
    })
}

fn update_tray_menu(app: &tauri::AppHandle, status: &str) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        let state = app.state::<ProcessManager>();

        let is_connected = status.contains("Connected");
        let status_label = if status.contains("Connected") {
            "🟢 Connected"
        } else if status.contains("Error") {
            "🟡 Error"
        } else {
            "🔴 Disconnected"
        };

        let active_id = {
            let map = state.processes.lock();
            map.keys().next().cloned()
        };

        let selected_id = {
            let guard = state.selected_profile_id.lock();
            guard.clone()
        };

        let display_id = active_id.or(selected_id);

        let (profile_name, proxy_url) = if let Some(ref pid) = display_id {
            if let Some(info) = get_profile_info(app, pid) {
                (
                    info.name,
                    format!("{}://127.0.0.1:{}", info.proxy_type, info.port),
                )
            } else {
                ("None".to_string(), "None".to_string())
            }
        } else {
            ("None".to_string(), "None".to_string())
        };

        let title_i = tauri::menu::MenuItemBuilder::with_id("title", "WirePort")
            .enabled(true)
            .build(app)
            .map_err(|e| e.to_string())?;

        let status_i = tauri::menu::MenuItemBuilder::with_id("status", status_label)
            .enabled(true)
            .build(app)
            .map_err(|e| e.to_string())?;

        let profile_i = tauri::menu::MenuItemBuilder::with_id(
            "profile",
            format!("Current Profile: {}", profile_name),
        )
        .enabled(true)
        .build(app)
        .map_err(|e| e.to_string())?;

        let proxy_i =
            tauri::menu::MenuItemBuilder::with_id("proxy", format!("Proxy: {}", proxy_url))
                .enabled(true)
                .build(app)
                .map_err(|e| e.to_string())?;

        let connect_i = tauri::menu::MenuItemBuilder::with_id("connect", "Connect")
            .enabled(!is_connected)
            .build(app)
            .map_err(|e| e.to_string())?;

        let disconnect_i = tauri::menu::MenuItemBuilder::with_id("disconnect", "Disconnect")
            .enabled(is_connected)
            .build(app)
            .map_err(|e| e.to_string())?;

        let open_i = tauri::menu::MenuItemBuilder::with_id("open", "Open Dashboard")
            .build(app)
            .map_err(|e| e.to_string())?;

        let quit_i = tauri::menu::MenuItemBuilder::with_id("quit", "Quit")
            .build(app)
            .map_err(|e| e.to_string())?;

        let menu = tauri::menu::Menu::with_items(
            app,
            &[
                &title_i,
                &status_i,
                &profile_i,
                &proxy_i,
                &connect_i,
                &disconnect_i,
                &open_i,
                &quit_i,
            ],
        )
        .map_err(|e| e.to_string())?;

        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn update_last_connected_at(app_handle: &tauri::AppHandle, profile_id: &str) -> Result<(), String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let file_path = app_dir.join("profiles.json");
    if !file_path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;
    let mut profiles: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {}", e))?;

    if let Some(arr) = profiles.as_array_mut() {
        let mut found = false;
        for p in arr.iter_mut() {
            if p.get("id").and_then(|id| id.as_str()) == Some(profile_id) {
                let now_str = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                p["lastConnectedAt"] = serde_json::Value::from(now_str);
                found = true;
                break;
            }
        }
        if found {
            let updated_json = serde_json::to_string(&profiles)
                .map_err(|e| format!("Failed to serialize profiles: {}", e))?;
            std::fs::write(&file_path, updated_json)
                .map_err(|e| format!("Failed to write profiles: {}", e))?;
            // Mirror the mutation into the in-memory cache.
            if let Some(state) = app_handle.try_state::<ProcessManager>() {
                invalidate_profiles_cache(&state);
            }
        }
    }
    Ok(())
}

fn quit_app(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<ProcessManager>();

    // Flush final byte totals for every running profile BEFORE killing —
    // wireproxy's counters die with the process, so this is the only chance
    // to capture traffic since the last poll.
    {
        let map = state.processes.lock();
        for (profile_id, proc) in map.iter() {
            flush_profile_usage_blocking(app_handle, &state, profile_id, proc.info_port);
        }
    }

    {
        let mut map = state.processes.lock();
        for (profile_id, mut proc) in map.drain() {
            let _ = proc.child.kill();
            let _ = proc.child.wait();
            // Clear stats cache
            let mut stats_map = state.stats_cache.lock();
            stats_map.remove(&profile_id);
        }
    }

    // Final persistent write so quit captures everything accumulated this session.
    {
        let snapshot = state.usage_store.lock().clone();
        let _ = write_usage_store(app_handle, &snapshot);
    }

    app_handle.exit(0);
}

fn stop_running_profile(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let state = app_handle.state::<ProcessManager>();
    let running_id = {
        let map = state.processes.lock();
        map.keys().next().cloned()
    };
    match running_id {
        Some(profile_id) => {
            stop_wireproxy(app_handle.clone(), profile_id, state)?;
        }
        None => {
            let _ = app_handle
                .notification()
                .builder()
                .title("WirePort")
                .body("No active profile.")
                .show();
        }
    }
    Ok(())
}

fn connect_current_profile(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let state = app_handle.state::<ProcessManager>();
    let selected_id = {
        let id_lock = state.selected_profile_id.lock();
        id_lock.clone()
    };

    let profile_id = match selected_id {
        Some(id) => id,
        None => {
            let _ = app_handle
                .notification()
                .builder()
                .title("No profile selected")
                .body("Open WirePort and select a profile first.")
                .show();
            return Ok(());
        }
    };

    let is_running = {
        let map = state.processes.lock();
        !map.is_empty()
    };
    if is_running {
        let _ = app_handle
            .notification()
            .builder()
            .title("WirePort")
            .body("A profile is already running.")
            .show();
        return Ok(());
    }

    // Load profiles.json
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let file_path = app_dir.join("profiles.json");
    if !file_path.exists() {
        return Err("No profiles found".to_string());
    }
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read profiles: {}", e))?;
    let profiles: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse profiles: {}", e))?;

    let arr = profiles.as_array().ok_or("Invalid profiles format")?;
    let profile_obj = arr
        .iter()
        .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(&profile_id))
        .ok_or(format!("Profile not found: {}", profile_id))?;

    let port = profile_obj
        .get("port")
        .and_then(|v| v.as_u64())
        .ok_or("Port not found in profile")? as u16;

    // Get settings
    let settings = load_settings(app_handle.clone()).unwrap_or(AppSettings {
        wireproxy_binary_path: String::new(),
        hide_dock_icon: Some(false),
        disable_logs: Some(false),
    });

    let config_path = app_dir
        .join("generated")
        .join(format!("{}.conf", profile_id));
    let config_path_str = config_path.to_string_lossy().to_string();

    // Call start_wireproxy
    start_wireproxy(
        app_handle.clone(),
        profile_id,
        config_path_str,
        settings.wireproxy_binary_path,
        port,
        state,
    )?;

    Ok(())
}

#[tauri::command]
fn set_selected_profile(
    app_handle: tauri::AppHandle,
    profile_id: Option<String>,
    state: tauri::State<'_, ProcessManager>,
) -> Result<(), String> {
    {
        let mut selected = state.selected_profile_id.lock();
        *selected = profile_id;
    }

    // Update tray menu status
    let status = {
        let map = state.processes.lock();
        if map.is_empty() {
            "Disconnected"
        } else {
            "Connected"
        }
    };
    let _ = update_tray_menu(&app_handle, status);

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Single shared HTTP client — avoids rebuilding a connection pool on every poll.
    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("failed to build shared reqwest client");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(ProcessManager {
            processes: Mutex::new(HashMap::new()),
            stats_cache: Mutex::new(HashMap::new()),
            local_ip_cache: Mutex::new(None),
            logs: Mutex::new(HashMap::new()),
            worker_counts: Mutex::new(HashMap::new()),
            selected_profile_id: Mutex::new(None),
            profiles_cache: Mutex::new(None),
            http_client,
            usage_store: Mutex::new(UsageStore::default()),
            disable_logs: std::sync::atomic::AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            pick_parse_and_validate_file,
            save_profiles,
            load_profiles,
            generate_wireproxy_config,
            load_generated_config,
            load_settings,
            save_settings,
            start_wireproxy,
            stop_wireproxy,
            get_profile_status,
            test_proxy_connection,
            get_local_public_ip,
            get_profile_logs,
            clear_profile_logs,
            get_proxy_stats,
            get_usage_overview,
            get_usage_history,
            reset_usage,
            set_selected_profile
        ])
        .setup(|app| {
            // Load persistent usage store from disk into memory.
            {
                let store = load_usage_store(app.handle());
                if let Some(state) = app.try_state::<ProcessManager>() {
                    *state.usage_store.lock() = store;
                }
            }

            // Periodic 30s flush: persist usage.json so a crash never loses more
            // than ~30s of accumulated traffic. The thread owns a weak-ish handle
            // clone and exits when the app does.
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    let state = match handle.try_state::<ProcessManager>() {
                        Some(s) => s,
                        None => continue,
                    };
                    let store = state.usage_store.lock().clone();
                    let _ = write_usage_store(&handle, &store);
                });
            }

            // Load settings to set initial state of disable_logs
            if let Some(state) = app.try_state::<ProcessManager>() {
                if let Ok(app_dir) = app.path().app_data_dir() {
                    let file_path = app_dir.join("settings.json");
                    if file_path.exists() {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                                state.disable_logs.store(
                                    settings.disable_logs.unwrap_or(false),
                                    std::sync::atomic::Ordering::Relaxed,
                                );
                            }
                        }
                    }
                }
            }

            let icon_bytes = include_bytes!("../icons/32x32.png");
            let icon = tauri::image::Image::from_bytes(icon_bytes)?;

            let _tray = tauri::tray::TrayIconBuilder::with_id("main")
                .icon(icon)
                .icon_as_template(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => {
                        quit_app(app);
                    }
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "connect" => {
                        if let Err(err) = connect_current_profile(app) {
                            let _ = app
                                .notification()
                                .builder()
                                .title("WirePort Connection Failed")
                                .body(err)
                                .show();
                        }
                    }
                    "disconnect" => {
                        if let Err(err) = stop_running_profile(app) {
                            let _ = app
                                .notification()
                                .builder()
                                .title("WirePort Disconnect Failed")
                                .body(err)
                                .show();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            let handle = app.handle();
            let _ = update_tray_menu(handle, "Disconnected");

            // Apply activation policy on startup for macOS if hide_dock_icon is enabled
            #[cfg(target_os = "macos")]
            {
                if let Ok(settings) = load_settings(handle.clone()) {
                    if settings.hide_dock_icon.unwrap_or(false) {
                        app.set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                }
            }

            // Restore window state from window-state.json
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(app_dir) = app.path().app_data_dir() {
                    let file_path = app_dir.join("window-state.json");
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        if let Ok(state) = serde_json::from_str::<WindowState>(&content) {
                            if state.width > 100.0 && state.height > 100.0 {
                                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                                    width: state.width,
                                    height: state.height,
                                }));

                                if state.x > -10000
                                    && state.x < 10000
                                    && state.y > -10000
                                    && state.y < 10000
                                {
                                    let _ = window.set_position(tauri::Position::Logical(
                                        tauri::LogicalPosition {
                                            x: state.x as f64,
                                            y: state.y as f64,
                                        },
                                    ));
                                }
                            }
                        }
                    }
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();

                if !TRAY_NOTIFIED.swap(true, Ordering::Relaxed) {
                    let _ = window
                        .app_handle()
                        .notification()
                        .builder()
                        .title("WirePort")
                        .body("WirePort is still running in the system tray")
                        .show();
                }
            } else if let tauri::WindowEvent::Destroyed = event {
                if let Some(state) = window.try_state::<ProcessManager>() {
                    // Flush final byte totals before killing children.
                    {
                        let map = state.processes.lock();
                        let app_handle = window.app_handle();
                        for (profile_id, proc) in map.iter() {
                            flush_profile_usage_blocking(
                                app_handle,
                                &state,
                                profile_id,
                                proc.info_port,
                            );
                        }
                    }
                    let mut map = state.processes.lock();
                    for (_, mut proc) in map.drain() {
                        let _ = proc.child.kill();
                        let _ = proc.child.wait();
                    }
                    // Persist anything accumulated this session.
                    let snapshot = state.usage_store.lock().clone();
                    let _ = write_usage_store(window.app_handle(), &snapshot);
                }
            } else if matches!(
                event,
                tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_)
            ) {
                save_window_state(window);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CONFIG: &str = "[Interface]
PrivateKey = abc123privatekey
Address = 10.0.0.2/32
DNS = 1.1.1.1

[Peer]
PublicKey = xyz789publickey
Endpoint = vpn.example.com:51820
AllowedIPs = 0.0.0.0/0
";

    #[test]
    fn parses_valid_config() {
        let parsed = parse_wg_config(VALID_CONFIG).expect("valid config should parse");
        assert_eq!(parsed.address, "10.0.0.2/32");
        assert_eq!(parsed.dns, "1.1.1.1");
        assert_eq!(parsed.endpoint, "vpn.example.com:51820");
        assert_eq!(parsed.allowed_ips, "0.0.0.0/0");
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let config = "# This is a comment
; semicolon comment too

[Interface]
PrivateKey = key
Address = 10.5.5.5/24

[Peer]
# nested comment
PublicKey = pub
Endpoint = host:443
AllowedIPs = 10.0.0.0/8
";
        let parsed = parse_wg_config(config).expect("config with comments should parse");
        assert_eq!(parsed.address, "10.5.5.5/24");
        assert_eq!(parsed.endpoint, "host:443");
    }

    #[test]
    fn handles_whitespace_around_keys_and_values() {
        let config = "[Interface]
   PrivateKey    =    spacedkey
Address = 10.0.0.2/32

[Peer]
PublicKey =pub
Endpoint= vpn.example.com:51820
AllowedIPs= 0.0.0.0/0
";
        let parsed = parse_wg_config(config).expect("whitespace should be tolerated");
        assert_eq!(parsed.endpoint, "vpn.example.com:51820");
        assert_eq!(parsed.allowed_ips, "0.0.0.0/0");
    }

    #[test]
    fn rejects_config_missing_required_fields() {
        // Missing AllowedIPs
        let config = "[Interface]
PrivateKey = key
Address = 10.0.0.2/32

[Peer]
PublicKey = pub
Endpoint = vpn.example.com:51820
";
        assert!(parse_wg_config(config).is_none());

        // Missing PrivateKey
        let config2 = "[Interface]
Address = 10.0.0.2/32

[Peer]
PublicKey = pub
Endpoint = vpn.example.com:51820
AllowedIPs = 0.0.0.0/0
";
        assert!(parse_wg_config(config2).is_none());
    }

    #[test]
    fn dns_is_optional_and_defaults_to_empty() {
        let config = "[Interface]
PrivateKey = key
Address = 10.0.0.2/32

[Peer]
PublicKey = pub
Endpoint = vpn.example.com:51820
AllowedIPs = 0.0.0.0/0
";
        let parsed = parse_wg_config(config).expect("config without DNS should parse");
        assert_eq!(parsed.dns, "");
    }

    #[test]
    fn empty_input_returns_none() {
        assert!(parse_wg_config("").is_none());
    }

    #[test]
    fn parses_multiple_allowed_ips() {
        let config = "[Interface]
PrivateKey = key
Address = 10.0.0.2/32

[Peer]
PublicKey = pub
Endpoint = vpn.example.com:51820
AllowedIPs = 10.0.0.0/24, 192.168.1.0/24
";
        let parsed = parse_wg_config(config).expect("multi-IP config should parse");
        assert_eq!(parsed.allowed_ips, "10.0.0.0/24, 192.168.1.0/24");
    }

    #[test]
    fn parse_meta_comments_extracts_metadata() {
        let content = "# GeneratedAt: 2024-01-15T10:30:00Z
# ProxyType: http
# Port: 8080

[Interface]
PrivateKey = key
";
        let (generated_at, proxy_type, port) = parse_meta_comments(content);
        assert_eq!(generated_at, "2024-01-15T10:30:00Z");
        assert_eq!(proxy_type, "http");
        assert_eq!(port, 8080);
    }

    #[test]
    fn parse_meta_comments_uses_defaults_when_missing() {
        let content = "[Interface]
PrivateKey = key
";
        let (generated_at, proxy_type, port) = parse_meta_comments(content);
        assert_eq!(generated_at, "");
        assert_eq!(proxy_type, "socks5");
        assert_eq!(port, 1080);
    }

    #[test]
    fn classifies_log_levels_correctly() {
        assert_eq!(classify_stderr_line("something failed"), "ERROR");
        assert_eq!(classify_stderr_line("ERROR: bad thing"), "ERROR");
        assert_eq!(classify_stderr_line("warning: deprecated"), "WARN");
        assert_eq!(classify_stderr_line("just info"), "DEBUG");
    }

    #[test]
    fn today_key_is_iso_date() {
        let key = today_key();
        // YYYY-MM-DD shape: 4 digits, dash, 2 digits, dash, 2 digits.
        assert_eq!(key.len(), 10);
        assert_eq!(key.chars().nth(4), Some('-'));
        assert_eq!(key.chars().nth(7), Some('-'));
    }

    #[test]
    fn record_usage_delta_accumulates_into_today() {
        let mut store = UsageStore::default();
        let key = today_key();

        record_usage_delta(&mut store, 100, 200);
        record_usage_delta(&mut store, 50, 25);

        let day = &store.days[&key];
        assert_eq!(day.uploaded, 150);
        assert_eq!(day.downloaded, 225);
    }

    #[test]
    fn record_usage_delta_ignores_zero() {
        let mut store = UsageStore::default();
        record_usage_delta(&mut store, 0, 0);
        assert!(
            store.days.is_empty(),
            "zero delta should not create a day entry"
        );
    }

    #[test]
    fn compute_usage_overview_sums_windows() {
        let today = today_key();
        let yesterday = (chrono::Local::now() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let ten_days_ago = (chrono::Local::now() - chrono::Duration::days(10))
            .format("%Y-%m-%d")
            .to_string();

        let mut store = UsageStore::default();
        store.days.insert(
            today.clone(),
            UsageDay {
                uploaded: 1000,
                downloaded: 2000,
            },
        );
        store.days.insert(
            yesterday.clone(),
            UsageDay {
                uploaded: 500,
                downloaded: 800,
            },
        );
        store.days.insert(
            ten_days_ago,
            UsageDay {
                uploaded: 9999,
                downloaded: 9999,
            }, // outside week/month-window? month still
        );

        let ov = compute_usage_overview(&store);

        // Today = just today's bytes.
        assert_eq!(ov.today.uploaded, 1000);
        assert_eq!(ov.today.downloaded, 2000);

        // Last 24h = today + yesterday (best a day-keyed store can do).
        assert_eq!(ov.last_24h.uploaded, 1500);
        assert_eq!(ov.last_24h.downloaded, 2800);

        // Week = rolling 7 days -> today + yesterday (10 days ago excluded).
        assert_eq!(ov.week.uploaded, 1500);
        assert_eq!(ov.week.downloaded, 2800);

        // Month = first-of-month to now -> all three (assuming 10 days ago is
        // still this month; if today is the 1st-10th, ten_days_ago rolls into
        // last month and the assertion below still holds because month sum
        // would then exclude it — so we assert month >= week).
        assert!(
            ov.month.uploaded >= ov.week.uploaded,
            "month should include at least the week's bytes"
        );
        assert!(
            ov.month.downloaded >= ov.week.downloaded,
            "month should include at least the week's bytes"
        );

        // All-time total = sum of all three day entries (1000+500+9999, 2000+800+9999).
        assert_eq!(ov.total.uploaded, 11499);
        assert_eq!(ov.total.downloaded, 12799);
    }

    #[test]
    fn compute_usage_overview_empty_store_reports_zero() {
        let store = UsageStore::default();
        let ov = compute_usage_overview(&store);
        assert_eq!(ov.today.uploaded, 0);
        assert_eq!(ov.today.downloaded, 0);
        assert_eq!(ov.last_24h.uploaded, 0);
        assert_eq!(ov.week.uploaded, 0);
        assert_eq!(ov.month.uploaded, 0);
        assert_eq!(ov.total.uploaded, 0);
        assert_eq!(ov.total.downloaded, 0);
    }

    #[test]
    fn prune_old_days_removes_entries_past_retention() {
        let mut store = UsageStore::default();
        let old = (chrono::Local::now() - chrono::Duration::days(120))
            .format("%Y-%m-%d")
            .to_string();
        let recent = (chrono::Local::now() - chrono::Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();
        store.days.insert(
            old,
            UsageDay {
                uploaded: 1,
                downloaded: 1,
            },
        );
        store.days.insert(
            recent.clone(),
            UsageDay {
                uploaded: 2,
                downloaded: 2,
            },
        );

        prune_old_days(&mut store, 90);

        assert!(!store.days.contains_key(
            &(chrono::Local::now() - chrono::Duration::days(120))
                .format("%Y-%m-%d")
                .to_string()
        ));
        assert!(store.days.contains_key(&recent));
    }
}
