import { useState } from "react";
import { Sidebar } from "./components/Sidebar";
import { Header } from "./components/Header";
import { EmptyState } from "./components/EmptyState";
import { Profile } from "./types";
import "./App.css";

function App() {
  const [profiles] = useState<Profile[]>([]);

  const handleImportProfile = () => {
    console.log("Import Profile clicked");
  };

  const handleOpenSettings = () => {
    console.log("Settings clicked");
  };

  return (
    <div className="app-container">
      <Sidebar
        profiles={profiles}
        onImportClick={handleImportProfile}
        onSettingsClick={handleOpenSettings}
      />
      <main className="app-main">
        <Header />
        <EmptyState onImportClick={handleImportProfile} />
      </main>
    </div>
  );
}

export default App;
