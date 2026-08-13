import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Box, Button, Chip, Typography, Paper } from "@mui/material";
import { Trash } from "@phosphor-icons/react";
import type { LogEntry } from "../App";

interface Props {
  logs: LogEntry[];
  clearLogs: () => void;
}

const levelColor: Record<string, "info" | "success" | "error" | "warning"> = {
  info: "info",
  success: "success",
  error: "error",
  warn: "warning",
};

export default function LogPanel({ logs, clearLogs }: Props) {
  const { t } = useTranslation();
  const bodyRef = useRef<HTMLDivElement>(null);

  // 新增日志时自动滚动到底部，定位到最新一行
  useEffect(() => {
    const el = bodyRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [logs]);

  return (
    <Paper variant="outlined" className="log-panel">
      <Box className="log-header">
        <Typography variant="subtitle2" fontWeight={600}>{t("log.title")}</Typography>
        <Button size="small" startIcon={<Trash size={14} />} onClick={clearLogs}>
          {t("log.clear")}
        </Button>
      </Box>
      <Box className="log-body" ref={bodyRef}>
        {logs.length === 0 ? (
          <Typography variant="caption" color="text.disabled">{t("log.empty")}</Typography>
        ) : (
          logs.map((log, i) => (
            <Box key={i} className="log-entry">
              <Typography variant="caption" color="text.secondary" sx={{ flexShrink: 0 }}>
                {log.time}
              </Typography>
              <Chip size="small" color={levelColor[log.level]} label={log.level.toUpperCase()} variant="filled" sx={{ height: 18, fontSize: 10 }} />
              <Typography variant="caption">{log.message}</Typography>
            </Box>
          ))
        )}
      </Box>
    </Paper>
  );
}
