import React, { useState, useEffect } from "react";
import { 
  Shield, Globe, Calendar, Copy, Check, Eye, EyeOff, 
  FileText, ArrowUpRight, CheckCircle2, XCircle, AlertCircle, HelpCircle
} from "lucide-react";
import { Profile, ProxyType, ProfileStatus } from "../types";

interface ProfileDetailsProps {
  profile: Profile;
  onUpdate: (updatedProfile: Profile) => void;
  onDelete?: (id: string) => void;
}

export const ProfileDetails: React.FC<ProfileDetailsProps> = ({ 
  profile, 
  onUpdate,
  onDelete
}) => {
  const [name, setName] = useState(profile.name);
  const [proxyType, setProxyType] = useState<ProxyType>(profile.proxyType);
  const [port, setPort] = useState(profile.port);
  const [showConfig, setShowConfig] = useState(false);
  const [maskKey, setMaskKey] = useState(true);
  const [copied, setCopied] = useState(false);
  const [copiedConfig, setCopiedConfig] = useState(false);

  // Sync state with profile prop when it changes
  useEffect(() => {
    setName(profile.name);
    setProxyType(profile.proxyType);
    setPort(profile.port);
  }, [profile]);

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
    if (port >= 1 && port <= 65535 && port !== profile.port) {
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
                  onClick={() => handleProxyTypeChange("socks5")}
                >
                  SOCKS5
                </button>
                <button 
                  type="button"
                  className={`selector-btn ${proxyType === "http" ? "active" : ""}`}
                  onClick={() => handleProxyTypeChange("http")}
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
                min="1"
                max="65535"
                className="form-input"
                value={port}
                onChange={handlePortChange}
                onBlur={handlePortBlur}
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
