import { useState, useEffect, useCallback, lazy, Suspense } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ThemeProvider, createTheme, CssBaseline, Box, Tabs, Tab,
  IconButton, Tooltip, Dialog, DialogTitle, DialogContent,
  DialogActions, Button, Typography, LinearProgress, CircularProgress, Collapse,
} from "@mui/material";
import {
  ShieldCheck, Wrench, Info, Sun, Moon, Monitor, Certificate,
  ArrowDown, CaretDown, CaretUp,
} from "@phosphor-icons/react";
import LogPanel from "./components/LogPanel";
import i18n, { LANG_STORAGE_KEY } from "./i18n";

// 按 Tab 懒加载，减小首屏 bundle，切换 Tab 时才加载对应模块
const LicenseTab = lazy(() => import("./components/LicenseTab"));
const HubTab = lazy(() => import("./components/HubTab"));
const EditorTab = lazy(() => import("./components/EditorTab"));
const AboutTab = lazy(() => import("./components/AboutTab"));
// 仅在显示更新日志时按需加载 markdown 渲染器
const ReactMarkdown = lazy(() => import("react-markdown"));

export interface LogEntry {
  time: string;
  level: "info" | "success" | "error" | "warn";
  message: string;
}

type ThemeMode = "system" | "light" | "dark";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function baseThemeOptions() {
  return {
    typography: {
      fontFamily: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
      fontSize: 13,
    },
    shape: { borderRadius: 8 },
    components: {
      MuiTab: { styleOverrides: { root: { minHeight: 36, textTransform: "none", fontSize: 13 } } },
      MuiButton: { styleOverrides: { root: { textTransform: "none" } } },
    },
  };
}

const lightTheme = createTheme({
  ...baseThemeOptions(),
  palette: {
    mode: "light",
    primary: { main: "#1976d2" },
    background: { default: "#f5f5f5", paper: "#ffffff" },
    divider: "#e0e0e0",
  },
});

const darkTheme = createTheme({
  ...baseThemeOptions(),
  palette: {
    mode: "dark",
    primary: { main: "#90caf9" },
    background: { default: "#121212", paper: "#1e1e1e" },
    divider: "#333333",
  },
});

function TabLoadingFallback() {
  return (
    <Box sx={{ display: "flex", justifyContent: "center", alignItems: "center", py: 6 }}>
      <CircularProgress size={28} />
    </Box>
  );
}

export default function App() {
  const { t } = useTranslation();
  const [tab, setTab] = useState(0);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [themeMode, setThemeMode] = useState<ThemeMode>("system");
  const [systemDark, setSystemDark] = useState(getSystemTheme() === "dark");
  const [lang, setLang] = useState<string>(() => (i18n.language.startsWith("zh") ? "zh" : "en"));

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  const effectiveMode = themeMode === "system" ? (systemDark ? "dark" : "light") : themeMode;
  const theme = effectiveMode === "dark" ? darkTheme : lightTheme;

  const cycleTheme = useCallback(() => {
    setThemeMode((prev) => {
      const order: ThemeMode[] = ["system", "light", "dark"];
      return order[(order.indexOf(prev) + 1) % 3];
    });
  }, []);

  const toggleLanguage = useCallback(() => {
    const next = lang === "zh" ? "en" : "zh";
    i18n.changeLanguage(next);
    localStorage.setItem(LANG_STORAGE_KEY, next);
    setLang(next);
  }, [lang]);

  const themeIcon = themeMode === "system"
    ? <Monitor size={16} />
    : themeMode === "light"
      ? <Sun size={16} />
      : <Moon size={16} />;

  const themeLabel = themeMode === "system" ? "System" : themeMode === "light" ? "Light" : "Dark";

  const addLog = useCallback(
    (level: LogEntry["level"], message: string) => {
      setLogs((prev) => [...prev, { time: new Date().toLocaleTimeString(), level, message }]);
    },
    []
  );

  const clearLogs = useCallback(() => setLogs([]), []);

  // Auto-update state
  const [updateInfo, setUpdateInfo] = useState<{ version: string; downloadUrl: string; fileName: string; body: string } | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [changelogOpen, setChangelogOpen] = useState(false);

  // 共用状态 - 立即初始化，不等待后端
  const [licenseStatus, setLicenseStatus] = useState<string>("not_found");
  const [hubStatus, setHubStatus] = useState<string>("unknown");
  const [isAdmin, setIsAdmin] = useState(true); // 默认假设是管理员

  // 异步检查状态，不阻塞UI
  useEffect(() => {
    checkLicenseStatus();
    checkAdminStatus();
    checkHubStatus();
    checkForUpdate();
  }, []);

  const checkForUpdate = useCallback(async () => {
    try {
      const info = await invoke<{ version: string; download_url: string; file_name: string; body: string } | null>("check_update");
      if (info) {
        setUpdateInfo({
          version: info.version,
          downloadUrl: info.download_url,
          fileName: info.file_name,
          body: info.body,
        });
      }
    } catch (e) {
      console.error("Update check failed:", e);
    }
  }, []);

  const handleDownloadUpdate = useCallback(async () => {
    if (!updateInfo) return;
    setDownloading(true);
    setDownloadProgress(0);

    const unlisten = await listen<{ downloaded: number; total: number; percent: number }>("update-progress", (event) => {
      setDownloadProgress(event.payload.percent);
    });

    try {
      await invoke("download_update", {
        downloadUrl: updateInfo.downloadUrl,
        fileName: updateInfo.fileName,
      });
    } catch (e) {
      const msg = String(e);
      if (msg !== "Download cancelled") {
        console.error("Download failed:", e);
      }
    } finally {
      unlisten();
      setDownloading(false);
    }
  }, [updateInfo]);

  const handleCancelDownload = useCallback(async () => {
    try {
      await invoke("cancel_update_download");
    } catch (e) {
      console.error("Cancel failed:", e);
    }
  }, []);

  const checkLicenseStatus = useCallback(async () => {
    try {
      const status = await invoke<string>("check_license_status");
      setLicenseStatus(status);
      return status;
    } catch (e) {
      console.error("License check failed:", e);
      return null;
    }
  }, []);

  const checkHubStatus = useCallback(async () => {
    try {
      const status = await invoke<string>("check_hub_dll_status");
      setHubStatus(status);
      return status;
    } catch (e) {
      console.error("Hub check failed:", e);
      return null;
    }
  }, []);

  const checkAdminStatus = useCallback(async () => {
    try {
      const admin = await invoke<boolean>("check_admin");
      setIsAdmin(admin);
    } catch (e) {
      console.error("Admin check failed:", e);
    }
  }, []);

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Box className="app-container">
        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: 1, borderColor: "divider", px: 1 }}>
          <Tabs value={tab} onChange={(_, v) => setTab(v)}>
            <Tab icon={<Certificate size={16} />} label={t("tabs.license")} iconPosition="start" />
            <Tab icon={<ShieldCheck size={16} />} label={t("tabs.hub")} iconPosition="start" />
            <Tab icon={<Wrench size={16} />} label={t("tabs.editor")} iconPosition="start" />
            <Tab icon={<Info size={16} />} label={t("tabs.about")} iconPosition="start" />
          </Tabs>
          <Box sx={{ display: "flex", alignItems: "center" }}>
            <Tooltip title={lang === "zh" ? "Switch to English" : "切换为中文"}>
              <IconButton
                onClick={toggleLanguage}
                size="small"
                sx={{ mr: 0.5, fontSize: 12, fontWeight: 600, width: 30 }}
              >
                {lang === "zh" ? "中" : "EN"}
              </IconButton>
            </Tooltip>
            <Tooltip title={themeLabel}>
              <IconButton onClick={cycleTheme} size="small" sx={{ mr: 1 }}>
                {themeIcon}
              </IconButton>
            </Tooltip>
          </Box>
        </Box>
        <Box className="tab-body">
          <Suspense fallback={<TabLoadingFallback />}>
            {tab === 0 && <LicenseTab addLog={addLog} licenseStatus={licenseStatus} isAdmin={isAdmin} onRefresh={checkLicenseStatus} />}
            {tab === 1 && <HubTab addLog={addLog} licenseStatus={licenseStatus} isAdmin={isAdmin} hubStatus={hubStatus} onRefresh={checkLicenseStatus} onHubStatusChange={checkHubStatus} />}
            {tab === 2 && <EditorTab addLog={addLog} hubStatus={hubStatus} />}
            {tab === 3 && <AboutTab />}
          </Suspense>
        </Box>
        <LogPanel logs={logs} clearLogs={clearLogs} />
      </Box>

      <Dialog open={!!updateInfo} maxWidth="sm" fullWidth>
        <DialogTitle sx={{ display: "flex", alignItems: "center", gap: 1 }}>
          <ArrowDown size={20} />
          {t("update.available")} v{updateInfo?.version}
        </DialogTitle>
        <DialogContent>
          {downloading ? (
            <Box sx={{ mt: 1 }}>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 1 }}>
                {t("update.downloading")}
              </Typography>
              <LinearProgress variant="determinate" value={downloadProgress} />
              <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5, display: "block" }}>
                {Math.round(downloadProgress)}%
              </Typography>
            </Box>
          ) : (
            <Typography variant="body2" color="text.secondary">
              {t("update.install_hint")}
            </Typography>
          )}

          {updateInfo?.body && (
            <Box sx={{ mt: 1.5, borderTop: 1, borderColor: "divider", pt: 1 }}>
              <Box
                onClick={() => setChangelogOpen((v) => !v)}
                sx={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  cursor: "pointer",
                  userSelect: "none",
                }}
              >
                <Typography variant="subtitle2">{t("update.changelog")}</Typography>
                <IconButton size="small" sx={{ p: 0.25 }}>
                  {changelogOpen ? <CaretUp size={16} /> : <CaretDown size={16} />}
                </IconButton>
              </Box>
              <Collapse in={changelogOpen}>
                <Box
                  sx={{
                    mt: 1,
                    maxHeight: 240,
                    overflowY: "auto",
                    fontSize: 13,
                    color: "text.secondary",
                    "& h1, & h2, & h3": { fontSize: 14, fontWeight: 600, margin: "8px 0 4px", color: "text.primary" },
                    "& p": { margin: "4px 0" },
                    "& ul, & ol": { margin: "4px 0", paddingLeft: 20 },
                    "& li": { margin: "2px 0" },
                    "& a": { color: "primary.main" },
                    "& code": { fontFamily: "monospace", fontSize: 12 },
                  }}
                >
                  <Suspense fallback={<Typography variant="body2" color="text.secondary">{updateInfo.body}</Typography>}>
                    <ReactMarkdown>{updateInfo.body}</ReactMarkdown>
                  </Suspense>
                </Box>
              </Collapse>
            </Box>
          )}
        </DialogContent>
        <DialogActions>
          {downloading ? (
            <Button onClick={handleCancelDownload} color="error">{t("update.cancel")}</Button>
          ) : (
            <>
              <Button onClick={() => setUpdateInfo(null)}>{t("update.skip")}</Button>
              <Button variant="contained" onClick={handleDownloadUpdate}>
                {t("update.download")}
              </Button>
            </>
          )}
        </DialogActions>
      </Dialog>
    </ThemeProvider>
  );
}
