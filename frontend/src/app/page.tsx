"use client";

import React, { useState, useEffect } from "react";
import {
  Server,
  Terminal,
  Network,
  Plus,
  RefreshCw,
  Copy,
  Trash2,
  ExternalLink,
  ShieldCheck,
  Check,
  AlertCircle,
  Cpu,
  Globe,
  Radio,
  ArrowRight,
  Activity,
  Layers,
  CheckCircle2,
} from "lucide-react";
import { Navbar } from "@/components/Navbar";
import { CreateMachineModal } from "@/components/CreateMachineModal";
import { SetupCommandModal } from "@/components/SetupCommandModal";
import { RoutesModal } from "@/components/RoutesModal";
import { SettingsView } from "@/components/SettingsView";
import { LogsView } from "@/components/LogsView";
import {
  Machine,
  CloudflareZone,
  CloudflareSettings,
  fetchMachines,
  fetchZones,
  fetchSettings,
  deleteMachine,
} from "@/lib/api";
import { copyTextToClipboard } from "@/lib/clipboard";

export default function Dashboard() {
  const [activeTab, setActiveTab] = useState<"machines" | "settings" | "logs">("machines");
  const [machines, setMachines] = useState<Machine[]>([]);
  const [zones, setZones] = useState<CloudflareZone[]>([]);
  const [settings, setSettings] = useState<CloudflareSettings | null>(null);
  const [loading, setLoading] = useState(true);

  // Modals
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [preselectedZoneId, setPreselectedZoneId] = useState<string | undefined>(undefined);
  const [activeSetupMachine, setActiveSetupMachine] = useState<Machine | null>(null);
  const [activeRoutesMachine, setActiveRoutesMachine] = useState<Machine | null>(null);

  const [copiedId, setCopiedId] = useState<string | null>(null);

  const loadAll = async () => {
    try {
      setLoading(true);
      const conf = await fetchSettings();
      setSettings(conf);

      if (conf.configured) {
        const [machinesData, zonesData] = await Promise.all([
          fetchMachines().catch(() => []),
          fetchZones().catch(() => []),
        ]);
        setMachines(machinesData);
        setZones(zonesData);
      }
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadAll();
    const interval = setInterval(() => {
      if (settings?.configured) {
        fetchMachines()
          .then(setMachines)
          .catch(() => {});
      }
    }, 5000);
    return () => clearInterval(interval);
  }, [settings?.configured]);

  const handleDeleteMachine = async (id: string, name: string) => {
    if (!confirm(`Are you sure you want to delete node "${name}"? This will delete the Cloudflare tunnel and its DNS records.`)) {
      return;
    }
    try {
      await deleteMachine(id);
      await loadAll();
    } catch (err: any) {
      alert(`Error deleting machine: ${err.message}`);
    }
  };

  const copyQuickCurl = async (m: Machine) => {
    const host =
      settings?.server_url ||
      (typeof window !== "undefined" ? `${window.location.protocol}//${window.location.hostname}:8080` : "http://localhost:8080");
    const cmd = `curl -fsSL ${host.replace(/\/$/, "")}/setup/${m.setup_token}.sh | sudo bash`;
    await copyTextToClipboard(cmd);
    setCopiedId(m.id);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const openCreateForZone = (zoneId?: string) => {
    setPreselectedZoneId(zoneId);
    setShowCreateModal(true);
  };

  const onlineCount = machines.filter((m) => m.status === "online").length;
  const totalRoutes = machines.reduce((acc, m) => acc + (m.routes_count || 0), 0);

  return (
    <div className="min-h-screen flex flex-col bg-neutral-950 text-neutral-100">
      <Navbar
        activeTab={activeTab}
        setActiveTab={setActiveTab}
        onOpenCreate={() => openCreateForZone()}
        isConfigured={!!settings?.configured}
        totalMachines={machines.length}
        onlineMachines={onlineCount}
      />

      <main className="flex-1 max-w-7xl w-full mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {/* Settings Tab */}
        {activeTab === "settings" && (
          <SettingsView
            onConfigured={() => {
              loadAll();
              setActiveTab("machines");
            }}
          />
        )}

        {/* Logs Tab */}
        {activeTab === "logs" && <LogsView />}

        {/* Machines Tab */}
        {activeTab === "machines" && (
          <div className="space-y-8">
            {/* Unconfigured Warning Alert */}
            {!settings?.configured && !loading && (
              <div className="bg-gradient-to-r from-amber-500/10 via-orange-500/10 to-transparent border border-amber-500/20 rounded-2xl p-6 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
                <div className="flex items-start gap-3.5">
                  <div className="w-9 h-9 rounded-xl bg-amber-500/20 border border-amber-500/30 flex items-center justify-center shrink-0">
                    <AlertCircle className="w-5 h-5 text-amber-400" />
                  </div>
                  <div>
                    <h3 className="text-sm font-semibold text-white">Cloudflare API Token Required</h3>
                    <p className="text-xs text-neutral-400 mt-0.5">
                      To automatically create tunnels and map subdomains without touching config files, add your Cloudflare token in Settings.
                    </p>
                  </div>
                </div>
                <button
                  onClick={() => setActiveTab("settings")}
                  className="shrink-0 flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600 text-white shadow-md shadow-orange-500/20 transition-all cursor-pointer"
                >
                  <span>Configure API Token</span>
                  <ArrowRight className="w-3.5 h-3.5" />
                </button>
              </div>
            )}

            {/* Metrics Overview Bar */}
            <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
              <div className="bg-neutral-900/60 border border-neutral-800 rounded-2xl p-4 flex items-center gap-3.5">
                <div className="w-10 h-10 rounded-xl bg-orange-500/10 border border-orange-500/20 flex items-center justify-center">
                  <Server className="w-5 h-5 text-orange-400" />
                </div>
                <div>
                  <p className="text-[11px] font-medium text-neutral-400 uppercase tracking-wider">Total Nodes</p>
                  <p className="text-xl font-bold text-white tracking-tight">{machines.length}</p>
                </div>
              </div>

              <div className="bg-neutral-900/60 border border-neutral-800 rounded-2xl p-4 flex items-center gap-3.5">
                <div className="w-10 h-10 rounded-xl bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center">
                  <Radio className="w-5 h-5 text-emerald-400" />
                </div>
                <div>
                  <p className="text-[11px] font-medium text-neutral-400 uppercase tracking-wider">Active Tunnels</p>
                  <div className="flex items-center gap-2">
                    <p className="text-xl font-bold text-white tracking-tight">{onlineCount}</p>
                    <span className="text-[10px] px-1.5 py-0.2 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                      Online
                    </span>
                  </div>
                </div>
              </div>

              <div className="bg-neutral-900/60 border border-neutral-800 rounded-2xl p-4 flex items-center gap-3.5">
                <div className="w-10 h-10 rounded-xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center">
                  <Network className="w-5 h-5 text-blue-400" />
                </div>
                <div>
                  <p className="text-[11px] font-medium text-neutral-400 uppercase tracking-wider">Ingress Routes</p>
                  <p className="text-xl font-bold text-white tracking-tight">{totalRoutes}</p>
                </div>
              </div>

              <div className="bg-neutral-900/60 border border-neutral-800 rounded-2xl p-4 flex items-center gap-3.5">
                <div className="w-10 h-10 rounded-xl bg-purple-500/10 border border-purple-500/20 flex items-center justify-center">
                  <Globe className="w-5 h-5 text-purple-400" />
                </div>
                <div>
                  <p className="text-[11px] font-medium text-neutral-400 uppercase tracking-wider">Cloudflare Domains</p>
                  <div className="flex items-center gap-2">
                    <p className="text-xl font-bold text-white tracking-tight">{zones.length}</p>
                    <span className="text-[10px] px-1.5 py-0.2 rounded-full bg-purple-500/10 text-purple-400 border border-purple-500/20">
                      Ready
                    </span>
                  </div>
                </div>
              </div>
            </div>

            {/* Connected Domains Bar */}
            {zones.length > 0 && (
              <div className="p-4 rounded-2xl bg-neutral-900/40 border border-neutral-800/80 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3">
                <div className="flex items-center gap-2.5">
                  <div className="w-7 h-7 rounded-lg bg-purple-500/10 border border-purple-500/20 flex items-center justify-center">
                    <Globe className="w-4 h-4 text-purple-400" />
                  </div>
                  <div>
                    <span className="text-xs font-semibold text-neutral-200">Connected Cloudflare Domains:</span>
                    <span className="text-xs text-neutral-400 ml-2">Select a domain when adding any node</span>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  {zones.map((z) => (
                    <div
                      key={z.id}
                      className="flex items-center gap-2 px-3 py-1 rounded-xl bg-neutral-950 border border-neutral-800 text-xs font-mono"
                    >
                      <span className="w-2 h-2 rounded-full bg-emerald-400" />
                      <span className="text-neutral-200 font-medium">{z.name}</span>
                      <button
                        onClick={() => openCreateForZone(z.id)}
                        className="text-[10px] px-2 py-0.5 rounded-md bg-orange-500/10 text-orange-400 hover:bg-orange-500 hover:text-white transition-all font-sans font-semibold cursor-pointer"
                        title={`Deploy new tunnel on ${z.name}`}
                      >
                        + Add Node
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Nodes Header & Actions */}
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-lg font-bold text-white tracking-tight">Provisioned Server Nodes</h2>
                <p className="text-xs text-neutral-400">
                  Each machine runs cloudflared as a systemd daemon managed directly via Cloudflare Edge.
                </p>
              </div>
              <div className="flex items-center gap-2">
                <button
                  onClick={loadAll}
                  className="p-2 rounded-xl bg-neutral-900 border border-neutral-800 text-neutral-400 hover:text-white hover:bg-neutral-800 transition-colors"
                  title="Refresh status"
                >
                  <RefreshCw className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Machines Grid */}
            {loading && machines.length === 0 ? (
              <div className="py-24 text-center space-y-3">
                <div className="inline-block p-3 rounded-full bg-orange-500/10 border border-orange-500/20">
                  <RefreshCw className="w-6 h-6 animate-spin text-orange-400" />
                </div>
                <p className="text-xs text-neutral-400">Checking connected nodes & Cloudflare status...</p>
              </div>
            ) : machines.length === 0 ? (
              /* Empty State Onboarding */
              <div className="border border-neutral-800 rounded-3xl p-8 sm:p-12 text-center bg-gradient-to-b from-neutral-900/40 to-neutral-950 space-y-6">
                <div className="w-16 h-16 rounded-2xl bg-orange-500/10 border border-orange-500/20 flex items-center justify-center mx-auto text-orange-400">
                  <Server className="w-8 h-8" />
                </div>
                <div className="max-w-md mx-auto space-y-2">
                  <h3 className="text-base font-bold text-white">No machines configured yet</h3>
                  <p className="text-xs text-neutral-400 leading-relaxed">
                    {zones.length > 0
                      ? `Found ${zones.length} active Cloudflare domains (${zones.map((z) => z.name).join(", ")}). Click below to create your first tunnel node!`
                      : "Deploy your first node with our zero-touch setup: generate a one-line curl command, run it on your remote Ubuntu / lab server, and start mapping subdomains instantly!"}
                  </p>
                </div>

                <div className="max-w-xl mx-auto grid grid-cols-1 sm:grid-cols-3 gap-3 text-left">
                  <div className="p-3.5 rounded-xl bg-neutral-900/60 border border-neutral-800 space-y-1">
                    <span className="text-[10px] font-bold text-orange-400 uppercase">Step 1</span>
                    <p className="text-xs font-semibold text-white">Add Machine</p>
                    <p className="text-[11px] text-neutral-400">Name the node and select your domain</p>
                  </div>
                  <div className="p-3.5 rounded-xl bg-neutral-900/60 border border-neutral-800 space-y-1">
                    <span className="text-[10px] font-bold text-orange-400 uppercase">Step 2</span>
                    <p className="text-xs font-semibold text-white">Run Curl Command</p>
                    <p className="text-[11px] text-neutral-400">Paste in SSH; auto-installs cloudflared</p>
                  </div>
                  <div className="p-3.5 rounded-xl bg-neutral-900/60 border border-neutral-800 space-y-1">
                    <span className="text-[10px] font-bold text-orange-400 uppercase">Step 3</span>
                    <p className="text-xs font-semibold text-white">Add Subdomains</p>
                    <p className="text-[11px] text-neutral-400">Route ports without touching config.yml</p>
                  </div>
                </div>

                <div>
                  <button
                    onClick={() => {
                      if (!settings?.configured) {
                        setActiveTab("settings");
                      } else {
                        openCreateForZone();
                      }
                    }}
                    className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-xs font-semibold bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600 text-white shadow-lg shadow-orange-500/20 active:scale-95 transition-all cursor-pointer"
                  >
                    <Plus className="w-4 h-4" />
                    <span>{settings?.configured ? "Deploy First Node" : "Configure Cloudflare Settings"}</span>
                  </button>
                </div>
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
                {machines.map((m) => {
                  const isOnline = m.status === "online";
                  const isPending = m.status === "pending";

                  return (
                    <div
                      key={m.id}
                      className="bg-neutral-900/70 border border-neutral-800 hover:border-neutral-700/80 rounded-2xl p-5 flex flex-col justify-between space-y-5 transition-all shadow-lg hover:shadow-orange-500/5 group"
                    >
                      {/* Top Bar: Name & Status */}
                      <div className="space-y-3">
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex items-center gap-2.5">
                            <div className="w-9 h-9 rounded-xl bg-neutral-800/80 border border-neutral-700/60 flex items-center justify-center shrink-0">
                              <Cpu className="w-4 h-4 text-orange-400" />
                            </div>
                            <div>
                              <h3 className="text-sm font-bold text-white tracking-tight group-hover:text-orange-300 transition-colors">
                                {m.name}
                              </h3>
                              <p className="text-[11px] text-neutral-400 font-mono">
                                {m.zone_name || "No domain set"}
                              </p>
                            </div>
                          </div>

                          {/* Status Badge */}
                          <div className="shrink-0">
                            {isOnline ? (
                              <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[10px] font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                                Online
                              </span>
                            ) : isPending ? (
                              <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[10px] font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20">
                                <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-ping" />
                                Pending Setup
                              </span>
                            ) : (
                              <span className="flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[10px] font-semibold bg-neutral-800 text-neutral-400 border border-neutral-700">
                                <span className="w-1.5 h-1.5 rounded-full bg-neutral-500" />
                                Offline
                              </span>
                            )}
                          </div>
                        </div>

                        {/* Node details */}
                        <div className="rounded-xl bg-neutral-950/70 border border-neutral-800/80 p-3 space-y-1.5 text-[11px] font-mono">
                          <div className="flex items-center justify-between text-neutral-400">
                            <span>Tunnel ID:</span>
                            <span className="text-neutral-200" title={m.tunnel_id || ""}>
                              {m.tunnel_id ? `${m.tunnel_id.substring(0, 12)}...` : "None"}
                            </span>
                          </div>
                          <div className="flex items-center justify-between text-neutral-400">
                            <span>Host & OS:</span>
                            <span className="text-neutral-200 truncate max-w-[170px]" title={m.os_info || ""}>
                              {m.remote_hostname || (isPending ? "Awaiting SSH curl" : "Linux")}
                            </span>
                          </div>
                          <div className="flex items-center justify-between text-neutral-400">
                            <span>Remote IP:</span>
                            <span className="text-neutral-200">
                              {m.remote_ip || (isPending ? "---" : "Local/Protected")}
                            </span>
                          </div>
                          <div className="flex items-center justify-between text-neutral-400 pt-1 border-t border-neutral-800/60">
                            <span>Ingress Routes:</span>
                            <span className="text-orange-400 font-semibold">
                              {m.routes_count || 0} active
                            </span>
                          </div>
                        </div>
                      </div>

                      {/* Bottom Action Buttons */}
                      <div className="space-y-2 pt-2 border-t border-neutral-800/80">
                        <div className="grid grid-cols-2 gap-2">
                          <button
                            onClick={() => setActiveSetupMachine(m)}
                            className="flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl text-xs font-medium bg-neutral-800 hover:bg-neutral-700 text-neutral-200 transition-colors"
                            title="View one-line curl setup command"
                          >
                            <Terminal className="w-3.5 h-3.5 text-orange-400" />
                            <span>Setup Command</span>
                          </button>

                          <button
                            onClick={() => setActiveRoutesMachine(m)}
                            className="flex items-center justify-center gap-1.5 px-3 py-2 rounded-xl text-xs font-semibold bg-orange-500/10 hover:bg-orange-500/20 text-orange-400 border border-orange-500/30 transition-colors"
                            title="Manage subdomains and local port mappings"
                          >
                            <Network className="w-3.5 h-3.5" />
                            <span>Subdomains</span>
                          </button>
                        </div>

                        <div className="flex items-center justify-between pt-1">
                          <button
                            onClick={() => copyQuickCurl(m)}
                            className="text-[11px] text-neutral-400 hover:text-white flex items-center gap-1 py-1 px-1.5 rounded transition-colors"
                          >
                            {copiedId === m.id ? (
                              <>
                                <Check className="w-3 h-3 text-emerald-400" />
                                <span className="text-emerald-400 font-medium">Command Copied!</span>
                              </>
                            ) : (
                              <>
                                <Copy className="w-3 h-3 text-neutral-500" />
                                <span>Copy 1-line curl</span>
                              </>
                            )}
                          </button>

                          <button
                            onClick={() => handleDeleteMachine(m.id, m.name)}
                            className="p-1 text-neutral-500 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition-colors"
                            title="Delete machine & Cloudflare tunnel"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}
      </main>

      {/* Modals */}
      {showCreateModal && (
        <CreateMachineModal
          zones={zones}
          serverUrl={settings?.server_url || ""}
          initialZoneId={preselectedZoneId}
          onClose={() => {
            setShowCreateModal(false);
            setPreselectedZoneId(undefined);
          }}
          onCreated={(machine) => {
            setShowCreateModal(false);
            setPreselectedZoneId(undefined);
            loadAll();
            setActiveSetupMachine(machine);
          }}
        />
      )}

      {activeSetupMachine && (
        <SetupCommandModal
          machine={activeSetupMachine}
          defaultServerUrl={settings?.server_url || ""}
          onClose={() => setActiveSetupMachine(null)}
          onRegistered={() => {
            loadAll();
          }}
        />
      )}

      {activeRoutesMachine && (
        <RoutesModal
          machine={activeRoutesMachine}
          onClose={() => setActiveRoutesMachine(null)}
          onRoutesUpdated={() => {
            loadAll();
          }}
        />
      )}
    </div>
  );
}
