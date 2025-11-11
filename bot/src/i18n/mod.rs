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
                    // Only use messages.yml format (en/messages.yml, vi/messages.yml)
                    let yaml_file = base.join(lang).join("messages.yml");
                    
                    if !yaml_file.exists() {
                        continue;
                    }
                    
                    if let Ok(content) = fs::read_to_string(&yaml_file) {
                        let mut lang_map = HashMap::new();
                        let lines: Vec<&str> = content.lines().collect();
                        let mut i = 0;
                        
                        while i < lines.len() {
                            let line = lines[i];
                            let trimmed = line.trim();
                            
                            // Skip comments and empty lines
                            if trimmed.starts_with('#') || trimmed.is_empty() {
                                i += 1;
                                continue;
                            }
                            
                            // Check if this line has a key (contains ':')
                            if let Some(idx) = trimmed.find(':') {
                                let k = trimmed[..idx].trim();
                                let after_colon = trimmed[idx+1..].trim();
                                
                                if !k.is_empty() {
                                    // Check if it's a multi-line string (| or >)
                                    if after_colon == "|" || after_colon == ">" || after_colon == "|-" || after_colon == ">-" {
                                        // Multi-line string: read following indented lines
                                        let mut value_lines = Vec::new();
                                        i += 1;
                                        
                                        // Determine the base indentation level from the next non-empty line
                                        let mut base_indent = 0;
                                        if i < lines.len() {
                                            // Skip empty lines to find the first content line
                                            let mut first_content_line_idx = i;
                                            while first_content_line_idx < lines.len() && lines[first_content_line_idx].trim().is_empty() {
                                                first_content_line_idx += 1;
                                            }
                                            if first_content_line_idx < lines.len() {
                                                base_indent = lines[first_content_line_idx].len() - lines[first_content_line_idx].trim_start().len();
                                            }
                                        }
                                        
                                        // Read all lines that are indented (continuation of multi-line)
                                        while i < lines.len() {
                                            let next_line = lines[i];
                                            let indent = next_line.len() - next_line.trim_start().len();
                                            
                                            // Stop if we hit a root-level key (indent == 0 and has ':')
                                            if !next_line.trim().is_empty() {
                                                if indent == 0 && next_line.contains(':') && !next_line.trim_start().starts_with('#') {
                                                    break;
                                                }
                                                if indent < base_indent && next_line.contains(':') && !next_line.trim_start().starts_with('#') {
                                                    break;
                                                }
                                            }
                                            
                                            // If this line is part of the multi-line value
                                            if indent >= base_indent || next_line.trim().is_empty() {
                                                // Remove the base indentation
                                                if next_line.len() >= base_indent {
                                                    value_lines.push(&next_line[base_indent..]);
                                                } else {
                                                    value_lines.push(next_line);
                                                }
                                            } else if next_line.contains(':') && !next_line.trim_start().starts_with('#') {
                                                break;
                                            }
                                            i += 1;
                                        }
                                        
                                        // Join lines
                                        let mut value = value_lines.join("\n");
                                        if after_colon.ends_with('-') {
                                            value = value.trim_end().to_string();
                                        }
                                        
                                        if !value.is_empty() {
                                            lang_map.insert(k.to_string(), value);
                                        }
                                        continue;
                                    } else {
                                        // Single-line value
                                        let mut v = after_colon.trim_matches('"').trim_matches('\'').to_string();
                                        if !v.is_empty() {
                                            v = v.replace("\\n", "\n")
                                                .replace("\\t", "\t")
                                                .replace("\\r", "\r")
                                                .replace("\\\\", "\\");
                                            lang_map.insert(k.to_string(), v);
                                        }
                                    }
                                }
                            }
                            i += 1;
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
        ("vi", "algorithm_bollinger") => "📈 BB".to_string(),
        ("en", "algorithm_bollinger") => "📈 BB".to_string(),
        ("vi", "algorithm_ema") => "📉 EMA".to_string(),
        ("en", "algorithm_ema") => "📉 EMA".to_string(),
        ("vi", "algorithm_macd") => "📊 MACD".to_string(),
        ("en", "algorithm_macd") => "📊 MACD".to_string(),
        ("vi", "algorithm_ma") => "📊 MA".to_string(),
        ("en", "algorithm_ma") => "📊 MA".to_string(),
        ("vi", "algorithm_stochastic") => "📊 Stochastic".to_string(),
        ("en", "algorithm_stochastic") => "📊 Stochastic".to_string(),
        ("vi", "algorithm_adx") => "📊 ADX".to_string(),
        ("en", "algorithm_adx") => "📊 ADX".to_string(),
        ("vi", "strategy_cancel_button") => "❌ Hủy".to_string(),
        ("en", "strategy_cancel_button") => "❌ Cancel".to_string(),
        ("vi", "strategy_type_custom") => "🛠️ Tùy Chỉnh".to_string(),
        ("en", "strategy_type_custom") => "🛠️ Custom".to_string(),
        ("vi", "strategy_type_preset") => "📚 Có Sẵn".to_string(),
        ("en", "strategy_type_preset") => "📚 Preset".to_string(),
        ("vi", "strategy_type_custom_mix") => "🔀 Mix Chiến Lược".to_string(),
        ("en", "strategy_type_custom_mix") => "🔀 Mix Strategies".to_string(),
        ("vi", "strategy_mix_done") => "✅ Hoàn thành".to_string(),
        ("en", "strategy_mix_done") => "✅ Done".to_string(),
        
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
        
        // Pair buttons
        ("vi", "pair_btc_usdt") => "₿ BTC/USDT".to_string(),
        ("en", "pair_btc_usdt") => "₿ BTC/USDT".to_string(),
        ("vi", "pair_eth_usdt") => "Ξ ETH/USDT".to_string(),
        ("en", "pair_eth_usdt") => "Ξ ETH/USDT".to_string(),
        ("vi", "pair_bnb_usdt") => "BNB/USDT".to_string(),
        ("en", "pair_bnb_usdt") => "BNB/USDT".to_string(),
        ("vi", "pair_ada_usdt") => "ADA/USDT".to_string(),
        ("en", "pair_ada_usdt") => "ADA/USDT".to_string(),
        ("vi", "pair_sol_usdt") => "◎ SOL/USDT".to_string(),
        ("en", "pair_sol_usdt") => "◎ SOL/USDT".to_string(),
        ("vi", "pair_dot_usdt") => "DOT/USDT".to_string(),
        ("en", "pair_dot_usdt") => "DOT/USDT".to_string(),
        ("vi", "pair_manual") => "✏️ Khác".to_string(),
        ("en", "pair_manual") => "✏️ Other".to_string(),
        
        // Payment buttons
        ("vi", "payment_deposit_100") => "💵 Nạp 100 điểm".to_string(),
        ("en", "payment_deposit_100") => "💵 Deposit 100 points".to_string(),
        ("vi", "payment_deposit_500") => "💵 Nạp 500 điểm".to_string(),
        ("en", "payment_deposit_500") => "💵 Deposit 500 points".to_string(),
        ("vi", "payment_deposit_1000") => "💵 Nạp 1,000 điểm".to_string(),
        ("en", "payment_deposit_1000") => "💵 Deposit 1,000 points".to_string(),
        ("vi", "payment_deposit_5000") => "💵 Nạp 5,000 điểm".to_string(),
        ("en", "payment_deposit_5000") => "💵 Deposit 5,000 points".to_string(),
        ("vi", "payment_deposit_custom") => "✏️ Nhập số lượng tùy chỉnh".to_string(),
        ("en", "payment_deposit_custom") => "✏️ Custom Amount".to_string(),
        ("vi", "payment_cancel") => "❌ Hủy".to_string(),
        ("en", "payment_cancel") => "❌ Cancel".to_string(),
        ("vi", "payment_deposit_button") => "💵 Nạp Tiền".to_string(),
        ("en", "payment_deposit_button") => "💵 Deposit".to_string(),
        
        // Profile buttons
        ("vi", "profile_change_language") => "🌐 Đổi Ngôn Ngữ / Change Language".to_string(),
        ("en", "profile_change_language") => "🌐 Change Language".to_string(),
        
        // Strategy delete buttons
        ("vi", "strategy_delete_with_name") => "🗑️ Xóa".to_string(),
        ("en", "strategy_delete_with_name") => "🗑️ Delete".to_string(),
        ("vi", "strategy_delete_confirm_yes") => "✅ Xác nhận".to_string(),
        ("en", "strategy_delete_confirm_yes") => "✅ Confirm".to_string(),
        ("vi", "strategy_delete_confirm_no") => "❌ Hủy".to_string(),
        ("en", "strategy_delete_confirm_no") => "❌ Cancel".to_string(),
        
        // Live trading buttons
        ("vi", "live_trading_setup_binance") => "🔵 Thiết lập Binance".to_string(),
        ("en", "live_trading_setup_binance") => "🔵 Setup Binance".to_string(),
        ("vi", "live_trading_setup_okx") => "🟢 Thiết lập OKX".to_string(),
        ("en", "live_trading_setup_okx") => "🟢 Setup OKX".to_string(),
        ("vi", "live_trading_start_trading") => "🚀 Bắt đầu giao dịch".to_string(),
        ("en", "live_trading_start_trading") => "🚀 Start Trading".to_string(),
        
        // Trading buttons
        ("vi", "trading_cancel") => "❌ Hủy".to_string(),
        ("en", "trading_cancel") => "❌ Cancel".to_string(),
        
        // Stop trading buttons
        ("vi", "stop_trading_confirm_yes") => "✅ Xác nhận".to_string(),
        ("en", "stop_trading_confirm_yes") => "✅ Confirm".to_string(),
        ("vi", "stop_trading_confirm_no") => "❌ Hủy".to_string(),
        ("en", "stop_trading_confirm_no") => "❌ Cancel".to_string(),
        
        // Live trading callback feedback buttons
        ("vi", "live_trading_cancelled") => "❌ Đã hủy".to_string(),
        ("en", "live_trading_cancelled") => "❌ Cancelled".to_string(),
        
        // My trading buttons
        ("vi", "mytrading_stop_button") => "🛑 Dừng Live Trading".to_string(),
        ("en", "mytrading_stop_button") => "🛑 Stop Live Trading".to_string(),
        
        // Default fallback
        _ => key.to_string(),
    }
}
