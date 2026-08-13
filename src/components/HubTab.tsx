import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Alert, Box, Button, CircularProgress, Typography, Paper, Switch, FormControlLabel, Divider,
} from "@mui/material";
import { ShieldCheck, ArrowCounterClockwise, Warning, FolderOpen } from "@phosphor-icons/react";
import type { LogEntry } from "../App";
import StatusChip from "./StatusChip";
import { logLicenseResult, relaunchAsAdmin } from "../utils/actions";

interface Props {
  addLog: (level: LogEntry["level"], message: string) => void;
  licenseStatus: string;
  isAdmin: boolean;
  hubStatus: string;
  onRefresh: () => Promise<string | null>;
  onHubStatusChange: () => Promise<string | null>;
}

export default function HubTab({ addLog, licenseStatus, isAdmin, hubStatus, onRefresh, onHubStatusChange }: Props) {
  const { t } = useTranslation();
  const [hubConfigStatus, setHubConfigStatus] = useState<string>("unknown");
  const [disableSignin, setDisableSignin] = useState(true);
  const [disableUpdate, setDisableUpdate] = useState(true);
  const [hubPath, setHubPath] = useState<string>("");
  const [patching, setPatching] = useState(false);
  const [restoring, setRestoring] = useState(false);

  useEffect(() => {
    scanHubConfig();
    loadHubPath();
  }, []);

  async function scanHubConfig() {
    try {
      const config = await invoke<string>("check_hub_config_status");
      setHubConfigStatus(config);
    } catch (e) {
      addLog("error", `${t("log.scan_failed")}: ${e}`);
    }
  }

  async function loadHubPath() {
    try {
      const path = await invoke<string>("get_hub_path");
      setHubPath(path);
    } catch { /* ignore */ }
  }

  async function handleSelectHubPath(): Promise<boolean> {
    try {
      const path = await invoke<string>("select_hub_path");
      setHubPath(path);
      addLog("success", `Unity Hub path set: ${path}`);
      await scanHubConfig();
      await onHubStatusChange();
      return true;
    } catch (e) {
      // User cancelled or error occurred
      if (e !== "No file selected" && !String(e).includes("cancelled")) {
        addLog("error", `${t("log.select_hub_failed")}: ${e}`);
      }
      return false;
    }
  }

  async function handleResetHubPath() {
    try {
      await invoke("reset_hub_path");
      setHubPath("");
      addLog("info", "Hub path reset to default");
      await scanHubConfig();
      await onHubStatusChange();
    } catch (e) {
      addLog("error", `${e}`);
    }
  }

  async function doPatch() {
    try {
      const running = await invoke<boolean>("check_process", { name: "Unity Hub.exe" });
      if (running) {
        addLog("warn", t("log.hub_running"));
        await invoke("kill_process", { name: "Unity Hub.exe" });
        await new Promise((r) => setTimeout(r, 1000));
      }
    } catch { /* ignore */ }

    await invoke("patch_hub", { disableSignin, disableUpdate });
    addLog("success", t("hub.patch_success"));
    try {
      const result = await invoke<string>("copy_license");
      logLicenseResult(t, addLog, result);
    } catch (e) {
      addLog("error", `${t("log.license_copy_failed")}: ${e}`);
    }
    // Auto-launch Hub after patch
    try {
      await invoke("launch_hub");
      addLog("success", t("log.hub_launched"));
    } catch (e) {
      addLog("error", `${t("log.hub_launch_failed")}: ${e}`);
    }
  }

  async function handlePatch() {
    if (!isAdmin) {
      addLog("warn", t("log.admin_required"));
      return;
    }
    setPatching(true);
    try {
      await doPatch();
    } catch (e) {
      const err = String(e);
      // If app.asar not found, prompt user to select Hub location then retry
      if (err.includes("app.asar not found")) {
        addLog("warn", t("hub.not_found_hint"));
        const selected = await handleSelectHubPath();
        if (selected) {
          try {
            await doPatch();
          } catch (retryErr) {
            addLog("error", `[Hub] ${retryErr}`);
          }
        }
      } else {
        addLog("error", `[Hub] ${err}`);
      }
    }
    setPatching(false);
    await scanHubConfig();
    await onHubStatusChange();
    await onRefresh();
  }

  async function doRestore() {
    try {
      const running = await invoke<boolean>("check_process", { name: "Unity Hub.exe" });
      if (running) {
        addLog("warn", t("log.hub_running"));
        await invoke("kill_process", { name: "Unity Hub.exe" });
        await new Promise((r) => setTimeout(r, 1000));
      }
    } catch { /* ignore */ }

    await invoke("restore_hub");
    addLog("success", t("hub.restore_success"));
  }

  async function handleRestore() {
    if (!isAdmin) {
      addLog("warn", t("log.admin_required"));
      return;
    }
    setRestoring(true);
    try {
      await doRestore();
    } catch (e) {
      const err = String(e);
      if (err.includes("app.asar not found")) {
        addLog("warn", t("hub.not_found_hint"));
        const selected = await handleSelectHubPath();
        if (selected) {
          try {
            await doRestore();
          } catch (retryErr) {
            addLog("error", `[Hub] ${retryErr}`);
          }
        }
      } else {
        addLog("error", `[Hub] ${err}`);
      }
    }
    setRestoring(false);
    await scanHubConfig();
    await onHubStatusChange();
    await onRefresh();
  }

  async function handleRelaunchAsAdmin() {
    await relaunchAsAdmin(t, addLog);
  }

  const isPatched = hubStatus === "patched";
  const canRestore = hubStatus === "patched";

  return (
    <Box className="tab-content">
      {!isAdmin && (
        <Alert
          severity="warning"
          icon={<Warning size={18} />}
          action={
            <Button color="inherit" size="small" onClick={handleRelaunchAsAdmin}>
              {t("app.run_as_admin")}
            </Button>
          }
          sx={{ mb: 1 }}
        >
          {t("app.admin_hint")}
        </Alert>
      )}

      {licenseStatus === "missing_signature" && (
        <Alert severity="warning" icon={<Warning size={18} />} sx={{ mb: 1 }}>
          {t("hub.license_missing_signature")}
        </Alert>
      )}

      {licenseStatus !== "authorized" && licenseStatus !== "missing_signature" && (
        <Alert severity="info" icon={<Warning size={18} />} sx={{ mb: 1 }}>
          {t("hub.license_required_hint")}
        </Alert>
      )}

      {hubStatus === "not_found" && (
        <Alert
          severity="warning"
          icon={<FolderOpen size={18} />}
          action={
            <Button color="inherit" size="small" onClick={handleSelectHubPath}>
              {t("hub.select_path")}
            </Button>
          }
          sx={{ mb: 1 }}
        >
          {t("hub.not_found_hint")}
        </Alert>
      )}

      <Paper variant="outlined" sx={{ p: 2 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 1 }}>
          <ShieldCheck size={20} />
          <Typography variant="subtitle1" fontWeight={600}>{t("hub.title")}</Typography>
          <StatusChip status={hubStatus} />
        </Box>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {t("hub.desc")}
        </Typography>

        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 1, mb: 1.5 }}>
          <Box sx={{ minWidth: 0, flex: 1 }}>
            <Typography variant="body2">{t("hub.install_path")}</Typography>
            <Typography variant="caption" color="text.secondary" sx={{ wordBreak: "break-all" }}>
              {hubPath || t("hub.install_path_default")}
            </Typography>
          </Box>
          <Box sx={{ display: "flex", gap: 0.5 }}>
            <Button size="small" variant="outlined" onClick={handleSelectHubPath} startIcon={<FolderOpen size={14} />}>
              {t("hub.browse")}
            </Button>
            {hubPath && (
              <Button size="small" variant="text" onClick={handleResetHubPath}>
                {t("hub.reset")}
              </Button>
            )}
          </Box>
        </Box>

        <Divider sx={{ my: 1.5 }} />

        {/* Hub 补丁 - 需要许可证已授权 */}
        <Box sx={{ display: "flex", gap: 1, mb: 2 }}>
          <Button
            variant="contained"
            startIcon={patching ? <CircularProgress size={16} color="inherit" /> : <ShieldCheck size={16} />}
            disabled={patching || restoring || isPatched || !isAdmin || licenseStatus !== "authorized"}
            onClick={handlePatch}
            sx={{ flex: 1 }}
          >
            {patching ? t("hub.patching") : t("hub.patch")}
          </Button>
          <Button
            variant="outlined"
            startIcon={restoring ? <CircularProgress size={16} /> : <ArrowCounterClockwise size={16} />}
            disabled={patching || restoring || !canRestore || !isAdmin}
            onClick={handleRestore}
            sx={{ flex: 1 }}
          >
            {restoring ? t("hub.restoring") : t("hub.restore")}
          </Button>
        </Box>

        <Divider sx={{ my: 1.5 }} />

        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 1, mb: 1.5 }}>
          <Box sx={{ minWidth: 0 }}>
            <Typography variant="body2">{t("hub.config_patch")}</Typography>
            <Typography variant="caption" color="text.secondary">{t("hub.config_patch_desc")}</Typography>
          </Box>
          <StatusChip status={hubConfigStatus} />
        </Box>

        <FormControlLabel
          control={<Switch checked={disableSignin} onChange={(e) => setDisableSignin(e.target.checked)} size="small" />}
          label={
            <Box>
              <Typography variant="body2">{t("hub.disable_signin")}</Typography>
              <Typography variant="caption" color="text.secondary">{t("hub.disable_signin_desc")}</Typography>
            </Box>
          }
        />

        <FormControlLabel
          control={<Switch checked={disableUpdate} onChange={(e) => setDisableUpdate(e.target.checked)} size="small" />}
          label={
            <Box>
              <Typography variant="body2">{t("hub.disable_update")}</Typography>
              <Typography variant="caption" color="text.secondary">{t("hub.disable_update_desc")}</Typography>
            </Box>
          }
        />
      </Paper>
    </Box>
  );
}
