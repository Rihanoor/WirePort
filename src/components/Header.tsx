import React from "react";
import { ShieldAlert, Shield, Globe, Square } from "lucide-react";
import { Profile } from "../types";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface HeaderProps {
  activeProfile: Profile | null;
  onStopClick?: () => void;
}

export const Header: React.FC<HeaderProps> = ({ activeProfile, onStopClick }) => {
  const isOnline = activeProfile !== null;

  const handleDoubleClick = async (e: React.MouseEvent) => {
    const target = e.target as HTMLElement;
    if (
      target.closest(".status-pill") ||
      target.closest(".network-pill") ||
      target.closest("button")
    ) {
      return;
    }

    try {
      const appWindow = getCurrentWindow();
      const isMaximized = await appWindow.isMaximized();
      if (isMaximized) {
        await appWindow.unmaximize();
      } else {
        await appWindow.maximize();
      }
    } catch (err) {
      console.error("Failed to toggle maximize:", err);
    }
  };

  return (
    <header className="app-header" onDoubleClick={handleDoubleClick}>
      <div className="header-top-row" style={{ display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%" }}>
        <h2 className="page-title">Dashboard</h2>
        {isOnline && onStopClick && (
          <button 
            onClick={onStopClick}
            className="btn btn-danger btn-sm"
            style={{ 
              padding: "6px 14px", 
              fontSize: "12px", 
              fontWeight: 600, 
              borderRadius: "8px",
              cursor: "pointer",
              transition: "all 0.15s ease",
              display: "flex",
              alignItems: "center",
              gap: "6px"
            }}
            title="Quick Stop active proxy tunnel"
          >
            <Square size={12} fill="currentColor" />
            <span>Stop</span>
          </button>
        )}
      </div>
      <div className="header-bottom-row">
        {isOnline ? (
          <div className="status-pill connected">
            <Shield size={14} />
            <span>Secured Tunnel Online ({activeProfile.name})</span>
          </div>
        ) : (
          <div className="status-pill disconnected">
            <ShieldAlert size={14} />
            <span>Secured Tunnel Offline</span>
          </div>
        )}
        <div className="network-pill">
          <Globe size={14} />
          <span>
            {isOnline
              ? `${activeProfile.proxyType}://127.0.0.1:${activeProfile.port}`
              : "127.0.0.1"}
          </span>
        </div>
      </div>
    </header>
  );
};
