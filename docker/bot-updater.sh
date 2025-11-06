#!/bin/sh
set -e

WORK_DIR=${DEPLOY_WORK_DIR:-/opt/wisetrader/wisetrader}
RELEASE_FILE="${WORK_DIR}/bot/RELEASE.md"
CHECK_INTERVAL=${CHECK_INTERVAL:-30}  # Check every 30 seconds by default

echo "=== Bot Updater Starting ==="
echo "Work directory: ${WORK_DIR}"
echo "Release file: ${RELEASE_FILE}"
echo "Check interval: ${CHECK_INTERVAL} seconds"
echo ""

# Install docker compose if not available
if ! command -v docker >/dev/null 2>&1; then
    echo "Installing Docker CLI..."
    apk add --no-cache docker-cli docker-compose >/dev/null 2>&1 || {
        # Try alternative installation method
        ARCH=$(uname -m)
        if [ "$ARCH" = "x86_64" ]; then
            DOCKER_ARCH="x86_64"
        elif [ "$ARCH" = "aarch64" ]; then
            DOCKER_ARCH="aarch64"
        else
            DOCKER_ARCH="x86_64"
        fi
        
        # Install docker CLI
        wget -q -O /tmp/docker.tgz "https://download.docker.com/linux/static/stable/${DOCKER_ARCH}/docker-24.0.7.tgz" && \
        tar -xzC /tmp -f /tmp/docker.tgz && \
        mv /tmp/docker/docker /usr/local/bin/docker && \
        chmod +x /usr/local/bin/docker && \
        rm -rf /tmp/docker /tmp/docker.tgz
        
        # Install docker compose
        mkdir -p /usr/local/lib/docker/cli-plugins
        wget -q -O /usr/local/lib/docker/cli-plugins/docker-compose \
            "https://github.com/docker/compose/releases/download/v2.24.0/docker-compose-linux-${DOCKER_ARCH}" && \
        chmod +x /usr/local/lib/docker/cli-plugins/docker-compose
    fi
fi

# Function to check and deploy
deploy_bot() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Release file changed, deploying bot..."
    
    cd "${WORK_DIR}"
    
    # Configure git safe.directory to avoid ownership issues
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Configuring git safe.directory..."
    git config --global --add safe.directory "${WORK_DIR}" 2>/dev/null || true
    
    # Git pull
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Running git pull..."
    git pull origin || {
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Git pull failed"
        return 1
    }
    
    # Docker compose up
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Running docker compose up -d bot --build..."
    docker compose -f "${WORK_DIR}/docker-compose.yml" --project-directory "${WORK_DIR}" up -d bot --build || {
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Docker compose failed"
        return 1
    }
    
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Bot deployed successfully!"
}

# Get initial file hash
if [ -f "${RELEASE_FILE}" ]; then
    LAST_HASH=$(md5sum "${RELEASE_FILE}" 2>/dev/null | cut -d' ' -f1 || echo "")
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Initial release file hash: ${LAST_HASH}"
else
    LAST_HASH=""
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Release file not found, waiting for it to be created..."
fi

# Main loop - check file periodically
while true; do
    sleep "${CHECK_INTERVAL}"
    
    if [ ! -f "${RELEASE_FILE}" ]; then
        # File doesn't exist, wait for it
        if [ -n "${LAST_HASH}" ]; then
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Release file removed, resetting hash..."
            LAST_HASH=""
        fi
        continue
    fi
    
    # Calculate current hash
    CURRENT_HASH=$(md5sum "${RELEASE_FILE}" 2>/dev/null | cut -d' ' -f1 || echo "")
    
    if [ -z "${CURRENT_HASH}" ]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] WARNING: Failed to calculate hash, skipping check..."
        continue
    fi
    
    # Check if file changed
    if [ "${CURRENT_HASH}" != "${LAST_HASH}" ]; then
        if [ -n "${LAST_HASH}" ]; then
            # File changed (not first time)
            deploy_bot
        else
            # First time seeing the file
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] Release file detected, hash: ${CURRENT_HASH}"
        fi
        LAST_HASH="${CURRENT_HASH}"
    fi
done

