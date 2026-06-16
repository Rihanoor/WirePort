import React, { useState } from "react";
import { X, FolderOpen } from "lucide-react";
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
            <div className="form-group">
              <label htmlFor="binary-path" className="form-label">
                WireProxy Binary Path
              </label>
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
                Specify the absolute path to the local WireProxy executable binary.
              </span>
            </div>
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
