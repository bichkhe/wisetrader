# Hướng Dẫn Thêm Indicators Mới

Tài liệu này hướng dẫn cách thêm indicators mới vào hệ thống WiseTrader.

## Cấu Trúc Module

Hệ thống indicators được module hóa với các thành phần sau:

1. **Strategy Implementation** (`bot/src/services/strategy_engine/implementations.rs`)
   - Implement trait `Strategy` cho indicator mới
   - Xử lý logic tính toán và tạo signals

2. **Strategy Registry** (`bot/src/services/strategy_engine/registry.rs`)
   - Đăng ký indicator vào registry để hệ thống có thể tạo instance

3. **UI Integration** (`bot/src/commands/strategy.rs`)
   - Thêm button và handler cho indicator trong bot commands

4. **Translations** (`bot/locales/vi/messages.yml`, `bot/locales/en/messages.yml`)
   - Thêm translations cho tên và mô tả indicator

5. **Backtest Template Config** (`bot/src/services/strategy_engine/indicator_configs.rs`)
   - Implement `IndicatorConfig` trait để tự động generate Python template cho backtest
   - **✨ Hệ thống module hóa - không cần sửa hàm chung khi thêm indicator mới!**

## Các Bước Thêm Indicator Mới

### Bước 1: Implement Strategy Trait

Trong file `bot/src/services/strategy_engine/implementations.rs`, thêm struct và implementation:

```rust
/// [Tên Indicator] Strategy
#[derive(Debug)]
pub struct [Tên]Strategy {
    config: StrategyConfig,
    // Các fields cần thiết cho indicator
    // Ví dụ: period, prices, last_value, etc.
}

impl [Tên]Strategy {
    pub fn new(config: StrategyConfig, /* parameters */) -> Result<Self> {
        // Khởi tạo indicator từ thư viện ta
        Ok(Self {
            config,
            // Initialize fields
        })
    }
}

impl Strategy for [Tên]Strategy {
    fn name(&self) -> &str {
        "[TÊN INDICATOR]"
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn process_candle(&mut self, candle: &Candle) -> Option<StrategySignal> {
        // 1. Update indicator với giá mới
        // 2. Kiểm tra indicator đã ready chưa
        // 3. Parse buy/sell conditions từ config
        // 4. Return signal nếu có
    }
    
    fn reset(&mut self) {
        // Reset state khi cần
    }
    
    fn get_state_info(&self) -> String {
        // Thông tin debug
    }
}
```

### Bước 2: Đăng Ký Vào Registry

Trong file `bot/src/services/strategy_engine/registry.rs`:

1. **Import strategy mới:**
```rust
use crate::services::strategy_engine::{
    Strategy, StrategyConfig, 
    implementations::{
        RsiStrategy, MacdStrategy, BollingerStrategy, 
        EmaStrategy, MaStrategy,
        [Tên]Strategy, // Thêm dòng này
    },
};
```

2. **Đăng ký trong hàm `new()`:**
```rust
registry.register_strategy("[TÊN]", |config| {
    // Extract parameters từ config.parameters
    let param1 = config.parameters
        .get("param1")
        .and_then(|v| v.as_u64())
        .unwrap_or(default_value) as usize;
    
    Ok(Box::new([Tên]Strategy::new(config, param1)?))
});
```

### Bước 3: Thêm Vào UI

Trong file `bot/src/commands/strategy.rs`:

1. **Thêm button vào algorithm selection:**
```rust
InlineKeyboardButton::callback(
    i18n::get_button_text(&locale, "algorithm_[tên]"),
    "algorithm_[tên]"
),
```

2. **Thêm handler cho callback:**
```rust
"algorithm_[tên]" => {
    bot.answer_callback_query(q.id).await?;
    let algorithm_msg = i18n::translate(&locale, "strategy_algorithm_selected", Some(&[("algorithm", "[Tên]")]));
    let info_msg = i18n::translate(&locale, "strategy_algorithm_[tên]_info", None);
    let step2_msg = i18n::translate(&locale, "strategy_step2_enter_buy", Some(&[("example", "[ví dụ condition]")]));
    let instruction = format!("{}\n\n{}\n\n{}", algorithm_msg, info_msg, step2_msg);
    
    bot.edit_message_text(chat_id, message_id, instruction)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;
    
    dialogue.update(BotState::CreateStrategy(CreateStrategyState::WaitingForBuyCondition {
        algorithm: "[TÊN]".to_string(),
    })).await?;
}
```

### Bước 4: Thêm Translations

1. **Trong `bot/locales/vi/messages.yml`:**
```yaml
algorithm_[tên]: "📊 [Tên Indicator]"
strategy_algorithm_[tên]_info: |
  📊 <b>[Tên Indicator]</b>
  
  [Mô tả indicator bằng tiếng Việt]
  
  <b>Tham số mặc định:</b>
  - [param1]: [default_value]
  - [param2]: [default_value]
```

2. **Trong `bot/locales/en/messages.yml`:**
```yaml
algorithm_[tên]: "📊 [Indicator Name]"
strategy_algorithm_[tên]_info: |
  📊 <b>[Indicator Name]</b>
  
  [Description in English]
  
  <b>Default parameters:</b>
  - [param1]: [default_value]
  - [param2]: [default_value]
```

3. **Trong `bot/src/i18n/mod.rs` (nếu cần button text):**
```rust
("vi", "algorithm_[tên]") => "📊 [Tên Indicator]".to_string(),
("en", "algorithm_[tên]") => "📊 [Indicator Name]".to_string(),
```

### Bước 5: Cập Nhật Backtest Template (MODULAR SYSTEM)

**✨ Hệ thống mới sử dụng module pattern - chỉ cần implement `IndicatorConfig` trait!**

Thay vì phải sửa nhiều file, giờ chỉ cần thêm một struct implement `IndicatorConfig` trong `bot/src/services/strategy_engine/indicator_configs.rs`:

```rust
/// [Tên Indicator] Indicator Config
pub struct [Tên]Config;

impl IndicatorConfig for [Tên]Config {
    fn name(&self) -> &str {
        "[TÊN]"
    }
    
    fn is_enabled(&self, algorithm: &str) -> bool {
        algorithm.to_uppercase() == "[TÊN]"
    }
    
    fn extract_parameters(&self, params: &Value) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        let period = params
            .get("period")
            .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|v| v as u64)))
            .unwrap_or(default_value) as i32;
        map.insert("period".to_string(), period);
        // Thêm các parameters khác nếu cần
        map
    }
    
    fn parse_entry_condition(&self, buy_condition: &str) -> (bool, Option<i32>) {
        let enabled = buy_condition.to_uppercase().contains("[TÊN]") && buy_condition.contains("<");
        let threshold = if enabled {
            extract_threshold(buy_condition, "[Tên]").or(Some(default_threshold))
        } else {
            None
        };
        (enabled, threshold)
    }
    
    fn parse_exit_condition(&self, sell_condition: &str) -> (bool, Option<i32>) {
        let enabled = sell_condition.to_uppercase().contains("[TÊN]") && sell_condition.contains(">");
        let threshold = if enabled {
            extract_threshold(sell_condition, "[Tên]").or(Some(default_threshold))
        } else {
            None
        };
        (enabled, threshold)
    }
    
    fn generate_indicator_code(&self, params: &HashMap<String, i32>) -> String {
        let period = params.get("period").copied().unwrap_or(default_value);
        format!("dataframe['[tên]'] = ta.[TÊN](dataframe, timeperiod={})", period)
    }
    
    fn generate_entry_code(&self, threshold: Option<i32>) -> Option<String> {
        threshold.map(|t| format!("dataframe['[tên]'] < {}", t))
    }
    
    fn generate_exit_code(&self, threshold: Option<i32>) -> Option<String> {
        threshold.map(|t| format!("dataframe['[tên]'] > {}", t))
    }
}
```

Sau đó đăng ký trong `IndicatorConfigRegistry::new()`:

```rust
registry.register(Box::new([Tên]Config));
```

**✅ Ưu điểm của hệ thống mới:**
- ✅ Mỗi indicator tự quản lý config của mình
- ✅ Không cần sửa hàm chung khi thêm indicator mới
- ✅ Code tự động generate Python template
- ✅ Dễ maintain và scale khi có nhiều indicators

## Ví Dụ: Thêm Stochastic Indicator

Xem file `bot/src/services/strategy_engine/implementations.rs` để xem implementation của `StochasticStrategy`.

## Ví Dụ: Thêm ADX Indicator

Xem file `bot/src/services/strategy_engine/implementations.rs` để xem implementation của `AdxStrategy`.

## Lưu Ý

1. **Sử dụng thư viện `ta`:** Hầu hết indicators có sẵn trong crate `ta`. Kiểm tra [ta documentation](https://docs.rs/ta/) để xem indicators có sẵn.

2. **Parameters:** Extract parameters từ `config.parameters` (JSON Value) và có default values hợp lý.

3. **Condition Parsing:** Sử dụng hàm `parse_condition()` để parse buy/sell conditions từ string (ví dụ: "RSI < 30").

4. **Testing:** Test indicator với dữ liệu thực tế trước khi deploy.

5. **Documentation:** Cập nhật file này khi thêm indicators mới để người khác có thể tham khảo.

