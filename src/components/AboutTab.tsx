import { useTranslation } from "react-i18next";
import { Box, Typography, Link } from "@mui/material";
import { invoke } from "@tauri-apps/api/core";

export default function AboutTab() {
  const { t } = useTranslation();

  const openUrl = (url: string) => {
    invoke("open_browser", { url }).catch(console.error);
  };

  return (
    <Box className="tab-content" sx={{ textAlign: "center", pt: 3 }}>
      <Typography variant="h5" fontWeight={600}>UniFree v2.3.1</Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
        {t("about.description")}
      </Typography>
      <Box sx={{ mt: 2, display: "flex", flexDirection: "column", gap: 0.5, alignItems: "center" }}>
        <Typography variant="body2">{t("about.version")}: 2.3.1</Typography>
        <Typography variant="body2">{t("about.author")}: BinaryInject</Typography>
        <Link
          component="button"
          variant="body2"
          onClick={() => openUrl("https://github.com/Binaryinject/UniFree")}
        >
          {t("about.github")}
        </Link>
      </Box>
    </Box>
  );
}
