use anyhow::{anyhow, Result};
use base64::Engine;
use rand::RngCore;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::Value;

use crate::models::{CloudflareAccount, CloudflareZone, IngressRoute};

pub struct CloudflareClient {
    client: reqwest::Client,
    api_token: String,
    account_id: String,
}

impl CloudflareClient {
    pub fn new(api_token: String, account_id: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_token,
            account_id,
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_token)).unwrap(),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers
    }

    pub async fn verify_token(api_token: &str) -> Result<bool> {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.cloudflare.com/client/v4/user/tokens/verify")
            .header(AUTHORIZATION, format!("Bearer {}", api_token))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Ok(false);
        }

        let body: Value = resp.json().await?;
        Ok(body.get("success").and_then(|v| v.as_bool()).unwrap_or(false))
    }

    pub async fn list_accounts(api_token: &str) -> Result<Vec<CloudflareAccount>> {
        let client = reqwest::Client::new();
        let resp = client
            .get("https://api.cloudflare.com/client/v4/accounts")
            .header(AUTHORIZATION, format!("Bearer {}", api_token))
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to list accounts");
            return Err(anyhow!("{}", err_msg));
        }

        let mut accounts = Vec::new();
        if let Some(list) = body["result"].as_array() {
            for item in list {
                if let (Some(id), Some(name)) = (item["id"].as_str(), item["name"].as_str()) {
                    accounts.push(CloudflareAccount {
                        id: id.to_string(),
                        name: name.to_string(),
                    });
                }
            }
        }
        Ok(accounts)
    }

    pub async fn list_zones(&self) -> Result<Vec<CloudflareZone>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones?account.id={}&per_page=50",
            self.account_id
        );
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to fetch zones");
            return Err(anyhow!("{}", err_msg));
        }

        let mut zones = Vec::new();
        if let Some(list) = body["result"].as_array() {
            for item in list {
                if let (Some(id), Some(name), Some(status)) = (
                    item["id"].as_str(),
                    item["name"].as_str(),
                    item["status"].as_str(),
                ) {
                    zones.push(CloudflareZone {
                        id: id.to_string(),
                        name: name.to_string(),
                        status: status.to_string(),
                    });
                }
            }
        }
        Ok(zones)
    }

    /// Creates a tunnel and retrieves its runner token
    pub async fn create_tunnel(&self, name: &str) -> Result<(String, String)> {
        let mut secret_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_bytes);
        let tunnel_secret = base64::engine::general_purpose::STANDARD.encode(secret_bytes);

        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel",
            self.account_id
        );

        let payload = serde_json::json!({
            "name": name,
            "tunnel_secret": tunnel_secret,
            "config_src": "cloudflare"
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to create tunnel");
            return Err(anyhow!("{}", err_msg));
        }

        let tunnel_id = body["result"]["id"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing tunnel id in response"))?
            .to_string();

        let token_url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/token",
            self.account_id, tunnel_id
        );

        let token_resp = self
            .client
            .get(&token_url)
            .headers(self.headers())
            .send()
            .await?;

        let token_body: Value = token_resp.json().await?;
        let token = token_body["result"]
            .as_str()
            .ok_or_else(|| anyhow!("Failed to retrieve tunnel token"))?
            .to_string();

        Ok((tunnel_id, token))
    }

    /// Check if tunnel is active on Cloudflare edge
    pub async fn get_tunnel_status(&self, tunnel_id: &str) -> Result<(String, Option<String>)> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}",
            self.account_id, tunnel_id
        );

        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to fetch tunnel status");
            return Err(anyhow!("{}", err_msg));
        }

        let status = body["result"]["status"]
            .as_str()
            .unwrap_or("inactive")
            .to_string();

        let colo = body["result"]["connections"]
            .as_array()
            .and_then(|conns| conns.first())
            .and_then(|c| c["colo_name"].as_str())
            .map(|s| s.to_string());

        Ok((status, colo))
    }

    /// Configure ingress rules for the tunnel
    pub async fn update_tunnel_ingress(
        &self,
        tunnel_id: &str,
        routes: &[IngressRoute],
    ) -> Result<()> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/configurations",
            self.account_id, tunnel_id
        );

        let mut ingress_rules: Vec<Value> = Vec::new();
        for r in routes {
            ingress_rules.push(serde_json::json!({
                "hostname": r.hostname,
                "service": r.service,
            }));
        }

        ingress_rules.push(serde_json::json!({
            "service": "http_status:404"
        }));

        let payload = serde_json::json!({
            "config": {
                "ingress": ingress_rules
            }
        });

        let resp = self
            .client
            .put(&url)
            .headers(self.headers())
            .json(&payload)
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to update tunnel ingress configuration");
            return Err(anyhow!("{}", err_msg));
        }

        Ok(())
    }

    /// Upsert DNS CNAME record pointing hostname -> <tunnel_id>.cfargotunnel.com
    pub async fn upsert_dns_cname(
        &self,
        zone_id: &str,
        hostname: &str,
        tunnel_id: &str,
    ) -> Result<String> {
        let target = format!("{}.cfargotunnel.com", tunnel_id);

        let check_url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records?name={}",
            zone_id, hostname
        );

        let check_resp = self
            .client
            .get(&check_url)
            .headers(self.headers())
            .send()
            .await?;

        let check_body: Value = check_resp.json().await?;
        let existing_id = check_body["result"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["id"].as_str().map(|s| s.to_string()));

        let payload = serde_json::json!({
            "type": "CNAME",
            "name": hostname,
            "content": target,
            "proxied": true,
            "ttl": 1
        });

        if let Some(record_id) = existing_id {
            let update_url = format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
                zone_id, record_id
            );
            let resp = self
                .client
                .put(&update_url)
                .headers(self.headers())
                .json(&payload)
                .send()
                .await?;

            let body: Value = resp.json().await?;
            if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                let err_msg = body["errors"][0]["message"]
                    .as_str()
                    .unwrap_or("Failed to update DNS CNAME");
                return Err(anyhow!("{}", err_msg));
            }
            Ok(record_id)
        } else {
            let create_url = format!(
                "https://api.cloudflare.com/client/v4/zones/{}/dns_records",
                zone_id
            );
            let resp = self
                .client
                .post(&create_url)
                .headers(self.headers())
                .json(&payload)
                .send()
                .await?;

            let body: Value = resp.json().await?;
            if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                let err_msg = body["errors"][0]["message"]
                    .as_str()
                    .unwrap_or("Failed to create DNS CNAME");
                return Err(anyhow!("{}", err_msg));
            }

            let new_id = body["result"]["id"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing created DNS record ID"))?
                .to_string();

            Ok(new_id)
        }
    }

    /// Delete DNS record
    pub async fn delete_dns_record(&self, zone_id: &str, record_id: &str) -> Result<()> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/dns_records/{}",
            zone_id, record_id
        );
        let resp = self
            .client
            .delete(&url)
            .headers(self.headers())
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to delete DNS record");
            return Err(anyhow!("{}", err_msg));
        }
        Ok(())
    }

    /// Delete Cloudflare Tunnel
    pub async fn delete_tunnel(&self, tunnel_id: &str) -> Result<()> {
        let connections_url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}/connections",
            self.account_id, tunnel_id
        );
        let _ = self
            .client
            .delete(&connections_url)
            .headers(self.headers())
            .send()
            .await;

        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/cfd_tunnel/{}",
            self.account_id, tunnel_id
        );
        let resp = self
            .client
            .delete(&url)
            .headers(self.headers())
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to delete tunnel");
            return Err(anyhow!("{}", err_msg));
        }
        Ok(())
    }
}
