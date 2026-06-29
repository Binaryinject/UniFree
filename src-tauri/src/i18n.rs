pub fn get_system_lang() -> String {
    match sys_locale::get_locale() {
        Some(locale) => {
            if locale.starts_with("zh") {
                "zh".to_string()
            } else {
                "en".to_string()
            }
        }
        None => "en".to_string(),
    }
}
