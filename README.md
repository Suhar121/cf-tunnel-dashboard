# 🛡️ Cloudflare Tunnel Central (Rust + Next.js)

<p align="center">
  <strong>Zero-Touch, Automated Cloudflare Tunnel Provisioner & Ingress Routing Platform</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-Axum_0.7-orange?logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Next.js-16_(Turbopack)-black?logo=next.js&logoColor=white" alt="Next.js" />
  <img src="https://img.shields.io/badge/Cloudflare-Zero_Trust-F38020?logo=cloudflare&logoColor=white" alt="Cloudflare" />
  <img src="https://img.shields.io/badge/Tailwind_CSS-v4-38B2AC?logo=tailwind-css&logoColor=white" alt="Tailwind" />
  <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License" />
</p>

---

## ⚡ The Problem Solved

Whenever you set up a new remote server, lab machine, or VM:
- ❌ **Manual cloudflared install:** Locating, downloading, and unpacking the correct architecture binary (`amd64`, `arm64`, etc.).
- ❌ **Manual authentication:** Running `cloudflared tunnel login` or transferring `cert.pem` files across devices.
- ❌ **Manual YAML maintenance:** Crafting `~/.cloudflared/config.yml` with tunnel UUIDs, credential paths, ingress route arrays, and catch-all 404 rules.
- ❌ **Manual DNS management:** Logging into the Cloudflare dashboard to manually create and proxy CNAME records for every single subdomain.
- ❌ **Manual systemd daemons:** Configuring services, permissions, and log rotations.

### 🚀 The Solution with this Dashboard

1. **Start the Dashboard** on your primary machine or laptop.
2. Click **"Add Server / Lab Node"** and pick your Cloudflare domain.
3. SSH into your remote server and paste the **1-line setup command**:
   ```bash
   curl -L --output /tmp/cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb && sudo dpkg -i /tmp/cloudflared.deb && sudo cloudflared service install <TUNNEL_TOKEN>
   ```
4. **Instant Edge Handshake:**
   - The remote machine installs `cloudflared` as a background `systemd` daemon.
   - It establishes secure outbound QUIC / HTTP2 tunnels to Cloudflare Edge.
   - The dashboard automatically detects the connection and marks the node **🟢 Online**.
5. **No-YAML Ingress Routing:** Click **"Subdomains"** on any machine in the dashboard to map ports:
   - `api.yourdomain.com` ➔ `http://localhost:8000`
   - `app.yourdomain.com` ➔ `http://localhost:3000`
   - `ssh.yourdomain.com` ➔ `ssh://localhost:22`
   - **Both DNS CNAMEs and Edge ingress rules are pushed to Cloudflare in real time without touching configuration files on the remote server.**

---

## 🏗️ System Architecture

```
┌────────────────────────────────────────────────────────┐
│               Next.js 16 Dashboard (:3000)             │
│   • Node Health Badges (Online / Offline / Pending)    │
│   • Subdomain & Port Ingress Manager                   │
│   • One-Click Setup Commands & Firewall Bypass         │
└───────────────────────────┬────────────────────────────┘
                            │ (Transparent Proxy / REST)
┌───────────────────────────▼────────────────────────────┐
│                Rust Axum Backend (:8080)               │
│   • SQLite Persistence (`cf_dashboard.db`)             │
│   • Dynamic Bash Provisioner Generator                 │
│   • Cloudflare v4 REST API Client                      │
│   • Real-Time Edge Health Synchronizer                 │
└──────────────┬──────────────────────────┬──────────────┘
               │                          │
        (Cloudflare API)             (Tunnel Run)
               │                          │
┌──────────────▼────────────┐    ┌────────▼──────────────────────┐
│   Cloudflare Edge Network │    │ Remote Node (Ubuntu / Lab)    │
│ • Zero Trust CFD Tunnels  │    │ • systemd service daemon      │
│ • Remote Ingress Engine   │◄───┤ • Outbound QUIC connection    │
│ • Automatic DNS CNAMEs    │    │ • Zero open inbound ports     │
└───────────────────────────┘    └───────────────────────────────┘
```

---

## ✨ Features

- **Zero-Touch Provisioning:** Generates commands with embedded tokens for Ubuntu/Debian (`.deb`) and generic Linux (`binary`).
- **Edge Status Sync:** Automatically polls Cloudflare's Edge API (`cfd_tunnel/{id}`) to detect when nodes come online without requiring open local router ports.
- **Docker & Port Auto-Discovery:** Remote machines scan for listening TCP ports and running Docker containers (`docker ps`), streaming them to the dashboard for 1-click subdomain exposure.
- **Zero Trust Web App Protection:** 1-click protect exposed subdomains using Cloudflare Access Email PIN verification, ensuring only authorized email addresses can access sensitive web applications.
- **Dynamic Ingress Manager:** Add or delete subdomains with automatic DNS CNAME record upsertion and automatic conflict resolution.
- **Firewall-Immune Deployment:** Uses direct Cloudflare CDN installation so local Windows Defender firewalls or Wi-Fi AP isolation never block setup.
- **Native SSH Support:** Pre-configured SSH proxy command for instant `ssh homelab` terminal access from anywhere in the world.
- **Dark Glassmorphic UI:** Built with Tailwind CSS v4 and Lucide React icons.

---

## 🛠️ Quick Start

### 1. Prerequisites
- **Rust & Cargo** (`cargo >= 1.75`)
- **Node.js** (`node >= 18`)

### 2. Start Both Services

#### On Windows (PowerShell):
```powershell
.\start.ps1
```

#### On Linux / macOS:
```bash
chmod +x start.sh
./start.sh
```

#### Or Run Independently:
```bash
# Terminal 1: Rust Backend
cd backend
cargo run

# Terminal 2: Next.js Frontend
cd frontend
npm install
npm run dev
```

Open **`http://localhost:3000`** in your browser.

---

## 🔑 Cloudflare API Setup

In the **Cloudflare API** tab of the dashboard:
1. Provide a Cloudflare API Token with these permissions:
   - **Account** ➔ `Cloudflare Tunnel` ➔ `Edit`
   - **Zone** ➔ `DNS` ➔ `Edit`
   - **Zone** ➔ `Zone` ➔ `Read`
   - **Account** ➔ `Access: Apps and Policies` ➔ `Edit` (for 1-click Zero Trust Email PIN protection)
2. Provide your **Account ID** (found in the sidebar of any domain in the Cloudflare dashboard).
3. The dashboard will automatically fetch your active domains and make them available in the node creator.

---

## 💻 SSH Through Cloudflare Tunnel

To connect to any machine exposed via `ssh://localhost:22`:

### 1. Quick One-Line SSH
```bash
ssh -o ProxyCommand="cloudflared access ssh --hostname %h" user@ssh.yourdomain.com
```

### 2. Config File Setup (`~/.ssh/config`)
Add this block to `~/.ssh/config`:
```ssh-config
Host homelab ssh.yourdomain.com
    HostName ssh.yourdomain.com
    User your-user
    ProxyCommand cloudflared access ssh --hostname %h
```

Now you can connect simply by typing:
```bash
ssh homelab
```
*(Also compatible with VS Code's **Remote - SSH** extension!)*

---

## 📁 Repository Structure

```
cf-tunnel-dashboard/
├── backend/                   # Rust Axum Backend
│   ├── src/
│   │   ├── main.rs            # Server entry, routing & health worker
│   │   ├── models.rs          # Data models (Machines, Routes, Cloudflare)
│   │   ├── db.rs              # SQLite persistence (rusqlite bundled)
│   │   ├── cloudflare.rs      # Cloudflare API v4 client
│   │   ├── agent_script.rs    # Dynamic bash installer script generator
│   │   └── routes/            # Route modules (machines, cloudflare, agent)
│   ├── Cargo.toml
│   └── .env.example
│
├── frontend/                  # Next.js 16 App Router
│   ├── src/
│   │   ├── app/               # Layout, styles & dashboard page
│   │   ├── components/        # Navbar, Modals, Settings & Logs
│   │   └── lib/               # Typed API client & universal clipboard
│   ├── next.config.ts         # Proxy rewrites to :8080 & Turbopack config
│   ├── postcss.config.mjs
│   └── package.json
│
├── start.ps1                  # Windows one-click start script
├── start.sh                   # Unix one-click start script
├── .gitignore
└── README.md
```

---

## 🛡️ Security Note

- **Never commit `.db` or `.env` files.** The `.gitignore` is configured to ignore all SQLite databases and environment secrets.
- Inbound ports on your remote machines do **not** need to be opened on your router. Cloudflare tunnels operate entirely via outbound encrypted connections.

---

## 📄 License

MIT License. Built for modern homelab and cloud infrastructure automation.
