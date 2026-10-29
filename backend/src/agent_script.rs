pub fn generate_bash_script(
    server_url: &str,
    machine_name: &str,
    setup_token: &str,
    tunnel_token: &str,
    tunnel_id: &str,
) -> String {
    format!(r#"#!/bin/bash
# ==============================================================================
# Cloudflare Tunnel Auto-Provisioner
# Machine: {machine_name}
# ==============================================================================
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${{CYAN}}====================================================================${{NC}}"
echo -e "${{CYAN}}        Cloudflare Tunnel Node Auto-Provisioner                     ${{NC}}"
echo -e "${{CYAN}}====================================================================${{NC}}"
echo -e "${{BLUE}}[*] Target Machine:${{NC}} {machine_name}"
echo -e "${{BLUE}}[*] Dashboard Host:${{NC}} {server_url}"

# Check for root
if [ "$EUID" -ne 0 ]; then
  echo -e "${{RED}}[!] Please run as root or with sudo:${{NC}}"
  echo -e "    curl -fsSL ... | sudo bash"
  exit 1
fi

# Detect Architecture
ARCH=$(uname -m)
CF_ARCH="amd64"

case "$ARCH" in
  x86_64)
    CF_ARCH="amd64"
    ;;
  aarch64|arm64)
    CF_ARCH="arm64"
    ;;
  armv7l|armhf)
    CF_ARCH="arm"
    ;;
  i386|i686)
    CF_ARCH="386"
    ;;
  *)
    echo -e "${{YELLOW}}[!] Unknown architecture $ARCH, defaulting to amd64${{NC}}"
    CF_ARCH="amd64"
    ;;
esac

echo -e "${{BLUE}}[*] Detected architecture:${{NC}} $ARCH (binary: $CF_ARCH)"

# Ensure curl is installed
if ! command -v curl &> /dev/null; then
  echo -e "${{YELLOW}}[*] Installing curl...${{NC}}"
  if command -v apt-get &> /dev/null; then
    apt-get update -qq && apt-get install -y -qq curl
  elif command -v yum &> /dev/null; then
    yum install -y curl
  elif command -v dnf &> /dev/null; then
    dnf install -y curl
  elif command -v pacman &> /dev/null; then
    pacman -Sy --noconfirm curl
  fi
fi

# Download cloudflared if not present or needs update
CLOUDFLARED_BIN="/usr/local/bin/cloudflared"
echo -e "${{BLUE}}[*] Downloading latest cloudflared binary for $CF_ARCH...${{NC}}"
DOWNLOAD_URL="https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-$CF_ARCH"

curl -fsSL -o "$CLOUDFLARED_BIN" "$DOWNLOAD_URL"
chmod +x "$CLOUDFLARED_BIN"

CF_VERSION=$("$CLOUDFLARED_BIN" --version | head -n 1)
echo -e "${{GREEN}}[✓] Installed: $CF_VERSION${{NC}}"

# Stop any existing cloudflared service cleanly before reconfiguring
if systemctl is-active --quiet cloudflared; then
  echo -e "${{YELLOW}}[*] Stopping existing cloudflared service...${{NC}}"
  systemctl stop cloudflared || true
fi

# Install systemd service with Cloudflare Tunnel Token
echo -e "${{BLUE}}[*] Installing cloudflared service with remote management token...${{NC}}"
"$CLOUDFLARED_BIN" service uninstall &>/dev/null || true
"$CLOUDFLARED_BIN" service install "{tunnel_token}"

# Enable and start service
echo -e "${{BLUE}}[*] Starting cloudflared service...${{NC}}"
systemctl daemon-reload
systemctl enable cloudflared
systemctl restart cloudflared

sleep 2

# Verify service status
if systemctl is-active --quiet cloudflared; then
  echo -e "${{GREEN}}[✓] Cloudflared service is ACTIVE and running!${{NC}}"
else
  echo -e "${{RED}}[!] Cloudflared service failed to start. Logs:${{NC}}"
  journalctl -u cloudflared -n 20 --no-pager
  exit 1
fi

# Collect System Info for Registration
HOST_NAME=$(hostname)
OS_INFO="Linux"
if [ -f /etc/os-release ]; then
  . /etc/os-release
  OS_INFO="$PRETTY_NAME"
fi
PUBLIC_IP=$(curl -s -m 3 https://api.ipify.org || hostname -I | awk '{{print $1}}' || echo "Unknown")

# Register with Dashboard
echo -e "${{BLUE}}[*] Registering node status with Dashboard...${{NC}}"
REG_PAYLOAD=$(cat <<EOF
{{
  "setup_token": "{setup_token}",
  "hostname": "$HOST_NAME",
  "os_info": "$OS_INFO",
  "public_ip": "$PUBLIC_IP"
}}
EOF
)

curl -s -X POST "{server_url}/api/agent/register" \
  -H "Content-Type: application/json" \
  -d "$REG_PAYLOAD" || true

# Setup lightweight background heartbeat cron/service
HEARTBEAT_SCRIPT="/usr/local/bin/cf-node-heartbeat.sh"
cat << 'EOF' > "$HEARTBEAT_SCRIPT"
#!/bin/bash
TOKEN="{setup_token}"
SERVER="{server_url}"
UPTIME_SEC=$(awk '{{print int($1)}}' /proc/uptime 2>/dev/null || echo 0)
LOAD=$(awk '{{print $1", "$2", "$3}}' /proc/loadavg 2>/dev/null || echo "")

curl -s -m 5 -X POST "$SERVER/api/agent/heartbeat" \
  -H "Content-Type: application/json" \
  -d "{{\"setup_token\":\"$TOKEN\",\"uptime\":$UPTIME_SEC,\"load_avg\":\"$LOAD\"}}" >/dev/null 2>&1
EOF
chmod +x "$HEARTBEAT_SCRIPT"

# Install periodic timer or crontab for heartbeat
(crontab -l 2>/dev/null | grep -v "cf-node-heartbeat.sh"; echo "*/1 * * * * $HEARTBEAT_SCRIPT") | crontab - 2>/dev/null || true

echo ""
echo -e "${{GREEN}}====================================================================${{NC}}"
echo -e "${{GREEN}}     SUCCESS! Machine {machine_name} is fully configured!           ${{NC}}"
echo -e "${{GREEN}}====================================================================${{NC}}"
echo -e "${{CYAN}}Tunnel ID:     ${{NC}} {tunnel_id}"
echo -e "${{CYAN}}Hostname:      ${{NC}} $HOST_NAME ($PUBLIC_IP)"
echo -e "${{CYAN}}Operating Sys: ${{NC}} $OS_INFO"
echo -e "${{CYAN}}Status:        ${{NC}} Connected & Protected by Cloudflare"
echo ""
echo -e "You can now return to your Dashboard at ${{BLUE}}{server_url}${{NC}}"
echo -e "to assign subdomains, open ports, and map services directly!"
echo ""
"#,
        machine_name = machine_name,
        server_url = server_url.trim_end_matches('/'),
        setup_token = setup_token,
        tunnel_token = tunnel_token,
        tunnel_id = tunnel_id,
    )
}
