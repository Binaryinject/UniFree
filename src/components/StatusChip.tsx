import { Chip } from "@mui/material";
import { useTranslation } from "react-i18next";

type StatusColor = "success" | "info" | "default" | "warning";

const STATUS_MAP: Record<string, { color: StatusColor; key: string }> = {
  patched: { color: "success", key: "status.patched" },
  authorized: { color: "success", key: "status.authorized" },
  original: { color: "info", key: "status.original" },
  unauthorized: { color: "info", key: "status.unauthorized" },
  not_found: { color: "default", key: "status.not_found" },
  unknown: { color: "default", key: "status.unknown" },
  mismatch: { color: "warning", key: "status.mismatch" },
  missing_signature: { color: "warning", key: "status.missing_signature" },
  patched_no_backup: { color: "warning", key: "status.patched_no_backup" },
  partial: { color: "warning", key: "status.partial" },
};

interface Props {
  status: string;
  variant?: "outlined" | "filled";
  sx?: object;
}

export default function StatusChip({ status, variant = "outlined", sx }: Props) {
  const { t } = useTranslation();
  const entry = STATUS_MAP[status] ?? STATUS_MAP.unknown;
  return <Chip size="small" color={entry.color} label={t(entry.key)} variant={variant} sx={sx} />;
}