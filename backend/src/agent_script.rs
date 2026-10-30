pub fn generate_bash_script(
    server_url: &str,
    machine_name: &str,
    setup_token: &str,
    tunnel_token: &str,
    tunnel_id: &str,
) -> String {
    format!(r#"#!/bin/bash
# ==============================================================================
# Cloudflare Tunnel Auto-Provisioner & Discovery Agent
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

# Ensure curl and iproute2/net-tools are installed
if ! command -v curl &> /dev/null; then
  echo -e "${{YELLOW}}[*] Installing curl...${{NC}}"
  if command -v apt-get &> /dev/null; then
    apt-get update -qq && apt-get install -y -qq curl iproute2
  elif command -v yum &> /dev/null; then
    yum install -y curl iproute
  elif command -v dnf &> /dev/null; then
    dnf install -y curl iproute
  elif command -v pacman &> /dev/null; then
    pacman -Sy --noconfirm curl iproute2
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

# Install Service & Docker Discovery Probe Script
DISCOVER_SCRIPT="/usr/local/bin/cf-node-discover.sh"
cat << 'EOF' > "$DISCOVER_SCRIPT"
#!/bin/bash
TOKEN="{setup_token}"
SERVER="{server_url}"

PORTS=$(ss -tlnH 2>/dev/null | awk '{{print $4}}' | sed -E 's/.*:([0-9]+)$/\1/' | sort -nu | grep -vE '^(22|53|68)$' || true)

get_name() {{
  case "$1" in
    80) echo "HTTP Web Server" ;;
    443) echo "HTTPS Web Server" ;;
    3000) echo "Next.js / Grafana" ;;
    5000) echo "Flask / Registry" ;;
    5173) echo "Vite Dev Server" ;;
    8000) echo "FastAPI / Python" ;;
    8080) echo "Web Service" ;;
    8443) echo "Secure Web" ;;
    8888) echo "Jupyter Notebook" ;;
    9000) echo "Portainer / Sonar" ;;
    11434) echo "Ollama AI LLM" ;;
    5678) echo "n8n Workflow" ;;
    8123) echo "Home Assistant" ;;
    *) echo "Service on Port $1" ;;
  esac
}}

get_sub() {{
  case "$1" in
    80) echo "web" ;;
    443) echo "secure" ;;
    3000) echo "app" ;;
    5000) echo "flask" ;;
    5173) echo "dev" ;;
    8000) echo "api" ;;
    8080) echo "service" ;;
    8888) echo "jupyter" ;;
    9000) echo "portainer" ;;
    11434) echo "ollama" ;;
    5678) echo "n8n" ;;
    8123) echo "hass" ;;
    *) echo "port-$1" ;;
  esac
}}

SERVICES="["
FIRST=1

if command -v docker &>/dev/null && docker ps &>/dev/null; then
  while IFS='|' read -r cname cports; do
    if [ -n "$cname" ] && [ -n "$cports" ]; then
      for port in $(echo "$cports" | grep -oE '0\.0\.0\.0:[0-9]+' | cut -d: -f2 | sort -nu); do
        if [ "$FIRST" -eq 0 ]; then SERVICES="$SERVICES,"; fi
        FIRST=0
        clean_sub=$(echo "$cname" | tr -cd '[:alnum:]-' | tr '[:upper:]' '[:lower:]')
        SERVICES="$SERVICES{{\"name\":\"Docker: $cname\",\"port\":$port,\"protocol\":\"http\",\"source\":\"docker\",\"suggested_subdomain\":\"$clean_sub\"}}"
      done
    fi
  done < <(docker ps --format '{{{{.Names}}}}|{{{{.Ports}}}}' 2>/dev/null)
fi

for p in $PORTS; do
  if [ -n "$p" ] && ! echo "$SERVICES" | grep -q "\"port\":$p"; then
    if [ "$FIRST" -eq 0 ]; then SERVICES="$SERVICES,"; fi
    FIRST=0
    NAME=$(get_name "$p")
    SUB=$(get_sub "$p")
    SERVICES="$SERVICES{{\"name\":\"$NAME\",\"port\":$p,\"protocol\":\"http\",\"source\":\"system\",\"suggested_subdomain\":\"$SUB\"}}"
  fi
done

SERVICES="$SERVICES]"

curl -s -m 5 -X POST "$SERVER/api/agent/services" \
  -H "Content-Type: application/json" \
  -d "{{\"setup_token\":\"$TOKEN\",\"services\":$SERVICES}}" >/dev/null 2>&1 || true
EOF
chmod +x "$DISCOVER_SCRIPT"

# Run discovery probe once immediately
echo -e "${{BLUE}}[*] Scanning for listening ports & Docker containers...${{NC}}"
"$DISCOVER_SCRIPT" || true

# Setup background heartbeat & periodic discovery cron
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

# Refresh service discovery periodically in background
/usr/local/bin/cf-node-discover.sh >/dev/null 2>&1 &
EOF
chmod +x "$HEARTBEAT_SCRIPT"

# Install periodic timer in crontab
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
echo -e "to assign subdomains, auto-expose discovered ports, and enable Zero Trust!"
echo ""
"#,
        machine_name = machine_name,
        server_url = server_url.trim_end_matches('/'),
        setup_token = setup_token,
        tunnel_token = tunnel_token,
        tunnel_id = tunnel_id,
    )
}
