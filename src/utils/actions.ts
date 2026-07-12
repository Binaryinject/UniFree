import { invoke } from "@tauri-apps/api/core";
import type { TFunction } from "i18next";
import type { LogEntry } from "../App";

export type AddLog = (level: LogEntry["level"], message: string) => void;

export function logLicenseResult(t: TFunction, addLog: AddLog, result: string) {
  if (result.startsWith("skipped_missing_signature:")) {
    addLog("warn", t("log.license_skipped_missing_signature"));
  } else if (result.startsWith("preserved_signed:")) {
    addLog("success", t("log.license_preserved"));
  } else {
    addLog("success", t("log.license_copied"));
  }
}

export async function relaunchAsAdmin(t: TFunction, addLog: AddLog) {
  try {
    await invoke("relaunch_as_admin");
    addLog("info", t("log.admin_relaunch_started"));
  } catch (e) {
    addLog("error", `${t("log.admin_relaunch_failed")}: ${e}`);
  }
}