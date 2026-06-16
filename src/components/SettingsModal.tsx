import React, { useState } from "react";
import { X, FolderOpen, ChevronDown } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { AppSettings } from "../types";

interface SettingsModalProps {
  settings: AppSettings;
  onSave: (settings: AppSettings) => void;
  onClose: () => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({
  settings,
  onSave,
  onClose,
}) => {
  const [binaryPath, setBinaryPath] = useState(settings.wireproxyBinaryPath || "");
  const [isSaving, setIsSaving] = useState(false);
  const [showAdvanced, setShowAdvanced] = useState(!!settings.wireproxyBinaryPath);

  const handleBrowse = async () => {
    try {
      const result = await invoke<string | null>("pick_wireproxy_binary");
      if (result) {
        setBinaryPath(result);
      }
    } catch (err) {
      console.error("Failed to pick wireproxy binary:", err);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSaving(true);
    try {
      const updatedSettings = { wireproxyBinaryPath: binaryPath.trim() };
      await invoke("save_settings", { settings: updatedSettings });
      onSave(updatedSettings);
      onClose();
    } catch (err) {
      console.error("Failed to save settings:", err);
      alert("Failed to save settings to disk");
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-card" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2 className="modal-title">Settings</h2>
          <button className="modal-close-btn" onClick={onClose} aria-label="Close settings">
            <X size={18} />
          </button>
        </div>

        <form onSubmit={handleSave}>
          <div className="modal-body">
            <div className="settings-info-card">
              <span className="settings-info-badge">Default</span>
              <p className="settings-info-text">
                WirePort uses the bundled WireProxy sidecar by default to manage your connections. No setup is required.
              </p>
            </div>

            <div className="advanced-settings-toggle-wrapper">
              <button
                type="button"
                className="btn-toggle-advanced"
                onClick={() => setShowAdvanced(!showAdvanced)}
              >
                <span>{showAdvanced ? "Hide Advanced Settings" : "Show Advanced Settings"}</span>
                <ChevronDown
                  size={16}
                  className={`chevron-icon ${showAdvanced ? "expanded" : ""}`}
                />
              </button>
            </div>

            {showAdvanced && (
              <div className="advanced-settings-content">
                <div className="form-group">
                  <div className="form-label-row">
                    <label htmlFor="binary-path" className="form-label">
                      Custom WireProxy Binary Path
                    </label>
                    {binaryPath && (
                      <button
                        type="button"
                        className="btn-reset-binary"
                        onClick={() => setBinaryPath("")}
                      >
                        Reset to bundled binary
                      </button>
                    )}
                  </div>
                  <div className="input-with-browse">
                    <input
                      id="binary-path"
                      type="text"
                      placeholder="e.g. /usr/local/bin/wireproxy"
                      className="form-input"
                      value={binaryPath}
                      onChange={(e) => setBinaryPath(e.target.value)}
                    />
                    <button
                      type="button"
                      className="btn btn-secondary btn-browse"
                      onClick={handleBrowse}
                      title="Browse local files"
                    >
                      <FolderOpen size={16} />
                      <span>Browse</span>
                    </button>
                  </div>
                  <span className="form-help-text">
                    Specify the absolute path to a custom WireProxy executable binary to override the bundled version.
                  </span>
                </div>
              </div>
            )}
          </div>

          <div className="modal-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={onClose}
              disabled={isSaving}
            >
              Cancel
            </button>
            <button type="submit" className="btn btn-primary" disabled={isSaving}>
              {isSaving ? "Saving..." : "Save Settings"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
