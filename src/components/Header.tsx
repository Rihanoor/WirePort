import React from "react";
import { ShieldAlert, Globe } from "lucide-react";

export const Header: React.FC = () => {
  return (
    <header className="app-header">
      <div className="header-left">
        <h2 className="page-title">Dashboard</h2>
      </div>
      <div className="header-right">
        <div className="status-pill disconnected">
          <ShieldAlert size={14} />
          <span>Secured Tunnel Offline</span>
        </div>
        <div className="network-pill">
          <Globe size={14} />
          <span>127.0.0.1</span>
        </div>
      </div>
    </header>
  );
};
