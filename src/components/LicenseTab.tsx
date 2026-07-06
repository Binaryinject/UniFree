import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import {
  Box, Button, Chip, CircularProgress, Typography, Paper, LinearProgress, Divider, Alert,
} from "@mui/material";
import { Certificate, Warning } from "@phosphor-icons/react";
import type { LogEntry } from "../App";

interface Props {
  addLog: (level: LogEntry["level"], message: string) => void;
  licenseStatus: string;
  isAdmin: boolean;
  scanning?: boolean;
  onRefresh: () => Promise<void>;
}

export default function LicenseTab({ addLog, licenseStatus, isAdmin, scanning, onRefresh }: Props) {
  const { t } = useTranslation();
  const [generatingLicense, setGeneratingLicense] = useState(false);
  const [licenseProgress, setLicenseProgress] = useState(0);

  function statusChip(status: string) {
    const map: Record<string, { color: "success" | "info" | "default" | "warning"; label: string }> = {
      authorized: { color: "success", label: t("status.authorized") },
      unauthorized: { color: "info", label: t("status.unauthorized") },
      not_found: { color: "default", label: t("status.not_found") },
      unknown: { color: "default", label: t("status.unknown") },
      missing_signature: { color: "warning", label: t("status.missing_signature") },
    };
    const s = map[status] ?? map.unknown;
    return <Chip size="small" color={s.color} label={s.label} variant="outlined" />;
  }

  async function handleGenerateLicense() {
    if (!isAdmin) {
      addLog("warn", t("log.admin_required"));
      return;
    }

    setGeneratingLicense(true);
    setLicenseProgress(0);

    try {
      addLog("info", t("hub.generating_alf"));
      setLicenseProgress(25);
      await invoke<string>("generate_alf");
      addLog("success", t("hub.alf_generated"));

      setLicenseProgress(50);
      await new Promise((r) => setTimeout(r, 500));

      addLog("info", t("hub.generating_license"));
      setLicenseProgress(75);
      const result = await invoke<string>("generate_license_direct", {
        product: "Unity Pro",
        privateKeyPem: null,
      });

      setLicenseProgress(90);
      addLog("success", result);

      await new Promise((r) => setTimeout(r, 500));
      await onRefresh();

      if (licenseStatus === "authorized") {
        setLicenseProgress(100);
        addLog("success", t("hub.license_generated_success"));
      } else {
        addLog("warn", `${t("hub.license_status")}: ${licenseStatus}`);
      }

    } catch (e) {
      addLog("error", `${t("hub.license_generation_failed")}: ${e}`);
    } finally {
      setGeneratingLicense(false);
      await onRefresh();
    }
  }

  async function handleRelaunchAsAdmin() {
    try {
      await invoke("relaunch_as_admin");
      addLog("info", t("log.admin_relaunch_started"));
    } catch (e) {
      addLog("error", `${t("log.admin_relaunch_failed")}: ${e}`);
    }
  }

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

      <Paper variant="outlined" sx={{ p: 2 }}>
        <Box sx={{ display: "flex", alignItems: "center", gap: 1, mb: 1 }}>
          <Certificate size={20} />
          <Typography variant="subtitle1" fontWeight={600}>{t("hub.auto_license_title")}</Typography>
          {scanning ? <CircularProgress size={16} /> : statusChip(licenseStatus)}
        </Box>

        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          {licenseStatus === "authorized"
            ? t("hub.auto_license_regenerate_desc")
            : t("hub.auto_license_desc")}
        </Typography>

        <Divider sx={{ my: 1.5 }} />

        <Button
          variant="contained"
          startIcon={generatingLicense ? <CircularProgress size={16} color="inherit" /> : <Certificate size={16} />}
          disabled={generatingLicense || !isAdmin}
          onClick={handleGenerateLicense}
          fullWidth
        >
          {generatingLicense ? t("hub.generating_license_btn") : t("hub.generate_license_btn")}
        </Button>

        {generatingLicense && (
          <Box sx={{ mt: 1.5 }}>
            <LinearProgress variant="determinate" value={licenseProgress} />
            <Typography variant="caption" color="text.secondary" sx={{ mt: 0.5, display: "block" }}>
              {licenseProgress}%
            </Typography>
          </Box>
        )}
      </Paper>
    </Box>
  );
}
