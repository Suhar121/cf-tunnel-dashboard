"use client";

import React, { useState, useEffect } from "react";
import { Terminal, RefreshCw, Loader2, Clock } from "lucide-react";
import { AuditLog, fetchLogs } from "@/lib/api";

export const LogsView: React.FC = () => {
  const [logs, setLogs] = useState<AuditLog[]>([]);
  const [loading, setLoading] = useState(true);

  const loadLogs = async () => {
    try {
      setLoading(true);
      const data = await fetchLogs();
      setLogs(data);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadLogs();
    const interval = setInterval(loadLogs, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
            <Terminal className="w-5 h-5 text-orange-400" />
            Audit & System Logs
          </h2>
          <p className="text-xs text-neutral-400 mt-1">
            Real-time feed of tunnel provisioning, remote node handshakes, and Cloudflare DNS changes.
          </p>
        </div>
        <button
          onClick={loadLogs}
          disabled={loading}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-neutral-900 border border-neutral-800 text-neutral-300 hover:text-white hover:bg-neutral-800 transition-colors"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin text-orange-400" : ""}`} />
          <span>Refresh</span>
        </button>
      </div>

      <div className="bg-neutral-950 border border-neutral-800 rounded-2xl overflow-hidden shadow-xl font-mono text-xs">
        <div className="px-4 py-2.5 bg-neutral-900/60 border-b border-neutral-800 flex items-center justify-between text-[11px] text-neutral-400">
          <span>EVENT STREAM (LATEST 50)</span>
          <span>AUTO-REFRESH: 5s</span>
        </div>

        {loading && logs.length === 0 ? (
          <div className="py-20 flex items-center justify-center text-neutral-500 gap-2 font-sans text-xs">
            <Loader2 className="w-4 h-4 animate-spin text-orange-400" />
            <span>Fetching logs...</span>
          </div>
        ) : logs.length === 0 ? (
          <div className="py-16 text-center text-neutral-500 font-sans text-xs">
            No events recorded yet. Logs will appear here once tunnels or machines are created.
          </div>
        ) : (
          <div className="divide-y divide-neutral-900 max-h-[600px] overflow-y-auto">
            {logs.map((log) => {
              const dateStr = new Date(log.timestamp).toLocaleTimeString();
              const isError = log.level === "ERROR";
              return (
                <div key={log.id} className="px-4 py-2.5 hover:bg-neutral-900/40 flex items-start gap-3 transition-colors">
                  <span className="text-neutral-500 text-[11px] shrink-0 flex items-center gap-1 pt-0.5">
                    <Clock className="w-3 h-3 text-neutral-600" />
                    {dateStr}
                  </span>
                  <span
                    className={`text-[10px] uppercase font-bold px-1.5 py-0.5 rounded shrink-0 ${
                      isError
                        ? "bg-rose-500/20 text-rose-400 border border-rose-500/30"
                        : "bg-orange-500/10 text-orange-400 border border-orange-500/20"
                    }`}
                  >
                    {log.level}
                  </span>
                  <span className={`break-all ${isError ? "text-rose-300" : "text-neutral-300"}`}>
                    {log.message}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};

