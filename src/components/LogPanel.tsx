import { useCallback, useLayoutEffect, useRef } from "react";
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
  // 记录用户当前是否停留在底部（由滚动事件维护）
  const atBottomRef = useRef(true);

  const handleScroll = useCallback(() => {
    const el = bodyRef.current;
    if (el) {
      atBottomRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 8;
    }
  }, []);

  // 仅在用户停留在底部时，新增日志才自动滚动到底部；
  // 若用户向上翻阅历史日志，则不打断其查看位置。
  // 用 useLayoutEffect 在浏览器绘制前同步滚动，避免 useEffect 异步时序
  // 导致新日志行已插入但 scrollHeight 尚未稳定、滚动丢失的问题。
  useLayoutEffect(() => {
    const el = bodyRef.current;
    if (el && atBottomRef.current) {
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
      <div className="log-body" ref={bodyRef} onScroll={handleScroll}>
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
      </div>
    </Paper>
  );
}
