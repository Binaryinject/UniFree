import { useState, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import {
  ThemeProvider, createTheme, CssBaseline, Box, Tabs, Tab,
  IconButton, Tooltip,
} from "@mui/material";
import {
  ShieldCheck, Wrench, Info, Sun, Moon, Monitor,
} from "@phosphor-icons/react";
import HubTab from "./components/HubTab";
import EditorTab from "./components/EditorTab";
import AboutTab from "./components/AboutTab";
import LogPanel from "./components/LogPanel";

export interface LogEntry {
  time: string;
  level: "info" | "success" | "error" | "warn";
  message: string;
}

type ThemeMode = "system" | "light" | "dark";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

const lightTheme = createTheme({
  palette: {
    mode: "light",
    primary: { main: "#1976d2" },
    background: { default: "#f5f5f5", paper: "#ffffff" },
    divider: "#e0e0e0",
  },
  typography: {
    fontFamily: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    fontSize: 13,
  },
  shape: { borderRadius: 8 },
  components: {
    MuiTab: { styleOverrides: { root: { minHeight: 36, textTransform: "none", fontSize: 13 } } },
    MuiButton: { styleOverrides: { root: { textTransform: "none" } } },
  },
});

const darkTheme = createTheme({
  palette: {
    mode: "dark",
    primary: { main: "#90caf9" },
    background: { default: "#121212", paper: "#1e1e1e" },
    divider: "#333333",
  },
  typography: {
    fontFamily: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    fontSize: 13,
  },
  shape: { borderRadius: 8 },
  components: {
    MuiTab: { styleOverrides: { root: { minHeight: 36, textTransform: "none", fontSize: 13 } } },
    MuiButton: { styleOverrides: { root: { textTransform: "none" } } },
  },
});

export default function App() {
  const { t } = useTranslation();
  const [tab, setTab] = useState(0);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [themeMode, setThemeMode] = useState<ThemeMode>("system");
  const [systemDark, setSystemDark] = useState(getSystemTheme() === "dark");

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

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Box className="app-container">
        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", borderBottom: 1, borderColor: "divider", px: 1 }}>
          <Tabs value={tab} onChange={(_, v) => setTab(v)}>
            <Tab icon={<ShieldCheck size={16} />} label={t("tabs.hub")} iconPosition="start" />
            <Tab icon={<Wrench size={16} />} label={t("tabs.editor")} iconPosition="start" />
            <Tab icon={<Info size={16} />} label={t("tabs.about")} iconPosition="start" />
          </Tabs>
          <Tooltip title={themeLabel}>
            <IconButton onClick={cycleTheme} size="small" sx={{ mr: 1 }}>
              {themeIcon}
            </IconButton>
          </Tooltip>
        </Box>
        <Box className="tab-body">
          {tab === 0 && <HubTab addLog={addLog} />}
          {tab === 1 && <EditorTab addLog={addLog} />}
          {tab === 2 && <AboutTab />}
        </Box>
        <LogPanel logs={logs} clearLogs={clearLogs} />
      </Box>
    </ThemeProvider>
  );
}
