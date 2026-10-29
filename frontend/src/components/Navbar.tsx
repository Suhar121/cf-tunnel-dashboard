"use client";

import React, { useState, useEffect } from "react";
import { Server, Settings, Terminal, Plus, ShieldCheck, Wifi } from "lucide-react";
import { fetchNetworkInfo } from "@/lib/api";

interface NavbarProps {
  activeTab: "machines" | "settings" | "logs";
  setActiveTab: (tab: "machines" | "settings" | "logs") => void;
  onOpenCreate: () => void;
  isConfigured: boolean;
  totalMachines: number;
  onlineMachines: number;
}

export const Navbar: React.FC<NavbarProps> = ({
  activeTab,
  setActiveTab,
  onOpenCreate,
  isConfigured,
  totalMachines,
  onlineMachines,
}) => {
  const [laptopIp, setLaptopIp] = useState("192.168.29.143");

  useEffect(() => {
    fetchNetworkInfo()
      .then((net) => {
        if (net.local_ip && net.local_ip !== "127.0.0.1") {
          setLaptopIp(net.local_ip);
        }
      })
      .catch(() => {});
  }, []);

  return (
    <header className="sticky top-0 z-40 border-b border-neutral-800 bg-neutral-950/80 backdrop-blur-md">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 h-16 flex items-center justify-between">
        {/* Brand */}
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-amber-500 via-orange-600 to-rose-600 p-[1px] flex items-center justify-center shadow-lg shadow-orange-500/10">
            <div className="w-full h-full bg-neutral-950 rounded-xl flex items-center justify-center">
              <ShieldCheck className="w-5 h-5 text-orange-500" />
            </div>
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="font-bold text-base tracking-tight text-white">CF Tunnel Central</span>
              <span className="text-[10px] font-semibold tracking-wider uppercase px-2 py-0.5 rounded-full bg-orange-500/10 text-orange-400 border border-orange-500/20">
                Rust + Next.js
              </span>
            </div>
            <div className="flex items-center gap-2 text-[11px] text-neutral-400">
              <span>Zero-Touch Auto-Provisioner</span>
              <span className="text-neutral-600">•</span>
              <span className="flex items-center gap-1 text-orange-300 font-mono" title="Your laptop Wi-Fi IP address">
                <Wifi className="w-3 h-3 text-orange-400" />
                Laptop IP: {laptopIp}
              </span>
            </div>
          </div>
        </div>

        {/* Navigation Tabs */}
        <nav className="flex items-center gap-1 bg-neutral-900/80 p-1 rounded-xl border border-neutral-800">
          <button
            onClick={() => setActiveTab("machines")}
            className={`flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
              activeTab === "machines"
                ? "bg-neutral-800 text-white shadow-sm"
                : "text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50"
            }`}
          >
            <Server className="w-3.5 h-3.5 text-orange-400" />
            Nodes & Tunnels
            <span className="ml-1 text-[10px] px-1.5 py-0.2 rounded-md bg-neutral-700/60 text-neutral-300">
              {onlineMachines}/{totalMachines}
            </span>
          </button>

          <button
            onClick={() => setActiveTab("settings")}
            className={`flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
              activeTab === "settings"
                ? "bg-neutral-800 text-white shadow-sm"
                : "text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50"
            }`}
          >
            <Settings className="w-3.5 h-3.5 text-neutral-400" />
            Cloudflare API
            {!isConfigured && (
              <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
            )}
          </button>

          <button
            onClick={() => setActiveTab("logs")}
            className={`flex items-center gap-2 px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
              activeTab === "logs"
                ? "bg-neutral-800 text-white shadow-sm"
                : "text-neutral-400 hover:text-neutral-200 hover:bg-neutral-800/50"
            }`}
          >
            <Terminal className="w-3.5 h-3.5 text-neutral-400" />
            Audit Logs
          </button>
        </nav>

        {/* Action Button */}
        <div className="flex items-center gap-3">
          <button
            onClick={onOpenCreate}
            disabled={!isConfigured}
            className={`flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold shadow-md transition-all ${
              isConfigured
                ? "bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600 text-white shadow-orange-500/20 active:scale-95 cursor-pointer"
                : "bg-neutral-800 text-neutral-500 cursor-not-allowed border border-neutral-700/50"
            }`}
            title={isConfigured ? "Deploy a new machine tunnel" : "Configure Cloudflare token in Settings first"}
          >
            <Plus className="w-4 h-4" />
            Add Server / Lab Node
          </button>
        </div>
      </div>
    </header>
  );
};
