"use client";

import React, { useState, useEffect } from "react";
import {
  Copy,
  Check,
  Terminal,
  Shield,
  Cpu,
  RefreshCw,
  X,
  Wifi,
  AlertTriangle,
  Zap,
  Globe,
  Sparkles,
} from "lucide-react";
import { Machine, fetchNetworkInfo, fetchMachineDetails } from "@/lib/api";
import { copyTextToClipboard } from "@/lib/clipboard";

interface SetupCommandModalProps {
  machine: Machine | null;
  defaultServerUrl: string;
  onClose: () => void;
  onRegistered: () => void;
}

export const SetupCommandModal: React.FC<SetupCommandModalProps> = ({
  machine,
  defaultServerUrl,
  onClose,
  onRegistered,
}) => {
  const [copiedType, setCopiedType] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<"direct" | "script">("direct");
  const [customServerUrl, setCustomServerUrl] = useState(defaultServerUrl);
  const [detectedLanIp, setDetectedLanIp] = useState("192.168.29.143");
  const [tunnelToken, setTunnelToken] = useState(machine?.tunnel_token || "");
  const [isWaiting, setIsWaiting] = useState(true);

  useEffect(() => {
    if (!machine) return;

    fetchNetworkInfo()
      .then((net) => {
        if (net.local_ip && net.local_ip !== "127.0.0.1") {
          setDetectedLanIp(net.local_ip);
          if (!customServerUrl || customServerUrl.includes("localhost") || customServerUrl.includes("127.0.0.1")) {
            setCustomServerUrl(`http://${net.local_ip}:8080`);
          }
        }
      })
      .catch(() => {});

    // Fetch latest tunnel token and details
    fetchMachineDetails(machine.id)
      .then((det) => {
        if (det.tunnel_token) setTunnelToken(det.tunnel_token);
        if (det.machine.status === "online") {
          setIsWaiting(false);
          onRegistered();
        }
      })
      .catch(() => {});

    // Poll status every 3 seconds
    const interval = setInterval(async () => {
      try {
        const res = await fetch(`/api/machines/${machine.id}`);
        if (res.ok) {
          const data = await res.json();
          if (data.machine && data.machine.status === "online") {
            setIsWaiting(false);
            onRegistered();
          }
        }
      } catch {
        // ignore
      }
    }, 3000);

    return () => clearInterval(interval);
  }, [machine, onRegistered]);

  if (!machine) return null;

  const tokenToUse = tunnelToken || machine.tunnel_token || "";

  // 1. Direct Cloudflare command (bypasses Windows firewall completely)
  const directDebCommand = `curl -L --output /tmp/cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb && sudo dpkg -i /tmp/cloudflared.deb && sudo cloudflared service install ${tokenToUse}`;

  // 2. Standalone binary command
  const directBinCommand = `sudo curl -fsSL -o /usr/local/bin/cloudflared https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 && sudo chmod +x /usr/local/bin/cloudflared && sudo cloudflared service install ${tokenToUse}`;

  // 3. Local laptop script command
  const currentHost =
    customServerUrl.trim() ||
    (detectedLanIp && detectedLanIp !== "127.0.0.1" ? `http://${detectedLanIp}:8080` : "http://127.0.0.1:8080");
  const scriptCommand = `curl -fsSL ${currentHost.replace(/\/$/, "")}/setup/${machine.setup_token}.sh | sudo bash`;

  const handleCopy = async (text: string, type: string) => {
    await copyTextToClipboard(text);
    setCopiedType(type);
    setTimeout(() => setCopiedType(null), 2500);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-2xl overflow-hidden shadow-2xl flex flex-col">
        {/* Header */}
        <div className="px-6 py-4 border-b border-neutral-800 flex items-center justify-between bg-neutral-950/50">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-orange-500/10 border border-orange-500/20 flex items-center justify-center">
              <Terminal className="w-4 h-4 text-orange-400" />
            </div>
            <div>
              <h3 className="text-base font-semibold text-white">Install Cloudflare Tunnel Node</h3>
              <p className="text-xs text-neutral-400">
                Node: <span className="text-orange-400 font-medium">{machine.name}</span> | Domain:{" "}
                <span className="text-neutral-300 font-mono">{machine.zone_name || "Active"}</span>
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-neutral-400 hover:text-white p-1 rounded-lg hover:bg-neutral-800 transition-colors cursor-pointer"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-5">
          {/* Status banner */}
          <div
            className={`p-3.5 rounded-xl border flex items-center justify-between ${
              machine.status === "online" || !isWaiting
                ? "bg-emerald-500/10 border-emerald-500/20 text-emerald-300"
                : "bg-amber-500/10 border-amber-500/20 text-amber-300"
            }`}
          >
            <div className="flex items-center gap-2.5">
              <span
                className={`w-2.5 h-2.5 rounded-full ${
                  machine.status === "online" || !isWaiting
                    ? "bg-emerald-400 animate-pulse"
                    : "bg-amber-400 animate-ping"
                }`}
              />
              <span className="text-xs font-medium">
                {machine.status === "online" || !isWaiting
                  ? "✓ Cloudflare Tunnel is connected and ACTIVE on Cloudflare Edge!"
                  : "Awaiting connection: Run the command below on your remote server"}
              </span>
            </div>
            {machine.status !== "online" && isWaiting && (
              <RefreshCw className="w-3.5 h-3.5 animate-spin text-amber-400" />
            )}
          </div>

          {/* Mode Switcher Tabs */}
          <div className="flex p-1 rounded-xl bg-neutral-950 border border-neutral-800">
            <button
              onClick={() => setActiveTab("direct")}
              className={`flex-1 py-2 px-3 rounded-lg text-xs font-semibold flex items-center justify-center gap-2 transition-all cursor-pointer ${
                activeTab === "direct"
                  ? "bg-orange-500 text-white shadow-sm"
                  : "text-neutral-400 hover:text-neutral-200"
              }`}
            >
              <Zap className="w-3.5 h-3.5" />
              <span>Direct Cloudflare Command (Recommended)</span>
              <span className="text-[10px] uppercase font-bold px-1.5 py-0.2 rounded bg-black/20 text-white">
                Bypasses Firewall
              </span>
            </button>

            <button
              onClick={() => setActiveTab("script")}
              className={`flex-1 py-2 px-3 rounded-lg text-xs font-semibold flex items-center justify-center gap-2 transition-all cursor-pointer ${
                activeTab === "script"
                  ? "bg-neutral-800 text-white shadow-sm"
                  : "text-neutral-400 hover:text-neutral-200"
              }`}
            >
              <Wifi className="w-3.5 h-3.5" />
              <span>Via Laptop IP ({detectedLanIp})</span>
            </button>
          </div>

          {/* Tab 1: Direct Cloudflare Command */}
          {activeTab === "direct" && (
            <div className="space-y-4">
              <div className="rounded-xl bg-emerald-500/5 border border-emerald-500/20 p-3.5 text-xs text-neutral-300 space-y-1">
                <div className="flex items-center gap-1.5 font-semibold text-emerald-400">
                  <Shield className="w-4 h-4" />
                  <span>100% Reliable — Direct Cloudflare CDN Installation</span>
                </div>
                <p className="text-[11px] text-neutral-400 leading-relaxed">
                  Downloads <code className="text-neutral-200 bg-neutral-900 px-1 py-0.5 rounded">cloudflared</code> directly from Cloudflare, installs it as a background systemd service, and authenticates using your tunnel token. Does not depend on your laptop&apos;s Windows firewall or Wi-Fi settings!
                </p>
              </div>

              {/* Ubuntu / Debian (.deb) */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
                    <Cpu className="w-3.5 h-3.5 text-orange-400" />
                    For Ubuntu / Debian (Recommended):
                  </span>
                  <span className="text-[11px] text-neutral-500 font-mono">apt / dpkg</span>
                </div>
                <div className="relative group bg-neutral-950 border border-neutral-800 rounded-xl p-3.5 font-mono text-xs flex items-center justify-between">
                  <code className="text-orange-300 break-all pr-4 select-all">{directDebCommand}</code>
                  <button
                    onClick={() => handleCopy(directDebCommand, "direct-deb")}
                    className="shrink-0 flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-orange-500 hover:bg-orange-600 text-white font-sans text-xs font-semibold transition-all shadow-md active:scale-95 cursor-pointer"
                  >
                    {copiedType === "direct-deb" ? (
                      <>
                        <Check className="w-3.5 h-3.5 text-white" />
                        <span>Copied!</span>
                      </>
                    ) : (
                      <>
                        <Copy className="w-3.5 h-3.5" />
                        <span>Copy</span>
                      </>
                    )}
                  </button>
                </div>
              </div>

              {/* Any Linux (Standalone binary) */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-neutral-300">
                    For Other Linux (CentOS / RHEL / Arch / Alpine):
                  </span>
                  <span className="text-[11px] text-neutral-500 font-mono">binary</span>
                </div>
                <div className="relative group bg-neutral-950 border border-neutral-800 rounded-xl p-3 font-mono text-[11px] flex items-center justify-between">
                  <code className="text-neutral-300 break-all pr-4 select-all">{directBinCommand}</code>
                  <button
                    onClick={() => handleCopy(directBinCommand, "direct-bin")}
                    className="shrink-0 flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-200 font-sans text-xs font-medium transition-all cursor-pointer"
                  >
                    {copiedType === "direct-bin" ? (
                      <>
                        <Check className="w-3 h-3 text-emerald-400" />
                        <span>Copied!</span>
                      </>
                    ) : (
                      <>
                        <Copy className="w-3 h-3" />
                        <span>Copy</span>
                      </>
                    )}
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* Tab 2: Dynamic Shell Script via Laptop IP */}
          {activeTab === "script" && (
            <div className="space-y-4">
              <div className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-300 text-xs flex items-start gap-2">
                <AlertTriangle className="w-4 h-4 shrink-0 mt-0.5" />
                <span>
                  <strong>Notice:</strong> If the curl command times out with <em>Failed to connect to {detectedLanIp}</em>, Windows Defender Firewall is blocking inbound connections to port 8080. Switch to the <strong>Direct Cloudflare Command</strong> tab above which works without needing local ports!
                </span>
              </div>

              <div className="space-y-1.5">
                <label className="text-xs font-medium text-neutral-300 flex items-center gap-1.5">
                  <Wifi className="w-3.5 h-3.5 text-orange-400" />
                  <span>Laptop IP Address:</span>
                </label>
                <input
                  type="text"
                  value={customServerUrl}
                  onChange={(e) => setCustomServerUrl(e.target.value)}
                  placeholder={`http://${detectedLanIp}:8080`}
                  className="w-full px-3.5 py-2 bg-neutral-950 border border-neutral-800 rounded-xl text-xs text-neutral-200 font-mono"
                />
              </div>

              <div className="relative group bg-neutral-950 border border-neutral-800 rounded-xl p-3.5 font-mono text-xs flex items-center justify-between">
                <code className="text-orange-300 break-all pr-4 select-all">{scriptCommand}</code>
                <button
                  onClick={() => handleCopy(scriptCommand, "script")}
                  className="shrink-0 flex items-center gap-1.5 px-3.5 py-2 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-white font-sans text-xs font-semibold transition-all shadow-md active:scale-95 cursor-pointer"
                >
                  {copiedType === "script" ? (
                    <>
                      <Check className="w-3.5 h-3.5 text-emerald-400" />
                      <span>Copied!</span>
                    </>
                  ) : (
                    <>
                      <Copy className="w-3.5 h-3.5" />
                      <span>Copy</span>
                    </>
                  )}
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-neutral-800 bg-neutral-950/50 flex items-center justify-between">
          <p className="text-[11px] text-neutral-500">
            Once installed, your machine will connect to Cloudflare and appear as{" "}
            <span className="text-emerald-400 font-medium">Online</span> in real time.
          </p>
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-xl text-xs font-medium bg-neutral-800 hover:bg-neutral-700 text-white transition-colors cursor-pointer"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
