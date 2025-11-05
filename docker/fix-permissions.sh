#!/bin/bash
# Helper script to fix permissions for /app/strategies
# This script should be called with sudo or run as root

set -e

echo "Fixing permissions for /app/strategies..."

# Ensure directory exists
mkdir -p /app/strategies

# Fix ownership
chown -R appuser:appuser /app/strategies || true

# Fix permissions
chmod -R 755 /app/strategies || true

# Verify
if [ -d /app/strategies ]; then
    echo "✅ Permissions fixed: $(ls -ld /app/strategies | awk '{print $1, $3, $4}')"
    # Test write
    if touch /app/strategies/.test_write 2>/dev/null; then
        rm -f /app/strategies/.test_write
        echo "✅ Write permission verified"
        exit 0
    else
        echo "⚠️  Warning: Write permission still failed, trying chmod 777..."
        chmod -R 777 /app/strategies || true
        exit 0
    fi
else
    echo "❌ Directory /app/strategies does not exist"
    exit 1
fi

