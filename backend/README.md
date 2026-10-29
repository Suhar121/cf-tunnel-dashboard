# 🦀 CF Tunnel Central - Rust Backend

The core daemon and API engine for automated Cloudflare Tunnel provisioning and ingress management.

## 🚀 Overview

Built with **Axum 0.7**, **Tokio**, and **Rusqlite** (bundled SQLite). It provides:
- **Cloudflare v4 REST API Integration:** Manages Cloudflare Zero Trust Tunnels, fetching token credentials, pushing remote ingress configurations, and managing DNS CNAME records.
- **Dynamic Bash Provisioner:** Dynamically serves architecture-aware bootstrap scripts (`x86_64`, `aarch64`, `armv7l`) with embedded runner tokens.
- **Background Health Daemon:** Asynchronously polls Cloudflare Edge tunnel status every 20 seconds to keep node status fresh in SQLite.
- **Local Network Detection:** Detects the host's active LAN IPv4 address (e.g. `192.168.x.x`) to display in setup scripts.
- **Root Redirector:** Accessing `http://localhost:8080/` serves a diagnostic status page with an automatic redirect to the Next.js UI on port 3000.

## 🛠️ Running Locally

```bash
cargo run
```

The server binds to `0.0.0.0:8080`.

## 📦 API Endpoints

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/` | Diagnostic status page & redirect |
| `GET` | `/api/network-info` | Detected host LAN IP & port info |
| `GET` | `/api/settings` | Current Cloudflare configuration & status |
| `POST` | `/api/settings` | Save Cloudflare API token & account ID |
| `GET` | `/api/cloudflare/zones` | Fetch list of active Cloudflare domains |
| `GET` | `/api/machines` | List all provisioned machines & edge health |
| `POST` | `/api/machines` | Create machine & provision Cloudflare tunnel |
| `DELETE` | `/api/machines/:id` | Delete machine & teardown remote tunnel |
| `GET` | `/api/machines/:id/routes` | Get active ingress routes for a machine |
| `POST` | `/api/machines/:id/routes` | Add ingress route & upsert DNS CNAME |
| `DELETE` | `/api/machines/:id/routes/:route_id` | Delete ingress route & DNS record |
| `GET` | `/api/logs` | Real-time system audit logs |
| `GET` | `/setup/:token.sh` | Dynamic bash installation script |
