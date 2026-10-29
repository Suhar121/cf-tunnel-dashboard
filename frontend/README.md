# ⚡ CF Tunnel Central - Next.js Frontend

The client dashboard for managing remote machines, Cloudflare Tunnels, and ingress routing.

## 🚀 Overview

Built with **Next.js 16 (App Router)**, **Turbopack**, and **Tailwind CSS v4**.

- **Node Cards & Status Badges:** Displays real-time node health (🟢 Online, 🟡 Pending, 🔴 Offline), IP addresses, and tunnel UUIDs.
- **Subdomain & Ingress Manager:** Live modal to add, edit, or delete subdomain routes (e.g., `app.domain.com` ➔ `http://localhost:3000` or `ssh.domain.com` ➔ `ssh://localhost:22`) with zero remote server SSH editing required.
- **Dual-Mode Setup Commands:**
  - **Direct Cloudflare CDN (Recommended):** Completely firewall-immune command with embedded tunnel token directly from Cloudflare's release debian packages.
  - **Local Dynamic Script:** Fallback one-liner curling the host machine's Axum provisioner.
- **Cross-Context Clipboard Utility:** Automatically handles clipboard copying across insecure HTTP LAN addresses (`http://192.168.x.x:3000`) without throwing security exceptions.
- **Cloudflare API Management:** Test token permissions and view available active zones.

## 🛠️ Running Locally

```bash
npm install
npm run dev
```

The frontend runs on `http://localhost:3000` and automatically proxies all `/api/*` and `/setup/*` requests to the Rust backend on port `8080`.
