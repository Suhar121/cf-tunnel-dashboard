use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareSettings {
    pub api_token: String,
    pub account_id: String,
    pub server_url: Option<String>, // e.g. http://192.168.1.50:8080 or public IP/domain
    pub admin_email: Option<String>, // Default email for Zero Trust Access policies
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub setup_token: String,
    pub tunnel_id: Option<String>,
    pub tunnel_name: Option<String>,
    pub tunnel_token: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub status: String, // "pending", "online", "offline", "error"
    pub remote_ip: Option<String>,
    pub remote_hostname: Option<String>,
    pub os_info: Option<String>,
    pub created_at: String,
    pub last_seen: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressRoute {
    pub id: String,
    pub machine_id: String,
    pub subdomain: String, // e.g. "api", "grafana", or "@" for root
    pub hostname: String,  // full FQDN: "api.domain.com"
    pub service: String,   // "http://localhost:8000" or "ssh://localhost:22"
    pub dns_record_id: Option<String>,
    pub access_app_id: Option<String>,
    pub is_protected: bool,
    pub allowed_emails: Option<String>,
    pub status: String,    // "synced", "pending", "failed"
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub id: String,
    pub machine_id: String,
    pub name: String,
    pub port: u16,
    pub protocol: String,
    pub source: String, // "docker" | "system"
    pub suggested_subdomain: String,
    pub is_exposed: bool,
    pub last_seen: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateMachineRequest {
    pub name: String,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub server_url: Option<String>, // Override or use detected/saved server url
}

#[derive(Debug, Deserialize)]
pub struct CreateRouteRequest {
    pub subdomain: String,
    pub service: String,
    pub protect: Option<bool>,
    pub allowed_emails: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleRouteProtectRequest {
    pub protect: bool,
    pub allowed_emails: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DiscoveredServiceItem {
    pub name: String,
    pub port: u16,
    pub protocol: Option<String>,
    pub source: Option<String>,
    pub suggested_subdomain: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentServicesPayload {
    pub setup_token: String,
    pub services: Vec<DiscoveredServiceItem>,
}

#[derive(Debug, Deserialize)]
pub struct ExposeDiscoveredRequest {
    pub service_id: String,
    pub subdomain: String,
    pub protect: Option<bool>,
    pub allowed_emails: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAgentPayload {
    pub setup_token: String,
    pub hostname: Option<String>,
    pub os_info: Option<String>,
    pub public_ip: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HeartbeatPayload {
    pub setup_token: String,
    pub uptime: Option<u64>,
    pub load_avg: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudflareZone {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudflareAccount {
    pub id: String,
    pub name: String,
}
