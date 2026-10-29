"use client";

import React, { useState, useEffect } from "react";
import { Server, Globe, X, ArrowRight, Loader2, Sparkles, CheckCircle2 } from "lucide-react";
import { CloudflareZone, createMachine, Machine } from "@/lib/api";

interface CreateMachineModalProps {
  zones: CloudflareZone[];
  serverUrl: string;
  initialZoneId?: string;
  onClose: () => void;
  onCreated: (machine: Machine) => void;
}

export const CreateMachineModal: React.FC<CreateMachineModalProps> = ({
  zones,
  serverUrl,
  initialZoneId,
  onClose,
  onCreated,
}) => {
  const [name, setName] = useState("");
  const [selectedZoneId, setSelectedZoneId] = useState(initialZoneId || zones[0]?.id || "");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!selectedZoneId && zones.length > 0) {
      setSelectedZoneId(initialZoneId || zones[0].id);
    }
  }, [zones, selectedZoneId, initialZoneId]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError("Please enter a name for the node");
      return;
    }

    setLoading(true);
    setError(null);

    const activeZoneId = selectedZoneId || zones[0]?.id;
    const zone = zones.find((z) => z.id === activeZoneId) || zones[0];

    try {
      const res = await createMachine({
        name: name.trim(),
        zone_id: zone?.id,
        zone_name: zone?.name,
        server_url: serverUrl.trim() || undefined,
      });

      onCreated(res.machine);
    } catch (err: any) {
      setError(err.message || "Failed to create machine tunnel");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="bg-neutral-900 border border-neutral-800 rounded-2xl w-full max-w-lg overflow-hidden shadow-2xl flex flex-col">
        {/* Header */}
        <div className="px-6 py-4 border-b border-neutral-800 flex items-center justify-between bg-neutral-950/50">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-orange-500/10 border border-orange-500/20 flex items-center justify-center">
              <Server className="w-4 h-4 text-orange-400" />
            </div>
            <div>
              <h3 className="text-base font-semibold text-white">Add New Remote Node</h3>
              <p className="text-xs text-neutral-400">Instantly provision Cloudflare tunnel & token</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-neutral-400 hover:text-white p-1 rounded-lg hover:bg-neutral-800 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-6 space-y-5">
          {error && (
            <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-300 text-xs">
              {error}
            </div>
          )}

          {/* Machine Name */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-neutral-300 flex items-center justify-between">
              <span>Node / Server Name</span>
              <span className="text-[11px] text-neutral-500">e.g. ubuntu-lab, my-vps, lab-node</span>
            </label>
            <div className="relative">
              <input
                type="text"
                required
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="ubuntu-lab-01"
                className="w-full pl-3.5 pr-20 py-2.5 bg-neutral-950 border border-neutral-800 rounded-xl text-xs text-neutral-200 focus:outline-none focus:border-orange-500/60 font-mono"
              />
              <button
                type="button"
                onClick={() => setName(`server-${Math.floor(Math.random() * 900 + 100)}`)}
                className="absolute right-2 top-2 px-2 py-1 text-[10px] font-medium bg-neutral-800 hover:bg-neutral-700 text-neutral-300 rounded-md transition-colors"
              >
                Random
              </button>
            </div>
          </div>

          {/* Domain Selection */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-neutral-300 flex items-center gap-1.5">
                <Globe className="w-3.5 h-3.5 text-orange-400" />
                <span>Assign to Cloudflare Domain</span>
              </label>
              <span className="text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-medium flex items-center gap-1">
                <CheckCircle2 className="w-3 h-3" />
                {zones.length} Domains Loaded
              </span>
            </div>

            {zones.length === 0 ? (
              <div className="p-3 bg-amber-500/10 border border-amber-500/20 rounded-xl text-xs text-amber-300">
                Fetching domains from Cloudflare...
              </div>
            ) : (
              <div className="space-y-2">
                <select
                  value={selectedZoneId || zones[0]?.id}
                  onChange={(e) => setSelectedZoneId(e.target.value)}
                  className="w-full px-3.5 py-2.5 bg-neutral-950 border border-neutral-800 rounded-xl text-xs text-neutral-200 focus:outline-none focus:border-orange-500/60 font-mono font-medium"
                >
                  {zones.map((z) => (
                    <option key={z.id} value={z.id}>
                      {z.name} ({z.status.toUpperCase()})
                    </option>
                  ))}
                </select>
                <div className="flex flex-wrap gap-1.5">
                  {zones.map((z) => (
                    <button
                      key={z.id}
                      type="button"
                      onClick={() => setSelectedZoneId(z.id)}
                      className={`px-2.5 py-1 rounded-lg text-[11px] font-mono border transition-all ${
                        (selectedZoneId || zones[0]?.id) === z.id
                          ? "bg-orange-500/10 border-orange-500/40 text-orange-300"
                          : "bg-neutral-950 border-neutral-800 text-neutral-400 hover:text-neutral-200"
                      }`}
                    >
                      {z.name}
                    </button>
                  ))}
                </div>
              </div>
            )}
            <p className="text-[11px] text-neutral-500">
              Any subdomains you add later will be created under this domain on Cloudflare automatically.
            </p>
          </div>

          <div className="rounded-xl bg-orange-500/5 border border-orange-500/10 p-3.5 flex items-start gap-2.5">
            <Sparkles className="w-4 h-4 text-orange-400 shrink-0 mt-0.5" />
            <p className="text-[11px] text-neutral-400 leading-relaxed">
              Once created, a ready-to-run <code className="text-orange-300 font-mono">curl</code> command will appear. Run it in your remote SSH terminal to auto-install cloudflared.
            </p>
          </div>

          {/* Submit */}
          <div className="pt-2 flex items-center justify-end gap-3">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-xs font-medium text-neutral-400 hover:text-white hover:bg-neutral-800 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading || zones.length === 0}
              className="flex items-center gap-2 px-5 py-2 rounded-xl text-xs font-semibold bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600 text-white shadow-md shadow-orange-500/20 active:scale-95 disabled:opacity-50 transition-all cursor-pointer"
            >
              {loading ? (
                <>
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  Creating Tunnel in Cloudflare...
                </>
              ) : (
                <>
                  <span>Create & Generate Command</span>
                  <ArrowRight className="w-3.5 h-3.5" />
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
