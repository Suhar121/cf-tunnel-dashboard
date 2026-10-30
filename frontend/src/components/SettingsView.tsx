"use client";

import React, { useState, useEffect } from "react";
import {
  Key,
  Shield,
  CheckCircle2,
  AlertTriangle,
  Loader2,
  ExternalLink,
  Save,
  Globe,
  Radio,
  Server,
  RefreshCw,
} from "lucide-react";
import {
  CloudflareSettings,
  CloudflareZone,
  fetchSettings,
  fetchZones,
  saveSettings,
  testCloudflareToken,
} from "@/lib/api";

interface SettingsViewProps {
  onConfigured: () => void;
}

export const SettingsView: React.FC<SettingsViewProps> = ({ onConfigured }) => {
  const [apiToken, setApiToken] = useState("");
  const [accountId, setAccountId] = useState("");
  const [serverUrl, setServerUrl] = useState("");
  const [adminEmail, setAdminEmail] = useState("");
  const [detectedLanIp, setDetectedLanIp] = useState("192.168.29.143");
  const [currentConfig, setCurrentConfig] = useState<CloudflareSettings | null>(null);
  const [zones, setZones] = useState<CloudflareZone[]>([]);

  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    valid: boolean;
    accounts?: Array<{ id: string; name: string }>;
    error?: string;
  } | null>(null);
  const [message, setMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      setLoading(true);
      const data = await fetchSettings();
      setCurrentConfig(data);
      if (data.detected_lan_ip) {
        setDetectedLanIp(data.detected_lan_ip);
      }
      if (data.account_id) setAccountId(data.account_id);
      if (data.admin_email) setAdminEmail(data.admin_email);
      if (data.server_url) {
        setServerUrl(data.server_url);
      } else if (data.detected_lan_ip) {
        setServerUrl(`http://${data.detected_lan_ip}:8080`);
      } else if (typeof window !== "undefined") {
        setServerUrl(`${window.location.protocol}//${window.location.hostname}:8080`);
      }

      if (data.configured) {
        try {
          const z = await fetchZones();
          setZones(z);
        } catch {
          // ignore
        }
      }
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  };

  const handleTestToken = async () => {
    if (!apiToken.trim()) {
      setMessage({ type: "error", text: "Please enter an API token to test" });
      return;
    }

    try {
      setTesting(true);
      setTestResult(null);
      const res = await testCloudflareToken(apiToken.trim());
      setTestResult({
        valid: res.valid,
        accounts: res.accounts,
      });

      if (res.accounts && res.accounts.length === 1 && !accountId) {
        setAccountId(res.accounts[0].id);
      }
    } catch (err: any) {
      setTestResult({
        valid: false,
        error: err.message || "Failed to verify token",
      });
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!apiToken.trim() && !currentConfig?.configured) {
      setMessage({ type: "error", text: "API Token is required" });
      return;
    }
    if (!accountId.trim()) {
      setMessage({ type: "error", text: "Account ID is required" });
      return;
    }

    try {
      setSaving(true);
      setMessage(null);
      await saveSettings({
        api_token: apiToken.trim() || "",
        account_id: accountId.trim(),
        server_url: serverUrl.trim() || undefined,
        admin_email: adminEmail.trim() || undefined,
      });

      setMessage({ type: "success", text: "Cloudflare settings saved and validated successfully!" });
      await loadSettings();
      onConfigured();
    } catch (err: any) {
      setMessage({ type: "error", text: err.message || "Failed to save settings" });
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="py-24 flex items-center justify-center text-neutral-400 gap-2">
        <Loader2 className="w-5 h-5 animate-spin text-orange-400" />
        <span className="text-sm">Loading Cloudflare settings...</span>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <div>
        <h2 className="text-xl font-bold tracking-tight text-white flex items-center gap-2.5">
          <Key className="w-5 h-5 text-orange-400" />
          Cloudflare API & Dashboard Setup
        </h2>
        <p className="text-xs text-neutral-400 mt-1">
          Connect your Cloudflare Zero Trust account to enable automated tunnel creation, token generation, and DNS management.
        </p>
      </div>

      {message && (
        <div
          className={`p-4 rounded-xl text-xs flex items-center gap-2.5 ${
            message.type === "success"
              ? "bg-emerald-500/10 border border-emerald-500/20 text-emerald-300"
              : "bg-rose-500/10 border border-rose-500/20 text-rose-300"
          }`}
        >
          {message.type === "success" ? (
            <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
          ) : (
            <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0" />
          )}
          <span>{message.text}</span>
        </div>
      )}

      {/* Settings Form */}
      <form onSubmit={handleSave} className="bg-neutral-900 border border-neutral-800 rounded-2xl p-6 space-y-6 shadow-xl">
        <div className="flex items-center justify-between pb-4 border-b border-neutral-800">
          <div className="flex items-center gap-2.5">
            <Shield className="w-4 h-4 text-orange-400" />
            <h3 className="text-sm font-semibold text-white">Credentials & Token Configuration</h3>
          </div>
          <div className="flex items-center gap-2">
            <span
              className={`w-2.5 h-2.5 rounded-full ${
                currentConfig?.configured ? "bg-emerald-400" : "bg-amber-400"
              }`}
            />
            <span className="text-xs font-medium text-neutral-300">
              {currentConfig?.configured ? "Connected" : "Unconfigured"}
            </span>
          </div>
        </div>

        {/* API Token Input */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-neutral-300">Cloudflare API Token</label>
            <a
              href="https://dash.cloudflare.com/profile/api-tokens"
              target="_blank"
              rel="noopener noreferrer"
              className="text-[11px] text-orange-400 hover:text-orange-300 flex items-center gap-1"
            >
              <span>Create Token in Cloudflare Dashboard</span>
              <ExternalLink className="w-3 h-3" />
            </a>
          </div>
          <div className="flex gap-2">
            <input
              type="password"
              value={apiToken}
              onChange={(e) => setApiToken(e.target.value)}
              placeholder={currentConfig?.masked_token ? `Current: ${currentConfig.masked_token}` : "Paste API Token (Bearer ...)"}
              className="w-full bg-neutral-950 border border-neutral-800 rounded-xl px-4 py-2.5 text-xs text-neutral-200 focus:outline-none focus:border-orange-500/60 font-mono"
            />
            <button
              type="button"
              onClick={handleTestToken}
              disabled={testing || !apiToken}
              className="shrink-0 px-4 py-2 bg-neutral-800 hover:bg-neutral-700 text-neutral-200 text-xs font-medium rounded-xl transition-all disabled:opacity-40 flex items-center gap-2 cursor-pointer"
            >
              {testing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Radio className="w-3.5 h-3.5 text-orange-400" />}
              <span>Test Token</span>
            </button>
          </div>
          <div className="text-[11px] text-neutral-400 space-y-1 bg-neutral-950/60 border border-neutral-800/80 rounded-xl p-3">
            <span className="font-semibold text-neutral-300">Required Token Permissions:</span>
            <ul className="list-disc list-inside space-y-0.5 text-neutral-400 pl-1">
              <li><strong className="text-orange-400">Account</strong> ➔ <strong className="text-neutral-200">Cloudflare Tunnel</strong> ➔ <strong className="text-neutral-200">Edit</strong> (creates & manages tunnels)</li>
              <li><strong className="text-orange-400">Zone</strong> ➔ <strong className="text-neutral-200">DNS</strong> ➔ <strong className="text-neutral-200">Edit</strong> (creates CNAME subdomain records)</li>
              <li><strong className="text-orange-400">Account</strong> ➔ <strong className="text-neutral-200">Access: Apps and Policies</strong> ➔ <strong className="text-neutral-200">Edit</strong> (enables 1-click Zero Trust Email PIN)</li>
            </ul>
          </div>
        </div>

        {/* Test Result Box */}
        {testResult && (
          <div
            className={`p-3.5 rounded-xl border text-xs space-y-2 ${
              testResult.valid
                ? "bg-emerald-500/10 border-emerald-500/20 text-emerald-300"
                : "bg-rose-500/10 border border-rose-500/20 text-rose-300"
            }`}
          >
            <div className="flex items-center gap-2 font-medium">
              {testResult.valid ? <CheckCircle2 className="w-4 h-4 text-emerald-400" /> : <AlertTriangle className="w-4 h-4 text-rose-400" />}
              <span>{testResult.valid ? "Token is valid and active!" : testResult.error || "Token invalid"}</span>
            </div>
            {testResult.accounts && testResult.accounts.length > 0 && (
              <div className="text-[11px] space-y-1 text-neutral-300">
                <span className="text-neutral-400">Available Accounts:</span>
                <div className="flex flex-wrap gap-1.5 mt-1">
                  {testResult.accounts.map((acc) => (
                    <button
                      key={acc.id}
                      type="button"
                      onClick={() => setAccountId(acc.id)}
                      className="px-2.5 py-1 bg-neutral-900 border border-neutral-700 rounded-lg hover:border-orange-400 font-mono text-[11px] text-neutral-200 transition-colors"
                    >
                      {acc.name} ({acc.id.substring(0, 8)}...)
                    </button>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Account ID */}
        <div className="space-y-1.5">
          <label className="text-xs font-medium text-neutral-300">Cloudflare Account ID (32-character Hex)</label>
          <input
            type="text"
            required
            value={accountId}
            onChange={(e) => setAccountId(e.target.value)}
            placeholder="e.g. 6f86803dd1cba77b41ce65908266d8e3"
            className="w-full bg-neutral-950 border border-neutral-800 rounded-xl px-4 py-2.5 text-xs text-neutral-200 focus:outline-none focus:border-orange-500/60 font-mono"
          />
          <p className="text-[11px] text-neutral-500">
            Found on the right sidebar of any domain overview in your Cloudflare dashboard.
          </p>
        </div>

        {/* Default Admin Email for Zero Trust */}
        <div className="space-y-1.5">
          <div className="flex items-center justify-between">
            <label className="text-xs font-medium text-neutral-300 flex items-center gap-1.5">
              <Shield className="w-3.5 h-3.5 text-orange-400" />
              Default Admin Email (Cloudflare Zero Trust Access)
            </label>
            <span className="text-[10px] text-neutral-500">Optional / Recommended</span>
          </div>
          <input
            type="email"
            value={adminEmail}
            onChange={(e) => setAdminEmail(e.target.value)}
            placeholder="e.g. admin@example.com"
            className="w-full bg-neutral-950 border border-neutral-800 rounded-xl px-4 py-2.5 text-xs text-neutral-200 focus:outline-none focus:border-orange-500/60 font-mono"
          />
          <p className="text-[11px] text-neutral-500">
            When protecting subdomains with Cloudflare Access, this email receives the 1-time login PIN OTP. Requires <span className="text-neutral-300">Account.Access: Apps and Policies: Edit</span> token permission.
          </p>
        </div>

        {/* Dashboard Server Base URL */}
        <div className="space-y-1.5 pt-2 border-t border-neutral-800/80">
          <label className="text-xs font-medium text-neutral-300 flex items-center justify-between">
            <span className="flex items-center gap-1.5">
              <Server className="w-3.5 h-3.5 text-neutral-400" />
              Dashboard Reachable Address (Server URL)
            </span>
            <span className="text-[11px] text-neutral-500">Used for generated curl commands</span>
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              placeholder={`http://${detectedLanIp}:8080`}
              className="w-full bg-neutral-950 border border-neutral-800 rounded-xl px-4 py-2.5 text-xs text-neutral-200 focus:outline-none focus:border-orange-500/60 font-mono"
            />
            <button
              type="button"
              onClick={() => setServerUrl(`http://${detectedLanIp}:8080`)}
              className="shrink-0 px-3 py-2 bg-neutral-800 hover:bg-neutral-700 text-orange-300 text-xs font-mono font-medium rounded-xl transition-all cursor-pointer border border-neutral-700"
              title="Auto-fill with laptop Wi-Fi IP address"
            >
              Use Laptop IP ({detectedLanIp})
            </button>
          </div>
          <p className="text-[11px] text-neutral-500">
            For remote Ubuntu/lab machines, use your laptop&apos;s Wi-Fi IP (<code className="text-orange-300 font-mono">http://{detectedLanIp}:8080</code>) instead of <code className="text-neutral-400 font-mono">localhost</code> so the remote machine can connect.
          </p>
        </div>

        {/* Connected Domains Overview */}
        {zones.length > 0 && (
          <div className="space-y-2 pt-3 border-t border-neutral-800/80">
            <div className="flex items-center justify-between">
              <label className="text-xs font-semibold text-neutral-200 flex items-center gap-1.5">
                <Globe className="w-3.5 h-3.5 text-purple-400" />
                Verified Connected Domains ({zones.length})
              </label>
              <span className="text-[10px] text-emerald-400 font-medium">Ready for Tunnels</span>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
              {zones.map((z) => (
                <div
                  key={z.id}
                  className="p-3 bg-neutral-950 border border-neutral-800 rounded-xl flex items-center justify-between font-mono text-xs"
                >
                  <div className="flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-emerald-400" />
                    <span className="text-neutral-200 font-medium">{z.name}</span>
                  </div>
                  <span className="text-[10px] font-sans px-2 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 uppercase font-semibold">
                    {z.status}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Save button */}
        <div className="pt-4 flex items-center justify-end">
          <button
            type="submit"
            disabled={saving}
            className="flex items-center gap-2 px-5 py-2.5 rounded-xl text-xs font-semibold bg-gradient-to-r from-orange-500 to-amber-500 hover:from-orange-600 hover:to-amber-600 text-white shadow-md shadow-orange-500/20 active:scale-95 transition-all disabled:opacity-50 cursor-pointer"
          >
            {saving ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Saving & Verifying...</span>
              </>
            ) : (
              <>
                <Save className="w-4 h-4" />
                <span>Save Configuration</span>
              </>
            )}
          </button>
        </div>
      </form>
    </div>
  );
};
