import React from "react";
import { ShieldAlert, Shield, Globe } from "lucide-react";
import { Profile } from "../types";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface HeaderProps {
  activeProfile: Profile | null;
}

export const Header: React.FC<HeaderProps> = ({ activeProfile }) => {
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
      <div className="header-top-row">
        <h2 className="page-title">Dashboard</h2>
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
