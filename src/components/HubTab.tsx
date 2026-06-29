import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Alert, Box, Button, Chip, CircularProgress, Typography, Paper, Switch, FormControlLabel, Divider,
} from "@mui/material";
import { ShieldCheck, ArrowCounterClockwise, Warning } from "@phosphor-icons/react";
import type { LogEntry } from "../App";

interface Props {
  addLog: (level: LogEntry["level"], message: string) => void;
}

export default function HubTab({ addLog }: Props) {
  const { t } = useTranslation();
  const [hubStatus, setHubStatus] = useState<string>("unknown");
  const [hubConfigStatus, setHubConfigStatus] = useState<string>("unknown");
  const [hubCertStatus, setHubCertStatus] = useState<string>("unknown");
  const [licenseStatus, setLicenseStatus] = useState<string>("unknown");
  const [isAdmin, setIsAdmin] = useState(true);
  const [loading, setLoading] = useState(false);
  const [scanning, setScanning] = useState(true);
  const [disableSignin, setDisableSignin] = useState(true);
  const [disableUpdate, setDisableUpdate] = useState(true);

  useEffect(() => { scanHub(); }, []);

  async function scanHub() {
    setScanning(true);
    try {
      const [status, config, cert, license, admin] = await Promise.all([
        invoke<string>("check_hub_dll_status"),
        invoke<string>("check_hub_config_status"),
        invoke<string>("check_hub_cert_status"),
        invoke<string>("check_license_status"),
        invoke<boolean>("check_admin"),
      ]);
      setHubStatus(status);
      setHubConfigStatus(config);
      setHubCertStatus(cert);
      setLicenseStatus(license);
      setIsAdmin(admin);
    } catch (e) {
      addLog("error", `${t("log.scan_failed")}: ${e}`);
    }
    setScanning(false);
  }

  function statusChip(status: string) {
    const map: Record<string, { color: "success" | "info" | "default" | "warning"; label: string }> = {
      patched: { color: "success", label: t("status.patched") },
      authorized: { color: "success", label: t("status.authorized") },
      original: { color: "info", label: t("status.original") },
      unauthorized: { color: "info", label: t("status.unauthorized") },
      not_found: { color: "default", label: t("status.not_found") },
      unknown: { color: "default", label: t("status.unknown") },
      mismatch: { color: "warning", label: t("status.mismatch") },
      missing_signature: { color: "warning", label: t("status.missing_signature") },
      patched_no_backup: { color: "warning", label: t("status.patched_no_backup") },
      partial: { color: "warning", label: t("status.partial") },
    };
    const s = map[status] ?? map.unknown;
    return <Chip size="small" color={s.color} label={s.label} variant="outlined" />;
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

  async function handlePatch() {
    if (!isAdmin) {
      addLog("warn", t("log.admin_required"));
      return;
    }
    setLoading(true);
    try {
      const running = await invoke<boolean>("check_process", { name: "Unity Hub.exe" });
      if (running) {
        addLog("warn", t("log.hub_running"));
        await invoke("kill_process", { name: "Unity Hub.exe" });
        await new Promise((r) => setTimeout(r, 1000));
      }
    } catch { /* ignore */ }

    try {
      await invoke("patch_hub", { disableSignin, disableUpdate });
      addLog("success", t("hub.patch_success"));
      try {
        const result = await invoke<string>("copy_license");
        logLicenseResult(result);
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
    } catch (e) {
      addLog("error", `[Hub] ${e}`);
    }
    setLoading(false);
    await scanHub();
  }

  async function handleRestore() {
    if (!isAdmin) {
      addLog("warn", t("log.admin_required"));
      return;
    }
    setLoading(true);
    try {
      const running = await invoke<boolean>("check_process", { name: "Unity Hub.exe" });
      if (running) {
        addLog("warn", t("log.hub_running"));
        await invoke("kill_process", { name: "Unity Hub.exe" });
        await new Promise((r) => setTimeout(r, 1000));
      }
    } catch { /* ignore */ }

    try {
      await invoke("restore_hub");
      addLog("success", t("hub.restore_success"));
    } catch (e) {
      addLog("error", `[Hub] ${e}`);
    }
    setLoading(false);
    await scanHub();
  }

  async function handleRelaunchAsAdmin() {
    try {
      await invoke("relaunch_as_admin");
      addLog("info", t("log.admin_relaunch_started"));
    } catch (e) {
      addLog("error", `${t("log.admin_relaunch_failed")}: ${e}`);
    }
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

      <Paper variant="outlined" sx={{ p: 2 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 1 }}>
          <ShieldCheck size={20} />
          <Typography variant="subtitle1" fontWeight={600}>{t("hub.title")}</Typography>
          {scanning ? <CircularProgress size={16} /> : statusChip(hubStatus)}
        </Box>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {t("hub.desc")}
        </Typography>

        <Alert severity="info" icon={<ShieldCheck size={18} />} sx={{ mb: 2 }}>
          <Typography variant="body2" fontWeight={600}>{t("hub.unihacker_method_title")}</Typography>
          <Typography variant="caption" display="block">
            • {t("hub.unihacker_method_step1")}
          </Typography>
          <Typography variant="caption" display="block">
            • {t("hub.unihacker_method_step2")}
          </Typography>
          <Typography variant="caption" display="block">
            • {t("hub.unihacker_method_step3")}
          </Typography>
        </Alert>

        <Box sx={{ display: "flex", gap: 1, mb: 2 }}>
          <Button
            variant="contained"
            startIcon={loading ? <CircularProgress size={16} color="inherit" /> : <ShieldCheck size={16} />}
            disabled={loading || isPatched || !isAdmin}
            onClick={handlePatch}
            sx={{ flex: 1 }}
          >
            {loading ? t("hub.patching") : t("hub.patch")}
          </Button>
          <Button
            variant="outlined"
            startIcon={loading ? <CircularProgress size={16} /> : <ArrowCounterClockwise size={16} />}
            disabled={loading || !canRestore || !isAdmin}
            onClick={handleRestore}
            sx={{ flex: 1 }}
          >
            {loading ? t("hub.restoring") : t("hub.restore")}
          </Button>
        </Box>

        <Divider sx={{ my: 1.5 }} />

        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 1, mb: 1 }}>
          <Box sx={{ minWidth: 0 }}>
            <Typography variant="body2">{t("hub.cert_patch")}</Typography>
            <Typography variant="caption" color="text.secondary">{t("hub.cert_patch_desc")}</Typography>
          </Box>
          {scanning ? <CircularProgress size={16} /> : statusChip(hubCertStatus)}
        </Box>

        <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 1, mb: 1.5 }}>
          <Box sx={{ minWidth: 0 }}>
            <Typography variant="body2">{t("hub.config_patch")}</Typography>
            <Typography variant="caption" color="text.secondary">{t("hub.config_patch_desc")}</Typography>
          </Box>
          {scanning ? <CircularProgress size={16} /> : statusChip(hubConfigStatus)}
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

        <Box sx={{ mt: 1, display: "flex", alignItems: "center", justifyContent: "space-between", gap: 1 }}>
          <Box sx={{ minWidth: 0 }}>
            <Typography variant="body2">{t("hub.license")}</Typography>
            <Typography variant="caption" color="text.secondary">{t("hub.license_desc")}</Typography>
          </Box>
          {scanning ? <CircularProgress size={16} /> : statusChip(licenseStatus)}
        </Box>
      </Paper>
    </Box>
  );
}
