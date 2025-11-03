# Hướng dẫn thiết lập Webhook Mode cho WiseTrader Bot

## Tổng quan

Bot hiện hỗ trợ 2 chế độ:
1. **Polling Mode** (mặc định): Bot tự động lấy updates từ Telegram
2. **Webhook Mode**: Telegram gửi updates đến bot qua HTTPS

## Cách bật Webhook Mode

### 1. Thiết lập biến môi trường

Thêm vào file `.env`:

```bash
WEBHOOK_URL=https://yourdomain.com  # URL công khai có HTTPS
WEBHOOK_PATH=/webhook                # Đường dẫn webhook (mặc định: /webhook)
WEBHOOK_PORT=8443                    # Port lắng nghe (mặc định: 8443)
```

**Lưu ý:**
- `WEBHOOK_URL` phải là HTTPS (Telegram yêu cầu)
- URL phải có thể truy cập công khai từ internet
- Port có thể là bất kỳ (mặc định 8443)

### 2. Cấu hình SSL Certificate

Webhook yêu cầu HTTPS. Có các lựa chọn:

#### Option A: Sử dụng domain có SSL (khuyến nghị cho production)
- Cài đặt reverse proxy (nginx/caddy) với Let's Encrypt
- Proxy requests đến bot server

#### Option B: Sử dụng ngrok (cho development)
```bash
ngrok http 8443
```
Sau đó dùng URL ngrok làm `WEBHOOK_URL`:
```bash
WEBHOOK_URL=https://abc123.ngrok.io
```

### 3. Chạy bot

Khi `WEBHOOK_URL` được set, bot sẽ tự động chuyển sang webhook mode:

```bash
cargo run --bin bot
```

Bot sẽ:
1. Xóa webhook cũ (nếu có)
2. Set webhook mới với Telegram
3. Khởi động HTTP server để nhận updates
4. Tự động xử lý các updates từ Telegram

### 4. Kiểm tra webhook

Bot sẽ log thông tin khi khởi động:
```
🌐 Starting bot in WEBHOOK mode
📡 Webhook URL: https://yourdomain.com
🔗 Webhook path: /webhook
🔌 Listening on port: 8443
🧹 Old webhook deleted
✅ Webhook set: https://yourdomain.com/webhook
🚀 Starting webhook server on 0.0.0.0:8443
🌐 Webhook HTTP server listening on 0.0.0.0:8443
```

## Fallback về Polling Mode

Nếu `WEBHOOK_URL` không được set, bot sẽ tự động dùng Polling Mode:
```
📡 Webhook URL not set, using POLLING mode
💡 To use webhook mode, set WEBHOOK_URL environment variable
```

## Lợi ích của Webhook Mode

1. **Không có timeout errors**: Không còn lỗi `TimedOut` như polling
2. **Nhanh hơn**: Updates được push ngay khi có
3. **Tiết kiệm tài nguyên**: Không cần liên tục polling
4. **Production-ready**: Phù hợp cho môi trường production

## Troubleshooting

### Lỗi "Webhook was not verified"
- Kiểm tra URL có đúng HTTPS không
- Kiểm tra port có mở firewall không
- Kiểm tra SSL certificate có hợp lệ không

### Bot không nhận được updates
- Kiểm tra webhook đã được set: `curl https://api.telegram.org/bot<TOKEN>/getWebhookInfo`
- Kiểm tra server có đang chạy không
- Kiểm tra logs của bot

### Muốn quay lại Polling Mode
Chỉ cần xóa hoặc comment `WEBHOOK_URL` trong `.env`

