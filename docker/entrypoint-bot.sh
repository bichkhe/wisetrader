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

# Wait for database to be ready
echo "Waiting for database to be ready..."
max_retries=30
retry_count=0

# Extract DB host and port from DATABASE_URL
# Format: mysql://user:pass@host:port/dbname
DB_HOST=$(echo $DATABASE_URL | sed -n 's/.*@\([^:]*\):.*/\1/p')
DB_PORT=$(echo $DATABASE_URL | sed -n 's/.*:\([0-9]*\)\/.*/\1/p')

echo "Checking database connection to $DB_HOST:$DB_PORT..."

while ! nc -z $DB_HOST $DB_PORT; do
    retry_count=$((retry_count + 1))
    if [ $retry_count -ge $max_retries ]; then
        echo "ERROR: Database is not ready after $max_retries attempts"
        exit 1
    fi
    echo "Database not ready yet (attempt $retry_count/$max_retries)... waiting"
    sleep 2
done

echo "✅ Database is ready!"

# Run migrations
if [ -f /app/migration ] && [ -x /app/migration ]; then
    echo "Running database migrations..."
    /app/migration up -u "$DATABASE_URL" 2>&1 | grep -v "Warning" || echo "✅ Migrations completed or already applied!"
else
    echo "⚠️  Migration binary not found, skipping migrations"
fi

# Check if bot binary exists and is executable
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
