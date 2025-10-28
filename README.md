# wisetrader
# Định dạng tóm tắt & roadmap dự án bot trading Telegram (Binance/OKX) sử dụng Rust (teloxide) & freqtrade

## Mục tiêu
- Xây dựng hệ thống bot Telegram gửi tín hiệu trading các sàn Binance, OKX tích hợp với freqtrade (backtest, hyperopt, AI).
- Nền tảng sử dụng Rust (teloxide) cho bot, hỗ trợ nhiều user đồng thời (vài trăm người).
- Cho phép user: nhận tín hiệu, bật live trading, chọn chiến lược/thuật toán, tùy chỉnh risk, thanh toán/subscription.
- Hỗ trợ quản trị nhiều user, đa gói subscription, tích hợp thanh toán (Stripe/PayPal hoặc gateway nội địa).

---

## 1. Tổng quan chức năng (Functional Requirements)

### User-facing
- Đăng ký/đăng nhập qua Telegram OAuth.
- Quản lý subscription (Free trial, Basic, Pro, VIP), tự động gia hạn/hết hạn qua webhook.
- Nhận tín hiệu qua channel hoặc private (1:n).
- Bật/tắt live trading (Yêu cầu opt-in, KYC nếu cần).
- Chọn/tuỳ biến chiến lược (dùng sẵn hoặc upload config freqtrade).
- Tùy chỉnh các tham số risk (position size, stoploss, takeprofit, số lệnh mở tối đa).
- Xem lịch sử tín hiệu, trạng thái lệnh, P&L cá nhân.

### Backend/admin
- Scheduler/dispatcher tiếp nhận tín hiệu freqtrade, gửi bot, (tuỳ chọn) đặt lệnh lên exchange.
- Kết nối freqtrade: chạy backtest/hyperopt, lấy kết quả/report.
- Service đặt lệnh (đảm bảo an toàn, idempotent, log replay-safe).
- Cấu hình riêng theo từng user/group.
- Dashboard quản trị: quản lý user, sub, thống kê metric/logs, thao tác thủ công.
- Quản lý repo chiến lược: thêm/xóa/version.

### Payment/safety
- Tích hợp/invoice/refund qua Stripe hoặc gateway.
- Xác nhận opt-in rõ ràng cho live trading, TOS, khuyến cáo rủi ro.
- Lưu khóa API mã hóa, hướng dẫn tạo API (không quyền rút tiền), rate-limit, log/audit.

---

## 2. Yêu cầu phi chức năng (Non-functional)
- Đồng thời: xử lý vài trăm user; hàng trăm lệnh/giờ.
- Độ trễ thấp: tín hiệu → đặt lệnh càng nhanh càng tốt (REST/WS tối ưu).
- Tin cậy: retry/redo logic, order transactional, log bền vững.
- Bảo mật: secret không lưu trong repo, mã hóa API key, 2FA cho admin.
- Quan sát: Prometheus metrics, trace OpenTelemetry, log chuẩn JSON.
- Triển khai: Docker hóa, Kubernetes/Helm cho prod, Compose dev.
- Hỗ trợ scale ngang: stateless service tách biệt DB/Redis.

---

## 3. Tech Stack đề xuất

- Bot: Rust + teloxide
- Backend microservices: Rust (Actix-Web/Axum)
- Task queue: Redis Stream (+ Rust consumer) hoặc RabbitMQ
- Database: MYSQL (chính), Redis (cache, rate-limit, state tạm)
- Strategy engine: freqtrade (Python, container/gRPC/HTTP)
- Kết nối order: Rust SDK hoặc wrap REST/WS cho Binance/OKX
- Container: Docker, orchestration K8s/Compose
- Payment: Stripe hoặc cổng VN, webhook integration
- Monitor: Prometheus, Grafana, Loki logs
- CI/CD: GitHub Actions hoặc GitLab CI
- Secret: Vault/AWS/K8s Secrets

---

## 4. Kiến trúc tổng quan (Data Flow)

```
User (Telegram) <-> Bot (teloxide) <-> Redis/Postgres <-> Task Queue (Redis Stream)
                          |                       |
                <--- freqtrade-adapter --->
                          |
                [Backtest/Live signals]
                          |
                Signal Dispatcher <-> Bot: gửi tín hiệu
                          |
                (nếu bật live) -> Order Executor -> Gọi API sàn -> Log
                          |
                All data persist: Postgres (orders, signals, trades), Prometheus (metrics)
```

---

## 5. Roadmap và task (theo milestone/phong cách checklist)

### **Milestone 0: Project setup & PoC**
- [ ] **0.1**: Tạo monorepo `/trading-bot/` (bot, backend, worker, freqtrade-adapter, infra).
  - Chạy CI lint/check Rust & Python.
- [ ] **0.2**: Bot teloxide hello world (bắt /start, /help, lưu user id vào Postgres).
- [ ] **0.3**: Tích hợp freqtrade container (dev), adapter gọi HTTP `/api/backtest`, nhận JSON report.

### **Milestone 1: Tín hiệu & UI**
- [ ] **1.1**: Redis Stream topic signals, worker phát tín hiệu qua bot cho đăng ký.
- [ ] **1.2**: DB schema (chiến lược, user_strategies, user_configs); bot: /add_strategy, /set_param, /list_strategies.
- [ ] **1.3**: Giao diện message giàu (InlineKeyboard: Đặt lệnh/Bỏ qua/Xem backtest), callback handler.

### **Milestone 2: Đặt lệnh & Risk**
- [ ] **2.1**: Kết nối sàn (Binance/OKX, idempotent, retry, rate-limit), testnet.
- [ ] **2.2**: Check policy, risk-engine (giới hạn số lệnh, size, circuit breaker).
- [ ] **2.3**: Log/audit, reconciliation job so sánh fill sàn vs DB.

### **Milestone 3: Payments & Access control**
- [ ] **3.1**: Tích hợp thanh toán (Stripe/gateway), webhook, quản lý sub/tier.
- [ ] **3.2**: Bảo vệ tính năng theo tier, block command nếu không đủ quyền.

### **Milestone 4: Scaling, quan sát, bảo mật**
- [ ] **4.1**: K8s manifest/HPA, chạy 3 replica bot/worker, test tải.
- [ ] **4.2**: Prometheus metric, Grafana dashboard, alert lỗi/tắc hàng đợi.
- [ ] **4.3**: Vault/K8s secrets, xoay API key, encrypt DB.

### **Milestone 5: Advanced**
- [ ] User tự upload chiến lược freqtrade (container riêng), leaderboards/copy trade, auto-hyperopt (premium), AI suggest.

---

## 6. Checklist bảo mật & pháp lý
- [ ] Opt-in/manual xác nhận live trading, chấp nhận TOS/disclaimer.
- [ ] API key mã hóa, user tự tạo & không có quyền rút tiền.
- [ ] Rate-limit user, giữ log/audit X ngày.
- [ ] Chèn cảnh báo pháp lý, khai báo rủi ro.

---

## 7. Checklist deploy/dev
- [ ] Docker images cho: bot, api, worker, freqtrade-adapter, freqtrade
- [ ] Compose dev: Postgres, Redis, MinIO (optional), Prometheus, Grafana
- [ ] K8s/Helm chart
- [ ] CI: test/build/push
- [ ] CD: Argo/Flux hoặc GitHub Actions deploy lên K8s

---

## 8. 3 bước tiếp theo đề xuất (quick win)

1. Tạo skeleton repo + CI (Task 0.1)
2. Triển khai bot teloxide + Postgres (Task 0.2)
3. Chạy freqtrade container, test backtest (Task 0.3)

---

## 9. Tips & lưu ý

- Luôn test live trading ở testnet (Binance testnet, OKX demo).
- Tuyệt đối không cấp quyền rút trên API key của user.
- Chạy chiến lược user upload trong container sandbox.
- Scale pool worker (không worker đơn lẻ chạy code mọi user).

---

## 10. Project Setup & Quick Start

### Prerequisites

- Rust 1.70+
- Docker & Docker Compose
- MySQL 8.0
- Redis 7+
- Telegram Bot Token (from @BotFather)

### Quick Start

1. **Clone the repository:**
```bash
git clone <repo-url>
cd wisetrader
```

2. **Set up environment:**
```bash
cp .env.example .env
# Edit .env and add your BOT_TOKEN
```

3. **Start infrastructure:**
```bash
docker-compose up -d
```

This will start MySQL and Redis containers with the necessary database schema and seed data.

4. **Build and run the bot:**
```bash
cargo build --release
cargo run --bin bot
```

5. **Test the bot on Telegram:**
- Find your bot on Telegram
- Send `/start` to register
- Send `/help` to see available commands
- Send `/strategies` to view trading strategies
- Send `/subscription` to check your plan

### Development

Run individual services:

```bash
# Start only the bot
cargo run --bin bot

# Start the API server
cargo run --bin api

# Start signal dispatcher worker
cargo run --bin signal_dispatcher

# Start order executor worker
cargo run --bin order_executor

# Start freqtrade adapter
cargo run --bin freqtrade_adapter
```

### Database Migrations

The database schema is automatically created when you run `docker-compose up`. The schema includes:
- Users with subscription tiers
- Trading strategies
- User strategy subscriptions
- Signal history
- Order tracking
- Billing plans and invoices

### Current Implementation Status

✅ **Completed (Phase 1):**
- Rust workspace with bot, api, shared crates
- MySQL database schema and migrations
- Docker Compose setup
- Basic teloxide bot with commands:
  - `/start` - User registration with free trial
  - `/help` - Show commands
  - `/subscription` - View subscription status
  - `/strategies` - List available strategies
  - `/my_strategies` - View user's active strategies
- Subscription tier system (Free Trial, Basic, Pro)
- Strategy management foundation

🚧 **In Progress (Phase 2):**
- Freqtrade integration
- Signal distribution system
- Payment integration
- Exchange connectivity (Binance/OKX)
- Live trading execution

### Architecture

```
┌─────────────────┐
│  Telegram User  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Bot (Rust)    │◄──┐
│   (teloxide)    │   │
└────────┬────────┘   │
         │            │
         ▼            │
┌─────────────────┐   │
│  API Backend    │   │
│   (Axum)        │   │
└────────┬────────┘   │
         │            │
         ▼            │
┌─────────────────┐   │
│   MySQL DB      │   │
└─────────────────┘   │
                      │
         ┌────────────┘
         │
         ▼
┌─────────────────┐
│ Redis Streams   │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌─────────┐ ┌─────────────┐
│ Signal  │ │   Order     │
│Dispatcher│ │   Executor  │
└─────────┘ └─────────────┘
```

### Next Steps

1. Add freqtrade container integration
2. Implement signal dispatcher worker
3. Add inline keyboard for signal actions
4. Implement payment webhook handlers
5. Connect to Binance testnet and OKX demo
6. Add monitoring with Prometheus/Grafana

---
