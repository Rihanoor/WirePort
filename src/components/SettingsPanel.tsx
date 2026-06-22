import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Settings } from "lucide-react";
import { Toggle } from "./Toggle";

interface SettingsPanelProps {
  showToast: (msg: string, type: "success" | "error") => void;
}

interface AppSettings {
  wireproxyBinaryPath: string;
  hideDockIcon: boolean;
  disableLogs: boolean;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ showToast }) => {
  const [settings, setSettings] = useState<AppSettings>({
    wireproxyBinaryPath: "",
    hideDockIcon: false,
    disableLogs: false,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchSettings = async () => {
      try {
        const res = await invoke<any>("load_settings");
        setSettings({
          wireproxyBinaryPath: res.wireproxyBinaryPath || "",
          hideDockIcon: res.hideDockIcon || false,
          disableLogs: res.disableLogs || false,
        });
      } catch (err) {
        console.error("Failed to load settings:", err);
        showToast("Failed to load settings", "error");
      } finally {
        setLoading(false);
      }
    };
    fetchSettings();
  }, []);

  const handleSave = async (updatedSettings: AppSettings) => {
    try {
      await invoke("save_settings", { settings: updatedSettings });
      setSettings(updatedSettings);
      showToast("Settings saved successfully", "success");
    } catch (err: any) {
      console.error("Failed to save settings:", err);
      showToast(err.toString() || "Failed to save settings", "error");
    }
  };

  const handleToggleHideDock = (checked: boolean) => {
    handleSave({ ...settings, hideDockIcon: checked });
  };

  const handleToggleDisableLogs = (checked: boolean) => {
    handleSave({ ...settings, disableLogs: checked });
  };

  const handleBinaryPathChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSettings((prev) => ({ ...prev, wireproxyBinaryPath: e.target.value }));
  };

  const handleBinaryPathBlur = () => {
    handleSave(settings);
  };

  return (
    <div className="profile-details-container">
      <div className="details-header">
        <div className="details-header-title">
          <div className="details-avatar">
            <Settings size={24} />
          </div>
          <div className="details-name-wrapper">
            <h2 className="settings-title">Application Settings</h2>
            <span className="details-meta-tag">Preferences</span>
          </div>
        </div>
      </div>

      {loading ? (
        <div className="loading-container">
          <span className="loading-text">Loading preferences...</span>
        </div>
      ) : (
        <div className="details-body" style={{ maxWidth: "600px" }}>
          <div className="details-section">
            <h3 className="section-title">General Settings</h3>

            <div className="settings-grid" style={{ gridTemplateColumns: "1fr" }}>
              {/* macOS Dock Icon Preference */}
              <div className="settings-row">
                <div className="settings-row-text">
                  <span className="settings-row-label">Hide Dock Icon (macOS)</span>
                  <span className="settings-row-help">
                    Hides the app from the Dock. WirePort will run in the background and can be managed from the menu bar status icon.
                  </span>
                </div>
                <Toggle
                  id="hide-dock-toggle"
                  checked={settings.hideDockIcon}
                  onChange={handleToggleHideDock}
                  aria-label="Hide Dock Icon"
                />
              </div>

              {/* Disable Runtime Logging Preference */}
              <div className="settings-row">
                <div className="settings-row-text">
                  <span className="settings-row-label">Disable Runtime Logging</span>
                  <span className="settings-row-help">
                    Turns off the console logging of WireProxy stdout and stderr. Can improve performance and reduce memory usage.
                  </span>
                </div>
                <Toggle
                  id="disable-logs-toggle"
                  checked={settings.disableLogs}
                  onChange={handleToggleDisableLogs}
                  aria-label="Disable Runtime Logging"
                />
              </div>

              {/* WireProxy Custom Binary Path */}
              <div className="form-group" style={{ marginTop: "12px" }}>
                <label className="form-label">Custom WireProxy Binary Path (Optional)</label>
                <input
                  type="text"
                  className="form-input"
                  value={settings.wireproxyBinaryPath}
                  onChange={handleBinaryPathChange}
                  onBlur={handleBinaryPathBlur}
                  placeholder="Leave blank to use bundled wireproxy"
                />
                <span className="settings-field-help">
                  Specify the absolute path to a custom wireproxy binary if you want to bypass the bundled sidecar.
                </span>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
