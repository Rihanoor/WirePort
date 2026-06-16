import React, { useState, useEffect } from "react";
import { 
  Shield, Globe, Calendar, Copy, Check, Eye, EyeOff, 
  FileText, ArrowUpRight, CheckCircle2, XCircle, AlertCircle, HelpCircle, FileCode,
  Activity, RefreshCw, Clock, Download, Trash2, Terminal, ChevronRight
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Profile, ProxyType, ProfileStatus, GeneratedConfigMeta, ConnectionHealthResult, LogEntry } from "../types";


interface ProfileDetailsProps {
  profile: Profile;
  onUpdate: (updatedProfile: Profile) => void;
  onDelete?: (id: string) => void;
  wireproxyBinaryPath: string;
  onStatusChange: (status: ProfileStatus) => void;
}

export const ProfileDetails: React.FC<ProfileDetailsProps> = ({ 
  profile, 
  onUpdate,
  onDelete,
  wireproxyBinaryPath,
  onStatusChange
}) => {
  const [name, setName] = useState(profile.name);
  const [proxyType, setProxyType] = useState<ProxyType>(profile.proxyType);
  const [port, setPort] = useState(profile.port);
  const [isActionLoading, setIsActionLoading] = useState(false);
  
  // WireProxy config file metadata state
  const [generatedConfigMeta, setGeneratedConfigMeta] = useState<GeneratedConfigMeta | null>(null);
  
  const [showGenConfig, setShowGenConfig] = useState(false);
  const [copiedGenConfig, setCopiedGenConfig] = useState(false);
  const [isRegenerating, setIsRegenerating] = useState(false);
  
  const [showConfig, setShowConfig] = useState(false);
  const [maskKey, setMaskKey] = useState(true);
  const [copied, setCopied] = useState(false);
  const [copiedConfig, setCopiedConfig] = useState(false);

  // Connection Health State
  const [health, setHealth] = useState<ConnectionHealthResult | null>(null);
  const [lastSuccessHealth, setLastSuccessHealth] = useState<ConnectionHealthResult | null>(null);
  const [isCheckingHealth, setIsCheckingHealth] = useState(false);
  const [lastCheckedTime, setLastCheckedTime] = useState<string | null>(null);
  const [autoCheck, setAutoCheck] = useState<boolean>(() => {
    const saved = localStorage.getItem("autoCheckHealth");
    return saved !== null ? saved === "true" : true;
  });

  // Logs State
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [autoScroll, setAutoScroll] = useState<boolean>(() => {
    const saved = localStorage.getItem("autoScrollLogs");
    return saved !== null ? saved === "true" : true;
  });
  const [isRefreshingLogs, setIsRefreshingLogs] = useState(false);
  const [copiedLogs, setCopiedLogs] = useState(false);
  const [isLogsExpanded, setIsLogsExpanded] = useState(false);
  const [logFilter, setLogFilter] = useState<"all" | "info" | "debug" | "warn" | "error">("all");
  const logEndRef = React.useRef<HTMLDivElement>(null);
  const logContainerRef = React.useRef<HTMLDivElement>(null);

  // Auto expand when logs are first received, auto collapse when cleared
  const prevLogsEmptyRef = React.useRef(true);
  useEffect(() => {
    const isEmpty = logs.length === 0;
    if (isEmpty !== prevLogsEmptyRef.current) {
      setIsLogsExpanded(!isEmpty);
      prevLogsEmptyRef.current = isEmpty;
    }
  }, [logs.length === 0]);

  const fetchLogs = async () => {
    try {
      setIsRefreshingLogs(true);
      const fetchedLogs = await invoke<LogEntry[]>("get_profile_logs", {
        profileId: profile.id,
      });
      setLogs(fetchedLogs);
    } catch (err) {
      console.error("Failed to fetch logs:", err);
    } finally {
      setIsRefreshingLogs(false);
    }
  };

  const handleClearLogs = async () => {
    try {
      await invoke("clear_profile_logs", { profileId: profile.id });
      setLogs([]);
    } catch (err) {
      console.error("Failed to clear logs:", err);
    }
  };

  const handleCopyLogs = () => {
    if (logs.length === 0) return;
    const logString = logs
      .map((log) => `[${log.timestamp}] [${log.level.toUpperCase()}] ${log.message}`)
      .join("\n");
    navigator.clipboard.writeText(logString);
    setCopiedLogs(true);
    setTimeout(() => setCopiedLogs(false), 2000);
  };

  const handleDownloadLogs = () => {
    if (logs.length === 0) return;
    const logString = logs
      .map((log) => `[${log.timestamp}] [${log.level.toUpperCase()}] ${log.message}`)
      .join("\n");
    
    const dateStr = new Date().toISOString().split("T")[0];
    const filename = `wireport-logs-${dateStr}.txt`;
    
    const blob = new Blob([logString], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  // Load logs once when profile.id changes
  useEffect(() => {
    fetchLogs();
  }, [profile.id]);

  // Poll logs every 2 seconds while profile is running or starting
  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | null = null;
    
    if (profile.status === "running" || profile.status === "starting") {
      interval = setInterval(() => {
        fetchLogs();
      }, 2000);
    }
    
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [profile.status, profile.id]);

  // Persist autoScroll preference
  useEffect(() => {
    localStorage.setItem("autoScrollLogs", autoScroll.toString());
  }, [autoScroll]);

  // Auto scroll to bottom
  useEffect(() => {
    if (autoScroll && logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: "auto" });
    }
  }, [logs, autoScroll]);


  // Sync state with profile prop when it changes
  useEffect(() => {
    setName(profile.name);
    setProxyType(profile.proxyType);
    setPort(profile.port);
  }, [profile]);

  // Load existing generated WireProxy config from disk on mount or profile change
  useEffect(() => {
    const fetchGenConfig = async () => {
      try {
        const meta = await invoke<GeneratedConfigMeta | null>("load_generated_config", { 
          profileId: profile.id 
        });
        setGeneratedConfigMeta(meta);
      } catch (err) {
        console.error("Failed to load generated config:", err);
        setGeneratedConfigMeta(null);
      }
    };
    
    fetchGenConfig();
  }, [profile.id]);

  // Check status on mount or profile change
  useEffect(() => {
    const checkStatus = async () => {
      try {
        const currentStatus = await invoke<ProfileStatus>("get_profile_status", { profileId: profile.id });
        onStatusChange(currentStatus);
      } catch (err) {
        console.error("Failed to check status:", err);
      }
    };
    
    checkStatus();
    
    const interval = setInterval(checkStatus, 3000);
    return () => clearInterval(interval);
  }, [profile.id]);

  // Persist autoCheck preference
  useEffect(() => {
    localStorage.setItem("autoCheckHealth", autoCheck.toString());
  }, [autoCheck]);

  // Function to run a health check
  const checkConnection = async (force: boolean = false) => {
    if (isCheckingHealth || (profile.status !== "running" && !force)) return;
    setIsCheckingHealth(true);
    try {
      const result = await invoke<ConnectionHealthResult>("test_proxy_connection", {
        profileId: profile.id,
      });
      setHealth(result);
      setLastCheckedTime(new Date().toLocaleTimeString());
      if (result.success) {
        setLastSuccessHealth(result);
      }
    } catch (err: any) {
      console.error("Connection health check failed:", err);
      const errResult: ConnectionHealthResult = {
        success: false,
        tunnelActive: false,
        exitIp: "",
        localIp: "",
        latencyMs: 0,
        error: err.toString() || "Unknown error",
      };
      setHealth(errResult);
      setLastCheckedTime(new Date().toLocaleTimeString());
    } finally {
      setIsCheckingHealth(false);
    }
  };

  // Trigger health check on status change or auto-check interval
  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | null = null;
    
    if (profile.status === "running") {
      // Trigger check immediately when status becomes running
      checkConnection();
      
      if (autoCheck) {
        interval = setInterval(() => {
          checkConnection();
        }, 30000); // 30-second interval
      }
    } else {
      // Reset health state when proxy is not running
      setHealth(null);
      setLastSuccessHealth(null);
      setLastCheckedTime(null);
    }
    
    return () => {
      if (interval) clearInterval(interval);
    };
  }, [profile.status, profile.id, autoCheck]);


  const handleNameChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setName(e.target.value);
  };

  const handleNameBlur = () => {
    const trimmed = name.trim();
    if (trimmed && trimmed !== profile.name) {
      onUpdate({ ...profile, name: trimmed, updatedAt: new Date().toISOString() });
    } else {
      setName(profile.name);
    }
  };

  const handleProxyTypeChange = (type: ProxyType) => {
    setProxyType(type);
    onUpdate({ ...profile, proxyType: type, updatedAt: new Date().toISOString() });
  };

  const handlePortChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = parseInt(e.target.value);
    if (!isNaN(val)) {
      setPort(val);
    }
  };

  const handlePortBlur = () => {
    // Validate port is between 1024 and 65535
    if (port >= 1024 && port <= 65535 && port !== profile.port) {
      onUpdate({ ...profile, port, updatedAt: new Date().toISOString() });
    } else {
      setPort(profile.port);
    }
  };

  const handleCopyEndpoint = () => {
    const endpointStr = `${proxyType}://127.0.0.1:${port}`;
    navigator.clipboard.writeText(endpointStr);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const maskPrivateKey = (content: string, mask: boolean): string => {
    if (!mask) return content;
    return content.replace(/^(PrivateKey\s*=\s*)(.+)$/im, "$1••••••••••••••••••••••••••••••••••••••••");
  };

  const handleCopyConfig = () => {
    // When copying config, copy the original unmasked content
    navigator.clipboard.writeText(profile.configContent);
    setCopiedConfig(true);
    setTimeout(() => setCopiedConfig(false), 2000);
  };

  const handleCopyGenConfig = () => {
    if (generatedConfigMeta) {
      navigator.clipboard.writeText(generatedConfigMeta.content);
      setCopiedGenConfig(true);
      setTimeout(() => setCopiedGenConfig(false), 2000);
    }
  };

  const handleRegenerate = async () => {
    if (port < 1024 || port > 65535) {
      alert("Port must be between 1024 and 65535");
      return;
    }

    setIsRegenerating(true);
    try {
      const timestamp = new Date().toISOString();
      const meta = await invoke<GeneratedConfigMeta>("generate_wireproxy_config", {
        profileId: profile.id,
        proxyType: proxyType,
        port: port,
        configContent: profile.configContent,
        generatedAt: timestamp
      });
      
      setGeneratedConfigMeta(meta);

      // Persist type and port in profiles.json if changed in UI
      if (profile.port !== port || profile.proxyType !== proxyType) {
        onUpdate({ 
          ...profile, 
          port: port, 
          proxyType: proxyType, 
          updatedAt: timestamp 
        });
      }
    } catch (err: any) {
      console.error(err);
      alert(err.toString() || "Failed to generate config file");
    } finally {
      setIsRegenerating(false);
    }
  };

  const handleStartProxy = async () => {
    if (configStatus !== "ready" || !generatedConfigMeta) {
      alert("Please generate or regenerate the proxy configuration file first.");
      return;
    }

    setIsActionLoading(true);
    onStatusChange("starting");
    try {
      await invoke("start_wireproxy", {
        profileId: profile.id,
        configPath: generatedConfigMeta.path,
        binaryPath: wireproxyBinaryPath,
        port: port,
      });
      onStatusChange("running");
    } catch (err: any) {
      console.error("Failed to start wireproxy:", err);
      onStatusChange("error");
      alert(err.toString() || "Failed to start WireProxy");
    } finally {
      setIsActionLoading(false);
    }
  };

  const handleStopProxy = async () => {
    setIsActionLoading(true);
    try {
      await invoke("stop_wireproxy", { profileId: profile.id });
      onStatusChange("stopped");
    } catch (err: any) {
      console.error("Failed to stop wireproxy:", err);
      alert(err.toString() || "Failed to stop WireProxy");
    } finally {
      setIsActionLoading(false);
    }
  };

  const isProxyActive = profile.status === "running" || profile.status === "starting";

  const getStatusIcon = (status: ProfileStatus) => {
    switch (status) {
      case "running":
        return <CheckCircle2 size={16} className="status-icon running" />;
      case "starting":
        return <AlertCircle size={16} className="status-icon starting" />;
      case "error":
        return <XCircle size={16} className="status-icon error" />;
      default:
        return <HelpCircle size={16} className="status-icon stopped" />;
    }
  };

  // Status logic as per requirements:
  // - No generated config loaded: "Not generated"
  // - Current profile proxyType/port differs from generatedConfigMeta proxyType/port: "Needs regeneration"
  // - Otherwise: "Ready"
  let configStatus: "not_generated" | "needs_regeneration" | "ready" = "not_generated";
  if (generatedConfigMeta) {
    if (
      port !== generatedConfigMeta.port || 
      proxyType !== generatedConfigMeta.proxyType ||
      profile.port !== generatedConfigMeta.port ||
      profile.proxyType !== generatedConfigMeta.proxyType
    ) {
      configStatus = "needs_regeneration";
    } else {
      configStatus = "ready";
    }
  }

  const isOutOfSync = configStatus !== "ready";

  return (
    <div className="profile-details-container">
      {/* Header Info */}
      <div className="details-header">
        <div className="details-header-title">
          <div className="details-avatar">
            <Shield size={24} />
          </div>
          <div className="details-name-wrapper">
            <input 
              type="text" 
              className="details-name-input"
              value={name}
              onChange={handleNameChange}
              onBlur={handleNameBlur}
              placeholder="Profile Name"
            />
            <span className="details-meta-tag">WireGuard Profile</span>
          </div>
        </div>

        <div className={`status-badge ${profile.status}`}>
          {getStatusIcon(profile.status)}
          <span>{profile.status.toUpperCase()}</span>
        </div>
      </div>

      <div className="details-body">
        {/* Connection Health Card */}
        {profile.status === "running" && (
          <div className="details-section">
            <h3 className="section-title">Connection Health</h3>
            <div className="health-card">
              <div className="health-card-header">
                <div className="health-status-wrapper">
                  <Activity size={18} className={`health-icon ${isCheckingHealth ? "animate-pulse" : ""}`} />
                  <span className="health-card-title">Connection Stats</span>
                </div>
                <div className="health-actions">
                  <label className="auto-check-toggle" htmlFor="auto-check-cb">
                    <input 
                      id="auto-check-cb"
                      type="checkbox"
                      checked={autoCheck}
                      onChange={(e) => setAutoCheck(e.target.checked)}
                    />
                    <span>Auto-check (30s)</span>
                  </label>
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary btn-health-test"
                    onClick={() => checkConnection(true)}
                    disabled={isCheckingHealth}
                  >
                    {isCheckingHealth ? (
                      <>
                        <RefreshCw size={12} className="animate-spin" />
                        <span>Testing...</span>
                      </>
                    ) : (
                      <>
                        <RefreshCw size={12} />
                        <span>Test Connection</span>
                      </>
                    )}
                  </button>
                </div>
              </div>

              <div className="health-grid">
                <div className="health-metric">
                  <span className="metric-label">Tunnel Active</span>
                  <div className="metric-value">
                    {isCheckingHealth && !health ? (
                      <span className="status-indicator-text checking">Checking...</span>
                    ) : health?.success && health.tunnelActive ? (
                      <span className="status-indicator-text active">
                        <span className="health-dot active" /> Yes
                      </span>
                    ) : (
                      <span className="status-indicator-text inactive">
                        <span className="health-dot inactive" /> No
                      </span>
                    )}
                  </div>
                </div>

                <div className="health-metric">
                  <span className="metric-label">Latency</span>
                  <div className="metric-value font-mono">
                    {health?.success ? (
                      `${health.latencyMs} ms`
                    ) : lastSuccessHealth ? (
                      <span className="fallback-value" title="Last successful latency">
                        {lastSuccessHealth.latencyMs} ms <span className="fallback-tag">(Last Ok)</span>
                      </span>
                    ) : isCheckingHealth ? (
                      "Checking..."
                    ) : (
                      "--"
                    )}
                  </div>
                </div>

                <div className="health-metric">
                  <span className="metric-label">Exit IP</span>
                  <div className="metric-value font-mono">
                    {health?.success ? (
                      health.exitIp
                    ) : lastSuccessHealth ? (
                      <span className="fallback-value" title="Last successful exit IP">
                        {lastSuccessHealth.exitIp} <span className="fallback-tag">(Last Ok)</span>
                      </span>
                    ) : isCheckingHealth ? (
                      "Checking..."
                    ) : (
                      "N/A"
                    )}
                  </div>
                </div>

                <div className="health-metric">
                  <span className="metric-label">Local IP</span>
                  <div className="metric-value font-mono">
                    {health?.localIp ? (
                      health.localIp
                    ) : isCheckingHealth ? (
                      "Checking..."
                    ) : (
                      "Unknown"
                    )}
                  </div>
                </div>

                <div className="health-metric">
                  <span className="metric-label">Last Checked</span>
                  <div className="metric-value font-mono">
                    {lastCheckedTime ? (
                      <span className="timestamp-value">
                        <Clock size={12} style={{ marginRight: '4px', display: 'inline', verticalAlign: 'middle' }} /> {lastCheckedTime}
                      </span>
                    ) : (
                      "Never"
                    )}
                  </div>
                </div>

                <div className="health-metric">
                  {/* Empty block to balance 2-column grid row */}
                </div>

                <div className="health-metric span-2">
                  <span className="metric-label">Last Error</span>
                  <div className={`metric-value error-message-box ${health && !health.success ? "has-error" : ""}`}>
                    {health ? (
                      health.success ? "None" : health.error
                    ) : isCheckingHealth ? (
                      "Checking connection..."
                    ) : (
                      "None"
                    )}
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Connection Setup */}
        <div className="details-section">
          <h3 className="section-title">Local Proxy Settings</h3>
          <div className="settings-grid">
            <div className="form-group">
              <label htmlFor="proxy-type" className="form-label">Proxy Type</label>
              <div className="proxy-type-selector">
                <button 
                  type="button"
                  className={`selector-btn ${proxyType === "socks5" ? "active" : ""}`}
                  onClick={() => !isProxyActive && handleProxyTypeChange("socks5")}
                  disabled={isProxyActive}
                >
                  SOCKS5
                </button>
                <button 
                  type="button"
                  className={`selector-btn ${proxyType === "http" ? "active" : ""}`}
                  onClick={() => !isProxyActive && handleProxyTypeChange("http")}
                  disabled={isProxyActive}
                >
                  HTTP
                </button>
              </div>
            </div>

            <div className="form-group">
              <label htmlFor="proxy-port" className="form-label">Local Port</label>
              <input 
                id="proxy-port"
                type="number"
                min="1024"
                max="65535"
                className="form-input"
                value={port}
                onChange={handlePortChange}
                onBlur={handlePortBlur}
                disabled={isProxyActive}
              />
            </div>

            <div className="form-group span-2">
              <label className="form-label">Local Proxy Endpoint</label>
              <div className="endpoint-copy-box">
                <span className="endpoint-text">{proxyType}://127.0.0.1:{port}</span>
                <button 
                  className="btn-icon copy-endpoint-btn"
                  onClick={handleCopyEndpoint}
                  title="Copy Endpoint"
                  aria-label="Copy Endpoint"
                >
                  {copied ? <Check size={16} className="text-success" /> : <Copy size={16} />}
                </button>
              </div>
            </div>

            <div className="form-group span-2 mt-2">
              <div className="proxy-action-buttons" style={{ display: "flex", gap: "12px" }}>
                {profile.status === "running" ? (
                  <button 
                    type="button" 
                    className="btn btn-danger btn-full"
                    onClick={handleStopProxy}
                    disabled={isActionLoading}
                  >
                    Stop Proxy
                  </button>
                ) : (
                  <button 
                    type="button" 
                    className="btn btn-primary btn-full"
                    onClick={handleStartProxy}
                    disabled={profile.status === "starting" || isActionLoading}
                  >
                    {profile.status === "starting" ? "Starting..." : "Start Proxy"}
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>

        {/* WireProxy Configuration File */}
        <div className="details-section">
          <h3 className="section-title">Proxy Configuration File</h3>
          <div className="config-file-card">
            <div className="config-file-info">
              <div className="config-file-status-wrapper">
                <FileCode size={20} className="config-icon" />
                <div className="config-file-details">
                  <span className="config-filename">profile-{profile.id.slice(0, 8)}.conf</span>
                  <div className="config-status-row">
                    <span className={`config-status-dot ${configStatus === "ready" ? "success" : (configStatus === "needs_regeneration" ? "warning" : "danger")}`} />
                    <span className="config-status-text">
                      {configStatus === "needs_regeneration" 
                        ? "Needs regeneration (Settings changed)" 
                        : (configStatus === "not_generated" ? "Not generated yet" : "Ready (Generated & synced)")}
                    </span>
                  </div>
                </div>
              </div>
              
              <div className="config-file-actions">
                <button 
                  type="button" 
                  className={`btn btn-sm ${showGenConfig ? "btn-secondary" : "btn-primary"}`}
                  onClick={() => setShowGenConfig(!showGenConfig)}
                  disabled={!generatedConfigMeta}
                >
                  {showGenConfig ? "Hide Preview" : "View Preview"}
                </button>
                <button 
                  type="button" 
                  className="btn btn-sm btn-secondary"
                  onClick={handleCopyGenConfig}
                  disabled={!generatedConfigMeta}
                >
                  {copiedGenConfig ? "Copied!" : "Copy"}
                </button>
                <button 
                  type="button" 
                  className={`btn btn-sm ${isOutOfSync ? "btn-primary-glow animate-pulse" : "btn-secondary"}`}
                  onClick={handleRegenerate}
                  disabled={isRegenerating}
                >
                  {isRegenerating ? "Generating..." : (generatedConfigMeta ? "Regenerate" : "Generate Config")}
                </button>
              </div>
            </div>

            {showGenConfig && generatedConfigMeta && (
              <div className="config-viewer-wrapper mt-3">
                <div className="config-toolbar">
                  <span className="file-info-tag">wireproxy.conf ({generatedConfigMeta.proxyType.toUpperCase()} on port {generatedConfigMeta.port})</span>
                  <span className="file-info-tag" style={{ marginLeft: "auto", fontSize: "10px" }}>
                    Last generated: {new Date(generatedConfigMeta.generatedAt).toLocaleTimeString()}
                  </span>
                </div>
                <pre className="config-pre">
                  <code>{maskPrivateKey(generatedConfigMeta.content, maskKey)}</code>
                </pre>
              </div>
            )}
          </div>
        </div>

        {/* WireGuard Configuration Info */}
        <div className="details-section">
          <h3 className="section-title">WireGuard Tunnel Details</h3>
          <div className="meta-grid">
            <div className="meta-card">
              <span className="meta-label">Endpoint</span>
              <span className="meta-value" title={profile.endpoint}>
                <Globe size={13} />
                <span className="truncate">{profile.endpoint}</span>
              </span>
            </div>

            <div className="meta-card">
              <span className="meta-label">Interface Address</span>
              <span className="meta-value" title={profile.address}>
                <ArrowUpRight size={13} />
                <span className="truncate">{profile.address}</span>
              </span>
            </div>

            <div className="meta-card">
              <span className="meta-label">Allowed IPs</span>
              <span className="meta-value" title={profile.allowedIps}>
                <Shield size={13} />
                <span className="truncate">{profile.allowedIps}</span>
              </span>
            </div>

            <div className="meta-card">
              <span className="meta-label">DNS Servers</span>
              <span className="meta-value" title={profile.dns || "None"}>
                <Globe size={13} />
                <span className="truncate">{profile.dns || "None"}</span>
              </span>
            </div>
          </div>
        </div>

        {/* Raw Config Code Block */}
        <div className="details-section">
          <div className="config-header" onClick={() => setShowConfig(!showConfig)}>
            <div className="config-title-wrapper">
              <FileText size={16} />
              <span>Raw WireGuard Config</span>
            </div>
            <button className="btn-toggle-config">
              {showConfig ? "Hide" : "Show"}
            </button>
          </div>

          {showConfig && (
            <div className="config-viewer-wrapper">
              <div className="config-toolbar">
                <span className="file-info-tag">
                  {profile.sourcePath ? `Source: ${profile.sourcePath.split("/").pop()}` : "Config Content"}
                </span>
                <div className="toolbar-actions">
                  <button 
                    className="toolbar-btn"
                    onClick={() => setMaskKey(!maskKey)}
                    title={maskKey ? "Show Private Key" : "Hide Private Key"}
                  >
                    {maskKey ? <Eye size={14} /> : <EyeOff size={14} />}
                    <span>{maskKey ? "Reveal Key" : "Mask Key"}</span>
                  </button>
                  <button 
                    className="toolbar-btn"
                    onClick={handleCopyConfig}
                    title="Copy full configuration content"
                  >
                    {copiedConfig ? <Check size={14} /> : <Copy size={14} />}
                    <span>{copiedConfig ? "Copied!" : "Copy"}</span>
                  </button>
                </div>
              </div>
              <pre className="config-pre">
                <code>{maskPrivateKey(profile.configContent, maskKey)}</code>
              </pre>
            </div>
          )}
        </div>

        {/* Runtime Logs Section */}
        <div className="details-section">
          <div className="logs-section-header" onClick={() => setIsLogsExpanded(!isLogsExpanded)}>
            <h3 className="section-title">
              <ChevronRight size={14} className={`collapse-chevron ${isLogsExpanded ? "expanded" : ""}`} />
              Runtime Logs
            </h3>
            <span className="logs-count-badge">
              {logs.length} {logs.length === 1 ? "entry" : "entries"}
            </span>
          </div>

          {isLogsExpanded && (
            <div className="logs-card">
              <div className="logs-header">
                <div className="logs-title-wrapper">
                  <Terminal size={16} className="logs-title-icon" />
                  <span>Console Output</span>
                </div>
                <div className="logs-actions">
                  <label className="auto-scroll-toggle" htmlFor="auto-scroll-cb">
                    <input 
                      id="auto-scroll-cb"
                      type="checkbox"
                      checked={autoScroll}
                      onChange={(e) => setAutoScroll(e.target.checked)}
                    />
                    <span>Auto Scroll</span>
                  </label>
                  <select
                    value={logFilter}
                    onChange={(e) => setLogFilter(e.target.value as any)}
                    className="log-filter-select"
                  >
                    <option value="all">All Levels</option>
                    <option value="info">Info</option>
                    <option value="debug">Debug</option>
                    <option value="warn">Warn</option>
                    <option value="error">Error</option>
                  </select>
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary btn-logs-action"
                    onClick={fetchLogs}
                    disabled={isRefreshingLogs}
                    title="Refresh logs"
                  >
                    <RefreshCw size={12} className={isRefreshingLogs ? "animate-spin" : ""} />
                    <span>Refresh</span>
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary btn-logs-action"
                    onClick={handleCopyLogs}
                    disabled={logs.length === 0}
                    title="Copy logs to clipboard"
                  >
                    <Copy size={12} />
                    <span>{copiedLogs ? "Copied!" : "Copy"}</span>
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary btn-logs-action"
                    onClick={handleDownloadLogs}
                    disabled={logs.length === 0}
                    title="Download logs as text file"
                  >
                    <Download size={12} />
                    <span>Download</span>
                  </button>
                  <button
                    type="button"
                    className="btn btn-sm btn-secondary btn-logs-action btn-danger-hover"
                    onClick={handleClearLogs}
                    disabled={logs.length === 0}
                    title="Clear log buffer"
                  >
                    <Trash2 size={12} />
                    <span>Clear</span>
                  </button>
                </div>
              </div>

              <div className={`logs-viewer-container ${logs.length === 0 ? "empty" : ""}`} ref={logContainerRef}>
                {logs.length === 0 ? (
                  <div className="logs-empty">
                    <div className="logs-empty-text-wrapper">
                      <p className="logs-empty-primary">No logs available yet.</p>
                      <p className="logs-empty-secondary">Start the proxy to begin collecting runtime output.</p>
                    </div>
                  </div>
                ) : (
                  <div className="logs-list">
                    {logs
                      .filter((log) => {
                        if (logFilter === "all") return true;
                        return log.level.toLowerCase() === logFilter.toLowerCase();
                      })
                      .map((log, index) => (
                        <div key={index} className={`log-entry ${log.level.toLowerCase()}`}>
                          <span className="log-timestamp">{log.timestamp}</span>
                          <span className={`log-level-badge ${log.level.toLowerCase()}`}>{log.level.toUpperCase()}</span>
                          <span className="log-message">{log.message}</span>
                        </div>
                      ))}
                    <div ref={logEndRef} />
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {onDelete && (
          <div className="details-danger-zone">
            <button 
              className="btn btn-danger"
              onClick={() => onDelete(profile.id)}
            >
              Delete Profile
            </button>
          </div>
        )}
      </div>

      <div className="details-footer">
        <div className="footer-meta-item">
          <Calendar size={12} />
          <span>Created: {new Date(profile.createdAt).toLocaleString()}</span>
        </div>
      </div>
    </div>
  );
};
