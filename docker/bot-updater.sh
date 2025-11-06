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
# Use /tmp for installation to avoid read-only filesystem issues
if ! command -v docker >/dev/null 2>&1; then
    echo "Installing Docker CLI..."
    ARCH=$(uname -m)
    if [ "$ARCH" = "x86_64" ]; then
        DOCKER_ARCH="x86_64"
    elif [ "$ARCH" = "aarch64" ]; then
        DOCKER_ARCH="aarch64"
    else
        DOCKER_ARCH="x86_64"
    fi
    
    # Install docker CLI to /tmp (writable location)
    mkdir -p /tmp/docker-bin
    wget -q -O /tmp/docker.tgz "https://download.docker.com/linux/static/stable/${DOCKER_ARCH}/docker-24.0.7.tgz" && \
    tar -xzC /tmp/docker-bin -f /tmp/docker.tgz && \
    chmod +x /tmp/docker-bin/docker/docker && \
    rm -f /tmp/docker.tgz
    
    # Install docker compose to /tmp
    mkdir -p /tmp/docker-bin/docker/cli-plugins
    wget -q -O /tmp/docker-bin/docker/cli-plugins/docker-compose \
        "https://github.com/docker/compose/releases/download/v2.24.0/docker-compose-linux-${DOCKER_ARCH}" && \
    chmod +x /tmp/docker-bin/docker/cli-plugins/docker-compose
    
    # Add to PATH
    export PATH="/tmp/docker-bin/docker:${PATH}"
    export DOCKER_CONFIG="/tmp/docker-bin"
fi

# Function to get host user UID from mounted directory
get_host_uid() {
    # Try to get UID from .git directory or work directory ownership
    stat -c '%u' "${WORK_DIR}/.git" 2>/dev/null || \
    stat -c '%u' "${WORK_DIR}" 2>/dev/null || \
    echo "1000"
}

# Configure sudo to allow running commands without password (if running as root)
configure_sudo() {
    if [ "$(id -u)" = "0" ] && command -v sudo >/dev/null 2>&1; then
        # Configure sudo to allow root to run commands as any user without password
        if [ ! -f /etc/sudoers.d/bot-updater ]; then
            echo "root ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/bot-updater 2>/dev/null || true
            chmod 0440 /etc/sudoers.d/bot-updater 2>/dev/null || true
        fi
    fi
}

# Function to run script with host user permissions
run_as_host_user() {
    local script_path="$1"
    local host_uid=$(get_host_uid)
    
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Running script as host user (UID: ${host_uid})..."
    
    # Configure sudo if needed
    configure_sudo
    
    # Try multiple methods to run as host user
    # Method 1: Find existing user with this UID
    EXISTING_USER=$(getent passwd "${host_uid}" 2>/dev/null | cut -d: -f1 || echo "")
    if [ -n "${EXISTING_USER}" ]; then
        sudo -u "${EXISTING_USER}" bash "${script_path}" 2>/dev/null && return 0
    fi
    
    # Method 2: Try runuser with UID
    if command -v runuser >/dev/null 2>&1; then
        runuser -u "#${host_uid}" -- bash "${script_path}" 2>/dev/null && return 0
    fi
    
    # Method 3: Try sudo with UID directly
    sudo -u "#${host_uid}" bash "${script_path}" 2>/dev/null && return 0
    
    # Method 4: Fallback - run directly (may have permission issues)
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] WARNING: Could not run as host user, running directly..."
    bash "${script_path}"
}

# Function to check and deploy
deploy_bot() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Release file changed, deploying bot..."
    
    # Check if custom deploy script exists
    DEPLOY_SCRIPT="${WORK_DIR}/deploy.sh"
    if [ -f "${DEPLOY_SCRIPT}" ] && [ -x "${DEPLOY_SCRIPT}" ]; then
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Found custom deploy script: ${DEPLOY_SCRIPT}"
        run_as_host_user "${DEPLOY_SCRIPT}" || {
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Custom deploy script failed"
            return 1
        }
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] Bot deployed successfully using custom script!"
        return 0
    fi
    
    # Default deployment logic
    cd "${WORK_DIR}"
    
    # Configure git safe.directory to avoid ownership issues
    # Use local config instead of global to avoid read-only filesystem issues
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Configuring git safe.directory..."
    git config --local --add safe.directory "${WORK_DIR}" 2>/dev/null || \
    git config --global --add safe.directory "${WORK_DIR}" 2>/dev/null || true
    
    # Git pull
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Running git pull..."
    git pull origin || {
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Git pull failed"
        return 1
    }
    
    # Docker compose up
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Running docker compose up -d bot --build..."
    # Use full path to docker if installed in /tmp
    if [ -f "/tmp/docker-bin/docker/docker" ]; then
        /tmp/docker-bin/docker/docker compose -f "${WORK_DIR}/docker-compose.yml" --project-directory "${WORK_DIR}" up -d bot --build || {
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Docker compose failed"
            return 1
        }
    else
        docker compose -f "${WORK_DIR}/docker-compose.yml" --project-directory "${WORK_DIR}" up -d bot --build || {
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: Docker compose failed"
            return 1
        }
    fi
    
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

