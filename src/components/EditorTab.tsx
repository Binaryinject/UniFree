import { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Box, Button, Chip, CircularProgress, Typography, Paper, Alert, Link, IconButton, Divider, Tooltip,
} from "@mui/material";
import { Wrench, Warning, DownloadSimple, FolderOpen, Plus, Trash } from "@phosphor-icons/react";
import type { LogEntry } from "../App";

interface EditorInfo {
  version: string;
  path: string;
  dll_path: string;
  dll_status: string;
  product_name: string;
  architecture: string;
}

interface Props {
  addLog: (level: LogEntry["level"], message: string) => void;
}

// 缓存扫描结果，避免重复扫描
let editorScanCache: { editors: EditorInfo[]; hubPatched: boolean } | null = null;

export function clearEditorScanCache() {
  editorScanCache = null;
}

export default function EditorTab({ addLog }: Props) {
  const { t } = useTranslation();
  const [editors, setEditors] = useState<EditorInfo[]>(editorScanCache?.editors ?? []);
  const [hubPatched, setHubPatched] = useState(editorScanCache?.hubPatched ?? true);
  const [scanning, setScanning] = useState(!editorScanCache);
  const [busyPath, setBusyPath] = useState<string | null>(null);
  const [batchLoading, setBatchLoading] = useState(false);
  const [customPaths, setCustomPaths] = useState<string[]>([]);
  const hasScanned = useRef(editorScanCache !== null);

  useEffect(() => {
    if (!hasScanned.current) {
      scanEditors();
    }
    loadCustomPaths();
  }, []);

  async function scanEditors() {
    hasScanned.current = true;
    setScanning(true);
    try {
      const [list, hub] = await Promise.all([
        invoke<EditorInfo[]>("scan_unity_editors"),
        invoke<string>("check_hub_dll_status"),
      ]);
      setEditors(list);
      const patched = hub === "patched" || hub === "patched_no_backup" || hub === "partial";
      setHubPatched(patched);
      editorScanCache = { editors: list, hubPatched: patched };
    } catch (e) {
      addLog("error", `${t("log.scan_failed")}: ${e}`);
    }
    setScanning(false);
  }

  async function loadCustomPaths() {
    try {
      const paths = await invoke<string[]>("get_editor_scan_paths");
      setCustomPaths(paths);
    } catch { /* ignore */ }
  }

  async function handleAddPath() {
    try {
      const path = await invoke<string>("add_editor_scan_path");
      setCustomPaths((prev) => [...prev, path]);
      addLog("success", `Added scan directory: ${path}`);
      editorScanCache = null;
      await scanEditors();
    } catch (e) {
      addLog("error", `${e}`);
    }
  }

  async function handleRemovePath(path: string) {
    try {
      await invoke("remove_editor_scan_path", { path });
      setCustomPaths((prev) => prev.filter((p) => p !== path));
      addLog("info", `Removed scan directory: ${path}`);
      editorScanCache = null;
      await scanEditors();
    } catch (e) {
      addLog("error", `${e}`);
    }
  }

  function statusChip(status: string) {
    const map: Record<string, { color: "success" | "info" | "default" | "warning"; label: string }> = {
      patched: { color: "success", label: t("status.patched") },
      original: { color: "info", label: t("status.original") },
      not_found: { color: "default", label: t("status.not_found") },
      unknown: { color: "default", label: t("status.unknown") },
      patched_no_backup: { color: "warning", label: t("status.patched_no_backup") },
    };
    const s = map[status] ?? map.unknown;
    return <Chip size="small" color={s.color} label={s.label} variant="outlined" sx={{ minWidth: 64 }} />;
  }

  function logLicenseResult(result: string) {
    if (result.startsWith("skipped_missing_signature:")) {
      addLog("warn", t("log.license_skipped_missing_signature"));
    } else if (result.startsWith("preserved_signed:")) {
      addLog("success", t("log.license_preserved"));
    } else {
      addLog("success", t("log.license_copied"));
    }
  }

  async function handleSingle(editor: EditorInfo, action: "patch" | "restore") {
    if (!hubPatched && action === "patch") {
      addLog("warn", t("editor.hub_first"));
      return;
    }
    setBusyPath(editor.dll_path);
    try {
      if (action === "patch") {
        const running = await invoke<boolean>("check_process", { name: "Unity.exe" });
        if (running) {
          addLog("warn", t("log.editor_running"));
          await invoke("kill_process", { name: "Unity.exe" });
          await new Promise((r) => setTimeout(r, 1000));
        }
        await invoke("patch_editor_dll", { dllPath: editor.dll_path });
        addLog("success", `[${editor.version}] ${t("editor.patch")} ✓`);
        try {
          const result = await invoke<string>("copy_license");
          logLicenseResult(result);
        } catch (le) {
          addLog("error", `${t("log.license_copy_failed")}: ${le}`);
        }
      } else {
        await invoke("restore_dll", { dllPath: editor.dll_path });
        addLog("success", `[${editor.version}] ${t("editor.restore")} ✓`);
      }
    } catch (e) {
      addLog("error", `[${editor.version}] ${e}`);
    }
    setBusyPath(null);
    await scanEditors();
  }

  async function handleBatch(action: "patch" | "restore") {
    if (!hubPatched && action === "patch") {
      addLog("warn", t("editor.hub_first"));
      return;
    }
    const targets = editors.filter((e) =>
      action === "patch" ? e.dll_status === "original" : (e.dll_status === "patched" || e.dll_status === "patched_no_backup")
    );
    if (targets.length === 0) return;
    setBatchLoading(true);

    if (action === "patch") {
      try {
        const running = await invoke<boolean>("check_process", { name: "Unity.exe" });
        if (running) {
          addLog("warn", t("log.editor_running"));
          await invoke("kill_process", { name: "Unity.exe" });
          await new Promise((r) => setTimeout(r, 1000));
        }
      } catch { /* ignore */ }
    }

    for (const e of targets) {
      try {
        if (action === "patch") {
          await invoke("patch_editor_dll", { dllPath: e.dll_path });
          addLog("success", `[${e.version}] ${t("editor.patch")} ✓`);
        } else {
          await invoke("restore_dll", { dllPath: e.dll_path });
          addLog("success", `[${e.version}] ${t("editor.restore")} ✓`);
        }
      } catch (err) {
        addLog("error", `[${e.version}] ${err}`);
      }
    }
    if (action === "patch") {
      try {
        const result = await invoke<string>("copy_license");
        logLicenseResult(result);
      } catch (e) {
        addLog("error", `${t("log.license_copy_failed")}: ${e}`);
      }
    }
    setBatchLoading(false);
    await scanEditors();
  }

  const canPatch = hubPatched && editors.some((e) => e.dll_status === "original");
  const canRestore = editors.some((e) => e.dll_status === "patched" || e.dll_status === "patched_no_backup");

  return (
    <Box className="tab-content">
      {!hubPatched && (
        <Alert severity="warning" icon={<Warning size={18} />} sx={{ mb: 1 }}>
          {t("editor.hub_first")}
        </Alert>
      )}

      <Paper variant="outlined" sx={{ p: 2 }}>
        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", mb: 1.5 }}>
          <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
            <Wrench size={18} />
            <Typography variant="subtitle1" fontWeight={600}>{t("editor.title")}</Typography>
          </Box>
          <Box sx={{ display: "flex", gap: 1 }}>
            <Button
              size="small"
              variant="contained"
              disabled={!canPatch || batchLoading}
              onClick={() => handleBatch("patch")}
              startIcon={batchLoading ? <CircularProgress size={14} color="inherit" /> : undefined}
            >
              {t("editor.patch_all")}
            </Button>
            <Button
              size="small"
              variant="outlined"
              disabled={!canRestore || batchLoading}
              onClick={() => handleBatch("restore")}
              startIcon={batchLoading ? <CircularProgress size={14} /> : undefined}
            >
              {t("editor.restore_all")}
            </Button>
          </Box>
        </Box>

        <Typography variant="body2" color="text.secondary" sx={{ mb: 1.5 }}>
          {t("editor.desc")}
        </Typography>

        <Link
          component="button"
          sx={{ display: "inline-flex", alignItems: "center", gap: 0.5, mb: 1.5 }}
          onClick={() => invoke("open_browser", { url: "https://unity3d.com/get-unity/download/archive" }).catch(console.error)}
        >
          <DownloadSimple size={16} />
          <Typography variant="body2">{t("editor.download")}</Typography>
        </Link>

        <Box sx={{ mb: 1.5 }}>
          <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", mb: 0.5 }}>
            <Typography variant="caption" color="text.secondary">{t("editor.custom_dirs")}</Typography>
            <Tooltip title={t("editor.add_dir")}>
              <IconButton size="small" onClick={handleAddPath}>
                <Plus size={16} />
              </IconButton>
            </Tooltip>
          </Box>
          {customPaths.length > 0 && (
            <Box sx={{ display: "flex", flexDirection: "column", gap: 0.25 }}>
              {customPaths.map((p) => (
                <Box key={p} sx={{ display: "flex", alignItems: "center", gap: 0.5 }}>
                  <FolderOpen size={14} color="action" />
                  <Typography variant="caption" sx={{ flex: 1, wordBreak: "break-all" }} color="text.secondary">
                    {p}
                  </Typography>
                  <IconButton size="small" onClick={() => handleRemovePath(p)} sx={{ p: 0.25 }}>
                    <Trash size={14} />
                  </IconButton>
                </Box>
              ))}
            </Box>
          )}
        </Box>

        <Divider sx={{ mb: 1.5 }} />

        {scanning ? (
          <Box sx={{ display: "flex", justifyContent: "center", py: 3 }}>
            <CircularProgress />
          </Box>
        ) : editors.length === 0 ? (
          <Typography variant="body2" color="text.disabled" sx={{ textAlign: "center", py: 3 }}>
            {t("editor.no_editors")}
          </Typography>
        ) : (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 0.5 }}>
            {editors.map((e) => {
              const isPatched = e.dll_status === "patched";
              const isOriginal = e.dll_status === "original";
              const isBusy = busyPath === e.dll_path;
              return (
                <Box
                  key={e.dll_path}
                  sx={{
                    display: "flex",
                    alignItems: "center",
                    gap: 1,
                    py: 0.75,
                    px: 1,
                    borderRadius: 1,
                    "&:hover": { bgcolor: "action.hover" },
                  }}
                >
                  <Box sx={{ flex: 1, minWidth: 0 }}>
                    <Typography variant="body2" fontWeight={500} noWrap>
                      {e.product_name} {e.version}
                    </Typography>
                    <Typography variant="caption" color="text.secondary" noWrap>
                      {e.path}
                    </Typography>
                  </Box>
                  {statusChip(e.dll_status)}
                  <Button
                    size="small"
                    variant="contained"
                    disabled={isPatched || !hubPatched || isBusy}
                    onClick={() => handleSingle(e, "patch")}
                    sx={{ minWidth: 64, height: 28 }}
                  >
                    {isBusy ? <CircularProgress size={14} color="inherit" /> : t("editor.patch")}
                  </Button>
                  <Button
                    size="small"
                    variant="outlined"
                    disabled={isOriginal || isBusy}
                    onClick={() => handleSingle(e, "restore")}
                    sx={{ minWidth: 64, height: 28 }}
                  >
                    {isBusy ? <CircularProgress size={14} /> : t("editor.restore")}
                  </Button>
                </Box>
              );
            })}
          </Box>
        )}
      </Paper>
    </Box>
  );
}
