import React from "react";
import { 
  Shield, 
  Plus, 
  ArrowRight, 
  Cpu, 
  Activity, 
  FileText, 
  Network, 
  CheckCircle,
  Clock
} from "lucide-react";
import { Profile } from "../types";

interface DashboardProps {
  profiles: Profile[];
  onProfileSelect: (id: string | null) => void;
  onImportClick: () => void;
}

export const Dashboard: React.FC<DashboardProps> = ({
  profiles,
  onProfileSelect,
  onImportClick
}) => {
  // Counters
  const totalProfiles = profiles.length;
  const runningProfiles = profiles.filter(p => p.status === "running").length;
  const socks5Profiles = profiles.filter(p => p.proxyType === "socks5").length;
  const httpProfiles = profiles.filter(p => p.proxyType === "http").length;

  // Recent profiles ordered by lastConnectedAt descending
  const recentProfiles = [...profiles]
    .sort((a, b) => {
      const aTime = a.lastConnectedAt ? new Date(a.lastConnectedAt).getTime() : 0;
      const bTime = b.lastConnectedAt ? new Date(b.lastConnectedAt).getTime() : 0;
      // Fallback to updatedAt if lastConnectedAt is not available
      if (aTime === 0 && bTime === 0) {
        return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
      }
      return bTime - aTime;
    })
    .slice(0, 5); // Display top 5

  const formatRelativeTime = (isoString?: string): string => {
    if (!isoString) return "Never";
    const date = new Date(isoString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSecs = Math.floor(diffMs / 1000);
    
    if (diffSecs < 1) return "Just now";
    if (diffSecs < 60) return `${diffSecs}s ago`;
    
    const diffMins = Math.floor(diffSecs / 60);
    if (diffMins < 60) return `${diffMins}m ago`;
    
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    
    return date.toLocaleDateString(undefined, { 
      month: "short", 
      day: "numeric", 
      hour: "2-digit", 
      minute: "2-digit" 
    });
  };

  return (
    <div className="dashboard-container">
      {/* Top Banner / Hero */}
      <div className="dashboard-hero">
        <div className="hero-content">
          <h1 className="hero-title">Overview</h1>
          <p className="hero-subtitle">
            Manage WireGuard tunnels and the local proxy endpoints they expose.
          </p>
        </div>
      </div>

      {/* Metrics Grid */}
      <div className="dashboard-stats-grid">
        <div className="dashboard-stat-card">
          <div className="stat-card-header">
            <span className="stat-label">Profiles</span>
            <Shield size={18} className="stat-icon" />
          </div>
          <span className="stat-value">{totalProfiles}</span>
        </div>

        <div className="dashboard-stat-card">
          <div className="stat-card-header">
            <span className="stat-label">Active</span>
            <span className={`status-indicator ${runningProfiles > 0 ? "running" : "stopped"}`} />
          </div>
          <span className="stat-value text-primary">{runningProfiles}</span>
        </div>

        <div className="dashboard-stat-card">
          <div className="stat-card-header">
            <span className="stat-label">SOCKS5</span>
            <Network size={18} className="stat-icon" />
          </div>
          <span className="stat-value">{socks5Profiles}</span>
        </div>

        <div className="dashboard-stat-card">
          <div className="stat-card-header">
            <span className="stat-label">HTTP</span>
            <Network size={18} className="stat-icon" />
          </div>
          <span className="stat-value">{httpProfiles}</span>
        </div>
      </div>

      {/* Two Column Layout */}
      <div className="dashboard-layout">
        {/* Left Column: Recent Profiles */}
        <div className="dashboard-col-left">
          <div className="dashboard-section-header">
            <h2 className="dashboard-section-title">Recent</h2>
          </div>

          <div className="recent-profiles-list">
            {recentProfiles.length === 0 ? (
              <div className="recent-empty">No profiles used yet.</div>
            ) : (
              recentProfiles.map(profile => (
                <div 
                  key={profile.id}
                  className="recent-profile-card"
                  onClick={() => onProfileSelect(profile.id)}
                >
                  <div className="recent-card-left">
                    <span className={`status-indicator-dot ${profile.status}`} />
                    <div className="recent-profile-info">
                      <span className="recent-profile-name">{profile.name}</span>
                      <span className="recent-profile-endpoint">{profile.endpoint}</span>
                    </div>
                  </div>

                  <div className="recent-card-right">
                    <span className="recent-profile-type-badge">
                      {profile.proxyType.toUpperCase()}:{profile.port}
                    </span>
                    <span className="recent-profile-time">
                      <Clock size={12} className="time-icon" />
                      {formatRelativeTime(profile.lastConnectedAt)}
                    </span>
                    <ArrowRight size={14} className="hover-arrow" />
                  </div>
                </div>
              ))
            )}
          </div>
        </div>

        {/* Right Column: Status & Quick Actions */}
        <div className="dashboard-col-right">
          {/* System Status Card */}
          <div className="status-system-card">
            <h3 className="card-sub-title">Status</h3>
            <div className="status-rows">
              <div className="status-row-item">
                <div className="status-row-left">
                  <Cpu size={16} className="status-row-icon" />
                  <span>WireProxy engine</span>
                </div>
                <span className="status-row-badge active">
                  <CheckCircle size={10} /> Ready
                </span>
              </div>

              <div className="status-row-item">
                <div className="status-row-left">
                  <Activity size={16} className="status-row-icon" />
                  <span>Health checks</span>
                </div>
                <span className="status-row-badge active">
                  <CheckCircle size={10} /> On
                </span>
              </div>

              <div className="status-row-item">
                <div className="status-row-left">
                  <FileText size={16} className="status-row-icon" />
                  <span>Runtime logs</span>
                </div>
                <span className="status-row-badge active">
                  <CheckCircle size={10} /> On
                </span>
              </div>

              <div className="status-row-item">
                <div className="status-row-left">
                  <Activity size={16} className="status-row-icon" />
                  <span>Live statistics</span>
                </div>
                <span className="status-row-badge active">
                  <CheckCircle size={10} /> On
                </span>
              </div>
            </div>
          </div>

          {/* Quick Actions */}
          <div className="quick-actions-card">
            <h3 className="card-sub-title">Quick actions</h3>
            <div className="quick-actions-buttons">
              <button className="btn btn-primary btn-full btn-action-dashboard" onClick={onImportClick}>
                <Plus size={16} />
                <span>Import .conf</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
