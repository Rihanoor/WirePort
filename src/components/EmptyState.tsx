import React from "react";
import { Plus, ArrowRight, ShieldCheck, Cpu, HardDrive, Share2 } from "lucide-react";

interface EmptyStateProps {
  onImportClick: () => void;
}

export const EmptyState: React.FC<EmptyStateProps> = ({ onImportClick }) => {
  return (
    <div className="empty-state-container">
      <div className="empty-state-card">
        {/* Decorative graphic */}
        <div className="empty-state-graphic">
          <div className="pulse-ring ring-1"></div>
          <div className="pulse-ring ring-2"></div>
          <div className="graphic-center">
            <ShieldCheck size={32} className="shield-icon" />
          </div>
        </div>

        {/* Messaging */}
        <h1 className="empty-state-title">No profiles yet</h1>
        <p className="empty-state-description">
          Import a WireGuard config to expose it as a local SOCKS5 or HTTP proxy
          that any app can route through — no root, no system VPN.
        </p>

        {/* Primary Action */}
        <button className="btn btn-primary btn-large btn-center" onClick={onImportClick}>
          <Plus size={18} />
          <span>Import .conf</span>
        </button>

        {/* Workflow Diagram */}
        <div className="workflow-diagram-container">
          <div className="workflow-title">How it works</div>
          <div className="workflow-flow">
            <div className="workflow-node">
              <HardDrive size={18} className="node-icon" />
              <div className="node-label">WireGuard .conf</div>
            </div>
            
            <ArrowRight size={14} className="flow-arrow" />
            
            <div className="workflow-node active">
              <Cpu size={18} className="node-icon active-icon" />
              <div className="node-label">WirePort</div>
            </div>
            
            <ArrowRight size={14} className="flow-arrow" />
            
            <div className="workflow-node">
              <Share2 size={18} className="node-icon" />
              <div className="node-label">Local proxy</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
