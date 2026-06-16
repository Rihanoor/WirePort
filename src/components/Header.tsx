import React from "react";
import { ShieldAlert, Shield, Globe } from "lucide-react";
import { Profile } from "../types";

interface HeaderProps {
  activeProfile: Profile | null;
}

export const Header: React.FC<HeaderProps> = ({ activeProfile }) => {
  const isOnline = activeProfile !== null;

  return (
    <header className="app-header">
      <div className="header-left">
        <h2 className="page-title">Dashboard</h2>
      </div>
      <div className="header-right">
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
