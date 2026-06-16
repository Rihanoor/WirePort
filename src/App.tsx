import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sidebar } from "./components/Sidebar";
import { Header } from "./components/Header";
import { EmptyState } from "./components/EmptyState";
import { ProfileDetails } from "./components/ProfileDetails";
import { SettingsModal } from "./components/SettingsModal";
import { Profile, AppSettings, ProfileStatus } from "./types";
import { CheckCircle2, AlertCircle } from "lucide-react";
import "./App.css";

interface Toast {
  id: string;
  message: string;
  type: "success" | "error";
}

function App() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [loading, setLoading] = useState(true);
  const [settings, setSettings] = useState<AppSettings>({ wireproxyBinaryPath: "" });
  const [showSettings, setShowSettings] = useState(false);

  const selectProfile = (id: string | null) => {
    setSelectedProfileId(id);
    if (id) {
      localStorage.setItem("lastUsedProfileId", id);
    } else {
      localStorage.removeItem("lastUsedProfileId");
    }
  };

  // Load profiles and settings from local storage on mount
  useEffect(() => {
    const fetchProfilesAndSettings = async () => {
      try {
        const res = await invoke<string>("load_profiles");
        const loaded: Profile[] = JSON.parse(res);
        // Force stopped status on launch for all profiles
        const stoppedProfiles = loaded.map(p => ({ ...p, status: "stopped" as const }));
        setProfiles(stoppedProfiles);

        // Restore last used profile
        const lastUsedId = localStorage.getItem("lastUsedProfileId");
        if (lastUsedId && stoppedProfiles.some(p => p.id === lastUsedId)) {
          setSelectedProfileId(lastUsedId);
        } else if (stoppedProfiles.length > 0) {
          setSelectedProfileId(stoppedProfiles[0].id);
        }
      } catch (err) {
        console.error("Failed to load profiles:", err);
        showToast("Failed to load profiles from storage", "error");
      }

      try {
        const loadedSettings = await invoke<AppSettings>("load_settings");
        setSettings(loadedSettings);
      } catch (err) {
        console.error("Failed to load settings:", err);
      } finally {
        setLoading(false);
      }
    };
    fetchProfilesAndSettings();
  }, []);

  const showToast = (message: string, type: "success" | "error" = "success") => {
    const id = Math.random().toString(36).substring(2, 9);
    setToasts((prev) => [...prev, { id, message, type }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 4000);
  };

  const getNextAvailablePort = (existingProfiles: Profile[]): number => {
    let port = 1080;
    const usedPorts = new Set(existingProfiles.map((p) => p.port));
    while (usedPorts.has(port)) {
      port++;
    }
    return port;
  };

  const handleImportProfile = async () => {
    try {
      const parsedData = await invoke<{
        name: string;
        endpoint: string;
        dns: string;
        address: string;
        allowed_ips: string;
        source_path: string;
        config_content: string;
      } | null>("pick_parse_and_validate_file");

      if (!parsedData) {
        // User cancelled the file dialog
        return;
      }

      // Check for duplicate profile names (e.g. if name already exists, append unique suffix)
      let profileName = parsedData.name;
      const isDuplicate = profiles.some((p) => p.name.toLowerCase() === profileName.toLowerCase());
      if (isDuplicate) {
        profileName = `${profileName} (${Date.now().toString().slice(-4)})`;
      }

      const newProfile: Profile = {
        id: crypto.randomUUID(),
        name: profileName,
        proxyType: "socks5",
        port: getNextAvailablePort(profiles),
        endpoint: parsedData.endpoint,
        dns: parsedData.dns,
        address: parsedData.address,
        allowedIps: parsedData.allowed_ips,
        sourcePath: parsedData.source_path || undefined,
        configContent: parsedData.config_content,
        status: "stopped",
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString()
      };

      const updatedProfiles = [...profiles, newProfile];
      // Force status to "stopped" when persisting to disk
      const profilesToSave = updatedProfiles.map(p => ({ ...p, status: "stopped" as const }));
      await invoke("save_profiles", { profilesJson: JSON.stringify(profilesToSave) });
      setProfiles(updatedProfiles);
      
      // Auto-generate the initial WireProxy config once on import
      try {
        await invoke("generate_wireproxy_config", {
          profileId: newProfile.id,
          proxyType: newProfile.proxyType,
          port: newProfile.port,
          configContent: newProfile.configContent,
        });
      } catch (genErr) {
        console.error("Failed to generate initial config:", genErr);
      }

      selectProfile(newProfile.id);
      showToast(`Imported profile "${profileName}"`, "success");
    } catch (err: any) {
      console.error("Import error:", err);
      showToast(err.toString() || "Invalid WireGuard configuration", "error");
    }
  };

  const handleUpdateProfile = async (updatedProfile: Profile) => {
    // Validate port range (1024-65535)
    if (updatedProfile.port < 1024 || updatedProfile.port > 65535) {
      showToast("Port must be between 1024 and 65535", "error");
      return;
    }

    // Check for port conflicts (exclude the current profile)
    const portConflict = profiles.some(
      (p) => p.id !== updatedProfile.id && p.port === updatedProfile.port
    );
    if (portConflict) {
      showToast(`Port ${updatedProfile.port} is already in use by another profile.`, "error");
      return;
    }

    // Check for name conflicts (exclude the current profile)
    const nameConflict = profiles.some(
      (p) => p.id !== updatedProfile.id && p.name.toLowerCase() === updatedProfile.name.toLowerCase()
    );
    if (nameConflict) {
      showToast(`A profile named "${updatedProfile.name}" already exists.`, "error");
      return;
    }

    const updatedProfiles = profiles.map((p) => p.id === updatedProfile.id ? updatedProfile : p);
    try {
      // Force status to "stopped" when persisting to disk
      const profilesToSave = updatedProfiles.map(p => ({ ...p, status: "stopped" as const }));
      await invoke("save_profiles", { profilesJson: JSON.stringify(profilesToSave) });
      setProfiles(updatedProfiles);
      showToast("Profile saved successfully", "success");
    } catch (err: any) {
      console.error("Save error:", err);
      showToast("Failed to save profile changes", "error");
    }
  };

  const handleDeleteProfile = async (id: string) => {
    const updatedProfiles = profiles.filter((p) => p.id !== id);
    try {
      // Force status to "stopped" when persisting to disk
      const profilesToSave = updatedProfiles.map(p => ({ ...p, status: "stopped" as const }));
      await invoke("save_profiles", { profilesJson: JSON.stringify(profilesToSave) });
      setProfiles(updatedProfiles);
      if (selectedProfileId === id) {
        selectProfile(null);
      }
      showToast("Profile deleted", "success");
    } catch (err: any) {
      console.error("Delete error:", err);
      showToast("Failed to delete profile", "error");
    }
  };

  const handleOpenSettings = () => {
    setShowSettings(true);
  };

  const handleStatusChange = (profileId: string, status: ProfileStatus) => {
    setProfiles((prev) => prev.map((p) => (p.id === profileId ? { ...p, status } : p)));
  };

  const selectedProfile = profiles.find((p) => p.id === selectedProfileId);
  const runningProfile = profiles.find((p) => p.status === "running");

  return (
    <div className="app-container">
      <Sidebar
        profiles={profiles}
        selectedProfileId={selectedProfileId}
        onProfileSelect={selectProfile}
        onImportClick={handleImportProfile}
        onSettingsClick={handleOpenSettings}
      />
      <main className="app-main">
        <Header activeProfile={runningProfile || null} />
        {loading ? (
          <div className="loading-container">
            <span className="loading-text">Loading Profiles...</span>
          </div>
        ) : selectedProfile ? (
          <ProfileDetails
            profile={selectedProfile}
            onUpdate={handleUpdateProfile}
            onDelete={handleDeleteProfile}
            wireproxyBinaryPath={settings.wireproxyBinaryPath}
            onStatusChange={(status) => handleStatusChange(selectedProfile.id, status)}
          />
        ) : (
          <EmptyState onImportClick={handleImportProfile} />
        )}
      </main>

      {/* Toast Notification Container */}
      <div className="toast-container">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast ${toast.type}`}>
            <span className="toast-icon">
              {toast.type === "success" ? <CheckCircle2 size={16} /> : <AlertCircle size={16} />}
            </span>
            <span className="toast-message">{toast.message}</span>
            <button 
              className="toast-close" 
              onClick={() => setToasts((prev) => prev.filter((t) => t.id !== toast.id))}
            >
              &times;
            </button>
          </div>
        ))}
      </div>

      {showSettings && (
        <SettingsModal
          settings={settings}
          onSave={setSettings}
          onClose={() => setShowSettings(false)}
        />
      )}
    </div>
  );
}

export default App;
