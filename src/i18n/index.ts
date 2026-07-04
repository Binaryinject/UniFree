import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import zh from "./zh.json";
import en from "./en.json";

async function initI18n() {
  const lang = navigator.language.startsWith("zh") ? "zh" : "en";
  await i18n.use(initReactI18next).init({
    resources: { zh: { translation: zh }, en: { translation: en } },
    lng: lang,
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });
}

export { initI18n };
export default i18n;
