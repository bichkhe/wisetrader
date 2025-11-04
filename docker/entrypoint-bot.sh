#!/bin/bash
set -e

# Check if we're being re-executed as appuser (after fixing permissions)
if [ "$SKIP_PERMISSIONS_FIX" = "1" ]; then
    # We're already running as appuser, skip permission fix
    :
else
    # Running as root, fix permissions first
    # Ensure strategies directory exists and has correct permissions
    echo "Setting up /app/strategies directory..."
    mkdir -p /app/strategies || true
    echo "Fixing permissions for /app/strategies..."
    chown -R appuser:appuser /app/strategies || true
    chmod -R 755 /app/strategies || true
    # Verify permissions
    if [ -d /app/strategies ]; then
        echo "✅ /app/strategies permissions: $(ls -ld /app/strategies | awk '{print $1, $3, $4}')"
        # Test write permission
        if touch /app/strategies/.test_write 2>/dev/null; then
            rm -f /app/strategies/.test_write
            echo "✅ Write permission test passed for /app/strategies"
        else
            echo "⚠️  Warning: Write permission test failed for /app/strategies"
            # Try to fix again
            chown -R appuser:appuser /app/strategies || true
            chmod -R 777 /app/strategies || true
        fi
    else
        echo "⚠️  Warning: /app/strategies directory does not exist"
    fi
    
    # Fix permissions for html_reports directory
    if [ -d /app/html_reports ]; then
        echo "Fixing permissions for /app/html_reports..."
        chown -R appuser:appuser /app/html_reports || true
        chmod -R 755 /app/html_reports || true
    fi
    
    # Fix Docker socket permissions
    if [ -S /var/run/docker.sock ]; then
        echo "Fixing Docker socket permissions..."
        # Get docker group GID from host socket
        DOCKER_GID=$(stat -c '%g' /var/run/docker.sock 2>/dev/null || stat -f '%g' /var/run/docker.sock 2>/dev/null || echo "999")
        echo "Docker socket GID: $DOCKER_GID"
        # Create docker group with same GID as host
        groupadd -g "$DOCKER_GID" docker 2>/dev/null || \
        (groupmod -g "$DOCKER_GID" docker 2>/dev/null || true)
        # Add appuser to docker group
        usermod -aG docker appuser 2>/dev/null || true
        # Fix socket permissions (ensure docker group can access)
        chmod 666 /var/run/docker.sock 2>/dev/null || true
        chown root:docker /var/run/docker.sock 2>/dev/null || true
        echo "Docker socket permissions fixed"
    fi
    
    # Switch to appuser and re-exec this script
    if [ "$(id -u)" = "0" ]; then
        echo "Switching to appuser for bot execution..."
        export SKIP_PERMISSIONS_FIX=1
        exec gosu appuser "$0" "$@"
    fi
fi

echo "=== Bot Container Starting ==="
echo "Running as user: $(whoami) (uid: $(id -u), gid: $(id -g))"
echo "Groups: $(groups)"
echo "Environment Variables:"
echo "BOT_TOKEN: ${BOT_TOKEN:0:10}..."
echo "BOT_NAME: $BOT_NAME"
echo "DATABASE_URL: $DATABASE_URL"
echo "REDIS_URL: $REDIS_URL"
echo "API_BASE_URL: $API_BASE_URL"
echo "RUST_LOG: $RUST_LOG"
echo "GENERATE_HTML_REPORTS: $GENERATE_HTML_REPORTS"
echo "STRATEGIES_PATH: ${STRATEGIES_PATH:-/app/strategies}"
echo ""
echo "=== Directory Permissions ==="
if [ -d /app/strategies ]; then
    echo "/app/strategies: $(ls -ld /app/strategies | awk '{print $1, $3, $4, $9}')"
    echo "Can write to /app/strategies: $(touch /app/strategies/.test 2>&1 && echo 'YES' && rm -f /app/strategies/.test || echo 'NO')"
else
    echo "/app/strategies: DOES NOT EXIST"
fi
if [ -d /app/html_reports ]; then
    echo "/app/html_reports: $(ls -ld /app/html_reports | awk '{print $1, $3, $4, $9}')"
fi
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
