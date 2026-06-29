import { useTranslation } from "react-i18next";
import { Box, Typography, Link } from "@mui/material";

export default function AboutTab() {
  const { t } = useTranslation();

  return (
    <Box className="tab-content" sx={{ textAlign: "center", pt: 3 }}>
      <Typography variant="h5" fontWeight={600}>UniFree v2.0</Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
        {t("about.description")}
      </Typography>
      <Box sx={{ mt: 2, display: "flex", flexDirection: "column", gap: 0.5, alignItems: "center" }}>
        <Typography variant="body2">{t("about.version")}: 2.0.0</Typography>
        <Typography variant="body2">{t("about.author")}: BinaryInject</Typography>
        <Link href="https://github.com/agentbillwh/unilic" target="_blank" variant="body2">
          {t("about.github")}
        </Link>
      </Box>
    </Box>
  );
}
