//! i18n module for handling translations

// Note: rust_i18n::i18n! macro is called in main.rs at crate root

/// Get translation for a key with optional arguments
pub fn translate(locale: &str, key: &str, args: Option<&[(&str, &str)]>) -> String {
    rust_i18n::set_locale(locale);
    
    // The t! macro is created by the i18n! macro above
    let result = rust_i18n::t!(key);
    
    if let Some(args_map) = args {
        // Replace placeholders in format {key}
        let mut result_str = result.to_string();
        for (k, v) in args_map {
            result_str = result_str.replace(&format!("{{{}}}", k), v);
        }
        result_str
    } else {
        result.to_string()
    }
}

/// Helper function to get user language or default to English
pub fn get_user_language(language: Option<&String>) -> &str {
    language
        .and_then(|lang| {
            if lang == "vi" || lang == "en" {
                Some(lang.as_str())
            } else {
                None
            }
        })
        .unwrap_or("en")
}

/// Get translation for user with their language
pub fn t_for_user(language: Option<&String>, key: &str, args: Option<&[(&str, &str)]>) -> String {
    let locale = get_user_language(language);
    translate(locale, key, args)
}
