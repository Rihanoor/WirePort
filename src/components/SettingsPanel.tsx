import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Settings } from "lucide-react";

interface SettingsPanelProps {
  showToast: (msg: string, type: "success" | "error") => void;
}

interface AppSettings {
  wireproxyBinaryPath: string;
  hideDockIcon: boolean;
}

export const SettingsPanel: React.FC<SettingsPanelProps> = ({ showToast }) => {
  const [settings, setSettings] = useState<AppSettings>({
    wireproxyBinaryPath: "",
    hideDockIcon: false,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchSettings = async () => {
      try {
        const res = await invoke<any>("load_settings");
        setSettings({
          wireproxyBinaryPath: res.wireproxyBinaryPath || "",
          hideDockIcon: res.hideDockIcon || false,
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
    const next = { ...settings, hideDockIcon: checked };
    handleSave(next);
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
            <h2 style={{ fontSize: "18px", fontWeight: 700 }}>Application Settings</h2>
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
              <div 
                className="form-group"
                style={{
                  display: "flex",
                  flexDirection: "row",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "16px 0",
                  borderBottom: "1px solid var(--border)",
                  gap: "24px"
                }}
              >
                <div style={{ display: "flex", flexDirection: "column", gap: "4px", flex: 1 }}>
                  <span style={{ fontSize: "14px", fontWeight: 600, color: "var(--text-primary)" }}>
                    Hide Dock Icon (macOS)
                  </span>
                  <span style={{ fontSize: "12px", color: "var(--text-secondary)" }}>
                    Hides the app from the Dock. WirePort will run in the background and can be managed from the menu bar status icon.
                  </span>
                </div>
                
                <label className="switch-toggle" style={{ position: "relative", display: "inline-block", width: "40px", height: "20px" }}>
                  <input 
                    type="checkbox" 
                    checked={settings.hideDockIcon}
                    onChange={(e) => handleToggleHideDock(e.target.checked)}
                    style={{ opacity: 0, width: 0, height: 0 }}
                  />
                  <span 
                    className="slider" 
                    style={{
                      position: "absolute",
                      cursor: "pointer",
                      top: 0,
                      left: 0,
                      right: 0,
                      bottom: 0,
                      backgroundColor: settings.hideDockIcon ? "var(--primary)" : "#2a2f45",
                      transition: ".3s",
                      borderRadius: "20px",
                    }}
                  >
                    <span 
                      style={{
                        position: "absolute",
                        content: '""',
                        height: "14px",
                        width: "14px",
                        left: "3px",
                        bottom: "3px",
                        backgroundColor: "white",
                        transition: ".3s",
                        borderRadius: "50%",
                        transform: settings.hideDockIcon ? "translateX(20px)" : "translateX(0)"
                      }}
                    />
                  </span>
                </label>
              </div>

              {/* WireProxy Custom Binary Path */}
              <div className="form-group" style={{ marginTop: "12px" }}>
                <label className="form-label">Custom WireProxy Binary Path (Optional)</label>
                <div style={{ display: "flex", gap: "10px" }}>
                  <input 
                    type="text" 
                    className="form-input"
                    value={settings.wireproxyBinaryPath}
                    onChange={handleBinaryPathChange}
                    onBlur={handleBinaryPathBlur}
                    placeholder="Leave blank to use bundled wireproxy"
                    style={{ flex: 1 }}
                  />
                </div>
                <span style={{ fontSize: "11px", color: "var(--text-muted)", marginTop: "4px" }}>
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
