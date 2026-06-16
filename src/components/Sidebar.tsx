import React from "react";
import { Plus, Settings, Shield, Info } from "lucide-react";
import { Profile } from "../types";

interface SidebarProps {
  profiles: Profile[];
  selectedProfileId: string | null;
  onProfileSelect: (id: string) => void;
  onImportClick: () => void;
  onSettingsClick: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  profiles,
  selectedProfileId,
  onProfileSelect,
  onImportClick,
  onSettingsClick,
}) => {
  return (
    <aside className="app-sidebar">
      {/* Brand Header */}
      <div className="sidebar-brand">
        <div className="brand-logo">
          <Shield size={20} className="brand-icon" />
        </div>
        <div className="brand-info">
          <span className="brand-name">WirePort</span>
          <span className="brand-tag">v0.1.0</span>
        </div>
      </div>

      {/* Primary Action */}
      <div className="sidebar-actions">
        <button className="btn btn-primary btn-full" onClick={onImportClick}>
          <Plus size={16} />
          <span>Import Profile</span>
        </button>
      </div>

      {/* Profiles List */}
      <div className="sidebar-nav">
        <div className="nav-section-title">WireGuard Profiles</div>
        {profiles.length === 0 ? (
          <div className="sidebar-empty-state">
            <Info size={14} />
            <span>No profiles imported yet</span>
          </div>
        ) : (
          <ul className="profile-list">
            {profiles.map((profile) => (
              <li 
                key={profile.id} 
                className={`profile-item ${profile.id === selectedProfileId ? "active" : ""}`}
                onClick={() => onProfileSelect(profile.id)}
              >
                <span className={`status-indicator ${profile.status}`} />
                <span className="profile-name">{profile.name}</span>
                <span className="profile-port">:{profile.port}</span>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Sidebar Footer */}
      <div className="sidebar-footer">
        <div className="footer-status">
          <div className="status-dot disconnected" />
          <span className="status-text">Engine: Offline</span>
        </div>
        <button 
          className="btn-icon settings-btn" 
          onClick={onSettingsClick}
          aria-label="Settings"
          title="Settings"
        >
          <Settings size={18} />
        </button>
      </div>
    </aside>
  );
};
