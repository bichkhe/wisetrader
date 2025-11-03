#!/bin/bash
set -e

echo "=== Bot Container Starting ==="
echo "Environment Variables:"
echo "BOT_TOKEN: ${BOT_TOKEN:0:10}..."
echo "BOT_NAME: $BOT_NAME"
echo "DATABASE_URL: $DATABASE_URL"
echo "REDIS_URL: $REDIS_URL"
echo "API_BASE_URL: $API_BASE_URL"
echo "RUST_LOG: $RUST_LOG"
echo "============================"

# Check if binary exists and is executable
if [ ! -f /app/bot ]; then
    echo "ERROR: /app/bot binary not found!"
    exit 1
fi

if [ ! -x /app/bot ]; then
    echo "ERROR: /app/bot is not executable!"
    exit 1
fi

echo "Starting bot..."
exec /app/bot "$@"
