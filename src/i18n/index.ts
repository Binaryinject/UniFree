import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zh from "./zh.json";
import en from "./en.json";
import { invoke } from "@tauri-apps/api/core";

async function initI18n() {
  const lang = await invoke<string>("get_system_lang");
  await i18n.use(initReactI18next).init({
    resources: { zh: { translation: zh }, en: { translation: en } },
    lng: lang,
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });
}

export { initI18n };
export default i18n;
