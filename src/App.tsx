import { useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { ConfigProvider, theme } from "antd";
import zhCN from "antd/locale/zh_CN";
import ErrorBoundary from "./components/ErrorBoundary";
import MainLayout from "./components/Layout/MainLayout";
import Dashboard from "./pages/Dashboard";
import Library from "./pages/Library";
import Statistics from "./pages/Statistics";
import Tags from "./pages/Tags";
import Settings from "./pages/Settings";
import { useSettingsStore } from "./stores/useSettingsStore";
import { useTagStore } from "./stores/useTagStore";

export default function App() {
  const loadSettings = useSettingsStore((s) => s.load);
  const loadTags = useTagStore((s) => s.loadTags);
  const darkMode = useSettingsStore((s) => s.settings.theme === "dark");

  useEffect(() => {
    loadSettings();
    loadTags();
  }, [loadSettings, loadTags]);

  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: darkMode ? theme.darkAlgorithm : theme.defaultAlgorithm,
        token: { colorPrimary: "#4a7dff" },
      }}
    >
      <ErrorBoundary>
        <HashRouter>
          <Routes>
            <Route path="/" element={<MainLayout />}>
              <Route index element={<Navigate to="/dashboard" replace />} />
              <Route path="dashboard" element={<Dashboard />} />
              <Route path="library" element={<Library />} />
              <Route path="statistics" element={<Statistics />} />
              <Route path="tags" element={<Tags />} />
              <Route path="settings" element={<Settings />} />
            </Route>
          </Routes>
        </HashRouter>
      </ErrorBoundary>
    </ConfigProvider>
  );
}
