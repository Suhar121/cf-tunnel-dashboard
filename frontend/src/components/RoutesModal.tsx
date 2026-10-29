"use client";

import React, { useState, useEffect } from "react";
import {
  Network,
  Plus,
  Trash2,
  RefreshCw,
  ExternalLink,
  X,
  CheckCircle2,
  AlertCircle,
  Zap,
  ArrowRight,
  Loader2,
} from "lucide-react";
import {
  Machine,
  IngressRoute,
  fetchMachineDetails,
  addRoute,
  deleteRoute,
  syncMachine,
} from "@/lib/api";

interface RoutesModalProps {
  machine: Machine | null;
  onClose: () => void;
  onRoutesUpdated: () => void;
}

export const RoutesModal: React.FC<RoutesModalProps> = ({
  machine,
  onClose,
  onRoutesUpdated,
}) => {
  const [routes, setRoutes] = useState<IngressRoute[]>([]);
  const [loading, setLoading] = useState(true);
  const [subdomain, setSubdomain] = useState("");
  const [service, setService] = useState("http://localhost:8000");
  const [adding, setAdding] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadData = async () => {
    if (!machine) return;
    try {
      setLoading(true);
      const res = await fetchMachineDetails(machine.id);
      setRoutes(res.routes);
    } catch (err: any) {
      setError(err.message || "Failed to load routes");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, [machine]);

  if (!machine) return null;

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!subdomain.trim() || !service.trim()) return;

    setAdding(true);
    setError(null);

    try {
      await addRoute(machine.id, {
        subdomain: subdomain.trim(),
        service: service.trim(),
      });
      setSubdomain("");
      await loadData();
      onRoutesUpdated();
    } catch (err: any) {
      setError(err.message || "Failed to create route");
    } finally {
      setAdding(false);
    }
  };

  const handleDelete = async (routeId: string) => {
    if (!confirm("Are you sure you want to remove this route? This will delete the Cloudflare DNS record.")) return;

    try {
      await deleteRoute(machine.id, routeId);
      await loadData();
      onRoutesUpdated();
    } catch (err: any) {
      setError(err.message || "Failed to delete route");
    }
  };

  const handleSyncAll = async () => {
    try {
      setSyncing(true);
      await syncMachine(machine.id);
      await loadData();
      onRoutesUpdated();
    } catch (err: any) {
      setError(err.message || "Failed to sync with Cloudflare");
    } finally {
      setSyncing(false);
    }
  };

  const applyPreset = (sub: string, srv: string) => {
    setSubdomain(sub);
    setService(srv);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-3xl overflow-hidden shadow-2xl flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="px-6 py-4 border-b border-neutral-800 flex items-center justify-between bg-neutral-950/50">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-orange-500/10 border border-orange-500/20 flex items-center justify-center">
              <Network className="w-4 h-4 text-orange-400" />
            </div>
            <div>
              <h3 className="text-base font-semibold text-white">Ingress & Subdomains Manager</h3>
              <p className="text-xs text-neutral-400">
                Node: <span className="text-orange-400 font-medium">{machine.name}</span> | Domain:{" "}
                <span className="text-neutral-300 font-mono">{machine.zone_name || "Not assigned"}</span>
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleSyncAll}
              disabled={syncing}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-neutral-800 hover:bg-neutral-700 text-neutral-200 transition-colors"
              title="Force re-sync DNS and ingress rules to Cloudflare"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${syncing ? "animate-spin text-orange-400" : ""}`} />
              <span>Sync Cloudflare</span>
            </button>
            <button
              onClick={onClose}
              className="text-neutral-400 hover:text-white p-1 rounded-lg hover:bg-neutral-800 transition-colors"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Content Body */}
        <div className="p-6 overflow-y-auto space-y-6">
          {error && (
            <div className="p-3.5 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-300 text-xs flex items-center justify-between">
              <span>{error}</span>
              <button onClick={() => setError(null)} className="text-rose-400 hover:text-white">
                <X className="w-4 h-4" />
              </button>
            </div>
          )}

          {/* Add Route Box */}
          <div className="bg-neutral-950/70 border border-neutral-800/80 rounded-xl p-4 space-y-4">
            <div className="flex items-center justify-between">
              <span className="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
                <Zap className="w-3.5 h-3.5 text-orange-400" />
                Add Subdomain & Port Mapping
              </span>
              <div className="flex items-center gap-1.5 text-[11px] text-neutral-400">
                <span>Presets:</span>
                <button
                  type="button"
                  onClick={() => applyPreset("app", "http://localhost:3000")}
                  className="px-2 py-0.5 rounded bg-neutral-900 hover:bg-neutral-800 text-neutral-300 border border-neutral-800 font-mono"
                >
                  app:3000
                </button>
                <button
                  type="button"
                  onClick={() => applyPreset("api", "http://localhost:8000")}
                  className="px-2 py-0.5 rounded bg-neutral-900 hover:bg-neutral-800 text-neutral-300 border border-neutral-800 font-mono"
                >
                  api:8000
                </button>
                <button
                  type="button"
                  onClick={() => applyPreset("ssh", "ssh://localhost:22")}
                  className="px-2 py-0.5 rounded bg-neutral-900 hover:bg-neutral-800 text-neutral-300 border border-neutral-800 font-mono"
                >
                  ssh:22
                </button>
              </div>
            </div>

            <form onSubmit={handleAdd} className="grid grid-cols-1 sm:grid-cols-12 gap-3 items-end">
              <div className="sm:col-span-5 space-y-1">
                <label className="text-[11px] font-medium text-neutral-400">Subdomain prefix</label>
                <div className="flex items-center bg-neutral-900 border border-neutral-800 rounded-lg px-3 py-2 text-xs focus-within:border-orange-500/60">
                  <input
                    type="text"
                    required
                    value={subdomain}
                    onChange={(e) => setSubdomain(e.target.value.toLowerCase())}
                    placeholder="api (or @ for root)"
                    className="bg-transparent w-full text-neutral-200 focus:outline-none font-mono"
                  />
                  <span className="text-neutral-500 font-mono shrink-0 pl-1">
                    .{machine.zone_name || "domain"}
                  </span>
                </div>
              </div>

              <div className="sm:col-span-4 space-y-1">
                <label className="text-[11px] font-medium text-neutral-400">Target Local Service</label>
                <input
                  type="text"
                  required
                  value={service}
                  onChange={(e) => setService(e.target.value)}
                  placeholder="http://localhost:8000"
                  className="w-full bg-neutral-900 border border-neutral-800 rounded-lg px-3 py-2 text-xs text-neutral-200 focus:outline-none focus:border-orange-500/60 font-mono"
                />
              </div>

              <div className="sm:col-span-3">
                <button
                  type="submit"
                  disabled={adding}
                  className="w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-lg text-xs font-semibold bg-orange-500 hover:bg-orange-600 text-white shadow-sm transition-all active:scale-95 disabled:opacity-50 cursor-pointer"
                >
                  {adding ? (
                    <>
                      <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      <span>Syncing...</span>
                    </>
                  ) : (
                    <>
                      <Plus className="w-3.5 h-3.5" />
                      <span>Add Route</span>
                    </>
                  )}
                </button>
              </div>
            </form>
          </div>

          {/* Current Ingress Routes Table */}
          <div className="space-y-3">
            <h4 className="text-xs font-semibold text-neutral-300 uppercase tracking-wider">
              Active Cloudflare Ingress Rules ({routes.length})
            </h4>

            {loading ? (
              <div className="py-12 flex items-center justify-center text-neutral-500 gap-2">
                <Loader2 className="w-4 h-4 animate-spin text-orange-400" />
                <span className="text-xs">Loading routes...</span>
              </div>
            ) : routes.length === 0 ? (
              <div className="py-10 border border-dashed border-neutral-800 rounded-xl text-center space-y-2">
                <p className="text-xs text-neutral-400">No subdomains mapped yet for this node.</p>
                <p className="text-[11px] text-neutral-500">
                  Add one above (e.g. <code className="text-neutral-400">api</code> mapping to <code className="text-neutral-400">http://localhost:8000</code>).
                </p>
              </div>
            ) : (
              <div className="divide-y divide-neutral-800/80 border border-neutral-800 rounded-xl overflow-hidden bg-neutral-950/40">
                {routes.map((r) => (
                  <div
                    key={r.id}
                    className="p-3.5 flex items-center justify-between hover:bg-neutral-900/40 transition-colors"
                  >
                    <div className="flex items-center gap-3">
                      <div className="w-2 h-2 rounded-full bg-emerald-400" />
                      <div>
                        <div className="flex items-center gap-2">
                          <a
                            href={`https://${r.hostname}`}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="font-mono text-xs font-semibold text-white hover:text-orange-400 flex items-center gap-1 group"
                          >
                            <span>{r.hostname}</span>
                            <ExternalLink className="w-3 h-3 text-neutral-500 group-hover:text-orange-400" />
                          </a>
                          <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium">
                            Cloudflare CNAME Active
                          </span>
                        </div>
                        <div className="text-[11px] text-neutral-400 flex items-center gap-1.5 mt-0.5 font-mono">
                          <span className="text-neutral-500">Proxies to</span>
                          <span className="text-orange-300">{r.service}</span>
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => handleDelete(r.id)}
                        className="p-1.5 text-neutral-500 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition-colors"
                        title="Delete route and remove Cloudflare DNS record"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-neutral-800 bg-neutral-950/50 flex items-center justify-between">
          <span className="text-[11px] text-neutral-500">
            Changes propagate to Cloudflare edge global network in real time.
          </span>
          <button
            onClick={onClose}
            className="px-4 py-2 rounded-xl text-xs font-medium bg-neutral-800 hover:bg-neutral-700 text-white transition-colors"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
};

