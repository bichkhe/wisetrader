//! i18n module for handling translations

// Note: rust_i18n::i18n! macro is called in main.rs at crate root
// The t! macro can only be used at crate root, so we use a helper approach

/// Get translation for a key with optional arguments
/// 
/// EXPLANATION: The rust_i18n::t! macro created by i18n! can only be used at crate root.
/// Since we're in a module, we use rust_i18n::set_locale() + rust_i18n::t! directly.
/// After set_locale(), t! should work with the current locale context.
pub fn translate(locale: &str, key: &str, args: Option<&[(&str, &str)]>) -> String {
    // Set the locale for the current thread
    rust_i18n::set_locale(locale);
    
    // The problem: t! macro is only available at crate root where i18n! is called.
    // Solution: We need to expose a helper in main.rs OR load YAML directly.
    // For now, let's try using rust_i18n::t! with full qualification.
    // Actually, in rust-i18n 3.1, after set_locale(), t! should work.
    // But since we're in a module, we can't use t! directly.
    
    // WORKAROUND: Use a public function in main.rs that wraps t!
    // OR: Load YAML files directly (simpler for now)
    
    // Let's use a simpler approach: load and parse YAML manually
    use std::sync::LazyLock;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    
    static CACHE: LazyLock<HashMap<String, HashMap<String, String>>> = LazyLock::new(|| {
        let mut map = HashMap::new();
        
        // Try different paths to find locales
        let paths = [
            "bot/locales",
            "locales",
            "./locales",
        ];
        
        for base_str in &paths {
            let base = PathBuf::from(base_str);
            if base.exists() {
                for lang in ["en", "vi"] {
                    let yaml_file = base.join(lang).join("messages.yml");
                    if let Ok(content) = fs::read_to_string(&yaml_file) {
                        let mut lang_map = HashMap::new();
                        // Simple parser for key: "value" format
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with('#') || trimmed.is_empty() {
                                continue;
                            }
                            if let Some(idx) = trimmed.find(':') {
                                let k = trimmed[..idx].trim();
                                let mut v = trimmed[idx+1..].trim().trim_matches('"').trim_matches('\'').to_string();
                                if !k.is_empty() && !v.is_empty() {
                                    // Unescape common escape sequences
                                    // Replace \n with actual newline, \t with tab, etc.
                                    v = v.replace("\\n", "\n")
                                        .replace("\\t", "\t")
                                        .replace("\\r", "\r")
                                        .replace("\\\\", "\\");
                                    lang_map.insert(k.to_string(), v);
                                }
                            }
                        }
                        map.insert(lang.to_string(), lang_map);
                    }
                }
                break;
            }
        }
        map
    });
    
    // Get translation from cache
    let cache = &*CACHE;
    let result = cache
        .get(locale)
        .or_else(|| cache.get("en"))
        .and_then(|m| m.get(key))
        .map(|s| s.as_str())
        .unwrap_or(key);
    
    if let Some(args_map) = args {
        // Replace placeholders in format {key}
        // IMPORTANT: Escape all argument values to prevent HTML parsing errors
        let mut result_str = result.to_string();
        for (k, v) in args_map {
            // Escape the argument value before inserting it
            // This prevents HTML parsing errors when arguments contain <, >, or &
            let escaped_value = v
                .replace("&", "&amp;")
                .replace("<", "&lt;")
                .replace(">", "&gt;")
                .replace("\"", "&quot;");
            result_str = result_str.replace(&format!("{{{}}}", k), &escaped_value);
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

/// Get button text for inline keyboard buttons
/// This is a workaround for i18n translation issues with buttons
pub fn get_button_text(locale: &str, key: &str) -> String {
    match (locale, key) {
        // Language selection
        ("vi", "lang_selection_button_vi") => "🇻🇳 Tiếng Việt".to_string(),
        ("vi", "lang_selection_button_en") => "🇬🇧 English".to_string(),
        ("en", "lang_selection_button_vi") => "🇻🇳 Tiếng Việt".to_string(),
        ("en", "lang_selection_button_en") => "🇬🇧 English".to_string(),
        
        // Backtest buttons
        ("vi", "backtest_button_cancel") => "❌ Hủy".to_string(),
        ("en", "backtest_button_cancel") => "❌ Cancel".to_string(),
        ("vi", "backtest_exchange_binance") => "🔵 Binance".to_string(),
        ("en", "backtest_exchange_binance") => "🔵 Binance".to_string(),
        ("vi", "backtest_exchange_okx") => "🟠 OKX".to_string(),
        ("en", "backtest_exchange_okx") => "🟠 OKX".to_string(),
        ("vi", "period_1day") => "📅 1 Ngày".to_string(),
        ("en", "period_1day") => "📅 1 Day".to_string(),
        ("vi", "period_1week") => "📅 1 Tuần".to_string(),
        ("en", "period_1week") => "📅 1 Week".to_string(),
        ("vi", "period_1month") => "📅 1 Tháng".to_string(),
        ("en", "period_1month") => "📅 1 Month".to_string(),
        ("vi", "period_3months") => "📅 3 Tháng".to_string(),
        ("en", "period_3months") => "📅 3 Months".to_string(),
        ("vi", "period_6months") => "📅 6 Tháng".to_string(),
        ("en", "period_6months") => "📅 6 Months".to_string(),
        
        // Strategy buttons
        ("vi", "algorithm_rsi") => "📊 RSI".to_string(),
        ("en", "algorithm_rsi") => "📊 RSI".to_string(),
        ("vi", "algorithm_bollinger") => "📊 Bollinger Bands".to_string(),
        ("en", "algorithm_bollinger") => "📊 Bollinger Bands".to_string(),
        ("vi", "algorithm_ema") => "📊 EMA".to_string(),
        ("en", "algorithm_ema") => "📊 EMA".to_string(),
        ("vi", "algorithm_macd") => "📊 MACD".to_string(),
        ("en", "algorithm_macd") => "📊 MACD".to_string(),
        ("vi", "algorithm_ma") => "📊 MA".to_string(),
        ("en", "algorithm_ma") => "📊 MA".to_string(),
        ("vi", "strategy_cancel_button") => "❌ Hủy".to_string(),
        ("en", "strategy_cancel_button") => "❌ Cancel".to_string(),
        
        // Timeframe buttons
        ("vi", "timeframe_1m") => "1 phút".to_string(),
        ("en", "timeframe_1m") => "1m".to_string(),
        ("vi", "timeframe_5m") => "5 phút".to_string(),
        ("en", "timeframe_5m") => "5m".to_string(),
        ("vi", "timeframe_15m") => "15 phút".to_string(),
        ("en", "timeframe_15m") => "15m".to_string(),
        ("vi", "timeframe_30m") => "30 phút".to_string(),
        ("en", "timeframe_30m") => "30m".to_string(),
        ("vi", "timeframe_1h") => "1 giờ".to_string(),
        ("en", "timeframe_1h") => "1h".to_string(),
        ("vi", "timeframe_4h") => "4 giờ".to_string(),
        ("en", "timeframe_4h") => "4h".to_string(),
        ("vi", "timeframe_1d") => "1 ngày".to_string(),
        ("en", "timeframe_1d") => "1d".to_string(),
        ("vi", "timeframe_1w") => "1 tuần".to_string(),
        ("en", "timeframe_1w") => "1w".to_string(),
        
        // Pair buttons - keep as is since they are market symbols
        ("vi", "pair_manual") => "✍️ Nhập thủ công".to_string(),
        ("en", "pair_manual") => "✍️ Manual".to_string(),
        
        // Default fallback
        _ => key.to_string(),
    }
}
