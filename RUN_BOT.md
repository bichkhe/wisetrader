# Cách chạy bot để test

## 1. Cấu hình environment

Tạo file `.env`:
```bash
BOT_TOKEN=your_telegram_bot_token_from_BotFather
DATABASE_URL=mysql://wisetrader:wisetrader2025@localhost:3306/wisetrader_db
REDIS_URL=redis://localhost:6379
```

## 2. Khởi động infrastructure

```bash
docker-compose up -d
```

## 3. Chạy bot

```bash
cargo run --bin bot
```

## 4. Test trên Telegram

1. Tìm bot của bạn trên Telegram
2. Gửi các lệnh sau:
   - `/start` - Đăng ký
   - `/help` - Xem lệnh
   - `/strategies` - Danh sách chiến thuật
   - `/subscription` - Thông tin gói

## Status

- ✅ Bot compiles successfully
- ✅ MySQL connected
- ✅ Redis ready
- ✅ Freqtrade API available
- ⏳ Strategy creation commands (simplified for now)

## Commands Available

- `/start` - Đăng ký user
- `/help` - Hiển thị help
- `/subscription` - Xem gói đăng ký
- `/strategies` - Danh sách chiến thuật
- `/mystrategies` - Chiến thuật của bạn

