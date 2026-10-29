export interface Machine {
  id: string;
  name: string;
  setup_token: string;
  tunnel_id: string | null;
  tunnel_name: string | null;
  tunnel_token?: string | null;
  zone_id: string | null;
  zone_name: string | null;
  status: "pending" | "online" | "offline" | "error";
  remote_ip: string | null;
  remote_hostname: string | null;
  os_info: string | null;
  created_at: string;
  last_seen: string | null;
  routes_count?: number;
}

export interface IngressRoute {
  id: string;
  machine_id: string;
  subdomain: string;
  hostname: string;
  service: string;
  dns_record_id: string | null;
  status: "synced" | "pending" | "dns_failed" | "config_failed";
  created_at: string;
}

export interface CloudflareSettings {
  configured: boolean;
  account_id?: string;
  masked_token?: string;
  server_url?: string;
  detected_lan_ip?: string;
}

export interface NetworkInfo {
  local_ip: string;
  backend_url: string;
  frontend_url: string;
}

export interface CloudflareZone {
  id: string;
  name: string;
  status: string;
}

export interface AuditLog {
  id: number;
  timestamp: string;
  level: string;
  message: string;
}

const API_BASE = "";

export async function fetchSettings(): Promise<CloudflareSettings> {
  const res = await fetch(`${API_BASE}/api/cloudflare/config`);
  if (!res.ok) throw new Error("Failed to fetch settings");
  return res.json();
}

export async function fetchNetworkInfo(): Promise<NetworkInfo> {
  const res = await fetch(`${API_BASE}/api/system/network`);
  if (!res.ok) throw new Error("Failed to fetch network info");
  return res.json();
}

export async function saveSettings(data: {
  api_token: string;
  account_id: string;
  server_url?: string;
}): Promise<{ success: boolean; message: string }> {
  const res = await fetch(`${API_BASE}/api/cloudflare/config`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to save settings");
  return json;
}

export async function testCloudflareToken(api_token: string) {
  const res = await fetch(`${API_BASE}/api/cloudflare/test`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ api_token }),
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Token test failed");
  return json;
}

export async function fetchZones(): Promise<CloudflareZone[]> {
  const res = await fetch(`${API_BASE}/api/cloudflare/zones`);
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to fetch zones");
  return json.zones || [];
}

export async function fetchMachines(): Promise<Machine[]> {
  const res = await fetch(`${API_BASE}/api/machines`);
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to fetch machines");
  return json.machines || [];
}

export async function fetchMachineDetails(id: string): Promise<{
  machine: Machine;
  routes: IngressRoute[];
  curl_command: string;
  direct_command?: string;
  tunnel_token?: string;
}> {
  const res = await fetch(`${API_BASE}/api/machines/${id}`);
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to fetch machine details");
  return json;
}

export async function createMachine(data: {
  name: string;
  zone_id?: string;
  zone_name?: string;
  server_url?: string;
}): Promise<{ machine: Machine; curl_command: string }> {
  const res = await fetch(`${API_BASE}/api/machines`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to create machine");
  return json;
}

export async function deleteMachine(id: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/machines/${id}`, {
    method: "DELETE",
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to delete machine");
}

export async function addRoute(
  machineId: string,
  data: { subdomain: string; service: string }
): Promise<IngressRoute> {
  const res = await fetch(`${API_BASE}/api/machines/${machineId}/routes`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to add route");
  return json;
}

export async function deleteRoute(machineId: string, routeId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/machines/${machineId}/routes/${routeId}`, {
    method: "DELETE",
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to delete route");
}

export async function syncMachine(machineId: string): Promise<void> {
  const res = await fetch(`${API_BASE}/api/machines/${machineId}/sync`, {
    method: "POST",
  });
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to sync machine");
}

export async function fetchLogs(): Promise<AuditLog[]> {
  const res = await fetch(`${API_BASE}/api/logs`);
  const json = await res.json();
  if (!res.ok) throw new Error(json.error || "Failed to fetch logs");
  return json.logs || [];
}

