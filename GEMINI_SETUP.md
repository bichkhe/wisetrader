# Gemini AI Integration Setup

Hệ thống đã được tích hợp với Google Gemini AI để tự động phân tích kết quả backtest.

## Cấu hình

### 1. Thêm API Key vào file `.env`

Thêm các dòng sau vào file `.env` của bạn:

```bash
# Gemini AI Configuration
GEMINI_API_KEY=AIzaSyCfEVe6e96bLcmh1xw10IeG00NvljTjpzE
ENABLE_GEMINI_ANALYSIS=true
```

### 2. Giải thích các biến môi trường

- **`GEMINI_API_KEY`**: API key của Google Gemini (bắt buộc nếu muốn sử dụng AI analysis)
- **`ENABLE_GEMINI_ANALYSIS`**: Bật/tắt tính năng AI analysis (mặc định: `true`)

### 3. Cách hoạt động

Khi chạy backtest:

1. Hệ thống sẽ tự động gọi Gemini API để phân tích kết quả
2. Phân tích bao gồm:
   - Đánh giá tổng quan về strategy
   - Điểm mạnh và điểm yếu
   - Khuyến nghị tối ưu
   - Phân tích rủi ro
   - Kết luận

3. Kết quả phân tích sẽ được:
   - Hiển thị trong HTML report (nếu có)
   - Tự động theo ngôn ngữ của user (tiếng Việt hoặc tiếng Anh)

### 4. Xem kết quả

Sau khi backtest hoàn thành, mở HTML report để xem phần **"🤖 AI Analysis (Powered by Gemini)"**.

## Lưu ý

- Nếu không có API key hoặc `ENABLE_GEMINI_ANALYSIS=false`, hệ thống vẫn chạy bình thường nhưng không có AI analysis
- API key được lưu trong biến môi trường, không hardcode trong code
- Phân tích được generate tự động, không cần thao tác thủ công

## Troubleshooting

Nếu gặp lỗi khi gọi Gemini API:

1. Kiểm tra API key có đúng không
2. Kiểm tra kết nối internet
3. Xem logs để biết chi tiết lỗi:
   ```
   ⚠️ Failed to generate Gemini AI analysis: [error message]
   ```

