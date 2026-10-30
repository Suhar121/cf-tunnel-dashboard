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

    /// Discover configured identity providers from existing access applications
    pub async fn discover_allowed_idps(&self) -> Vec<String> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/access/apps",
            self.account_id
        );
        let mut idps = Vec::new();
        if let Ok(resp) = self.client.get(&url).headers(self.headers()).send().await {
            if let Ok(body) = resp.json::<Value>().await {
                if let Some(apps) = body["result"].as_array() {
                    for app in apps {
                        if let Some(app_idps) = app["allowed_idps"].as_array() {
                            for idp in app_idps {
                                if let Some(s) = idp.as_str() {
                                    if !idps.contains(&s.to_string()) {
                                        idps.push(s.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        idps
    }

    /// Find an existing Access application by domain
    pub async fn find_access_app_by_domain(&self, domain: &str) -> Result<Option<String>> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/access/apps",
            self.account_id
        );
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await?;

        let body: Value = resp.json().await?;
        if let Some(apps) = body["result"].as_array() {
            for app in apps {
                if let (Some(id), Some(app_domain)) = (app["id"].as_str(), app["domain"].as_str()) {
                    if app_domain.eq_ignore_ascii_case(domain) {
                        return Ok(Some(id.to_string()));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Create or configure a Zero Trust Cloudflare Access Application with Email PIN login policy
    pub async fn create_access_application(
        &self,
        name: &str,
        domain: &str,
        allowed_emails: &[String],
    ) -> Result<String> {
        let discovered_idps = self.discover_allowed_idps().await;

        // Check if an Access Application already exists for this domain
        let app_id = if let Ok(Some(existing_id)) = self.find_access_app_by_domain(domain).await {
            // Update existing app to ensure allowed_idps and settings are active
            let update_url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/access/apps/{}",
                self.account_id, existing_id
            );
            let mut update_payload = serde_json::json!({
                "name": name,
                "domain": domain,
                "type": "self_hosted",
            });
            if !discovered_idps.is_empty() {
                update_payload["allowed_idps"] = serde_json::json!(discovered_idps);
            }
            let _ = self
                .client
                .put(&update_url)
                .headers(self.headers())
                .json(&update_payload)
                .send()
                .await;

            existing_id
        } else {
            let url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/access/apps",
                self.account_id
            );

            let mut payload = serde_json::json!({
                "name": name,
                "domain": domain,
                "type": "self_hosted",
                "session_duration": "24h",
                "auto_redirect_to_identity": false
            });

            if !discovered_idps.is_empty() {
                payload["allowed_idps"] = serde_json::json!(discovered_idps);
            }

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
                    .unwrap_or("Failed to create Cloudflare Access application");
                return Err(anyhow!("{}", err_msg));
            }

            body["result"]["id"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing access application id"))?
                .to_string()
        };

        // Attach or update email authorization policy if emails provided
        if !allowed_emails.is_empty() {
            let policy_url = format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/access/apps/{}/policies",
                self.account_id, app_id
            );

            let include_rules: Vec<Value> = allowed_emails
                .iter()
                .filter(|e| !e.trim().is_empty())
                .map(|email| serde_json::json!({ "email": { "email": email.trim() } }))
                .collect();

            if !include_rules.is_empty() {
                let policy_payload = serde_json::json!({
                    "name": "Dashboard Email Allowlist",
                    "decision": "allow",
                    "include": include_rules
                });

                // Check if a policy already exists on this application
                let existing_policy_id = if let Ok(resp) = self.client.get(&policy_url).headers(self.headers()).send().await {
                    if let Ok(body) = resp.json::<Value>().await {
                        body["result"].as_array().and_then(|arr| arr.first()).and_then(|p| p["id"].as_str()).map(|s| s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(pol_id) = existing_policy_id {
                    let update_pol_url = format!("{}/{}", policy_url, pol_id);
                    let _ = self
                        .client
                        .put(&update_pol_url)
                        .headers(self.headers())
                        .json(&policy_payload)
                        .send()
                        .await;
                } else {
                    let _ = self
                        .client
                        .post(&policy_url)
                        .headers(self.headers())
                        .json(&policy_payload)
                        .send()
                        .await;
                }
            }
        }

        Ok(app_id)
    }

    /// Delete Cloudflare Access Application
    pub async fn delete_access_application(&self, app_id: &str) -> Result<()> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/access/apps/{}",
            self.account_id, app_id
        );

        let resp = self
            .client
            .delete(&url)
            .headers(self.headers())
            .send()
            .await?;

        // 404 or success are both acceptable
        if resp.status().as_u16() == 404 {
            return Ok(());
        }

        let body: Value = resp.json().await?;
        if !body.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            let err_msg = body["errors"][0]["message"]
                .as_str()
                .unwrap_or("Failed to delete Access application");
            return Err(anyhow!("{}", err_msg));
        }

        Ok(())
    }

    pub async fn debug_app_and_policies(&self, app_id: &str) -> Result<(Value, Value)> {
        let app_url = format!("https://api.cloudflare.com/client/v4/accounts/{}/access/apps/{}", self.account_id, app_id);
        let pol_url = format!("https://api.cloudflare.com/client/v4/accounts/{}/access/apps/{}/policies", self.account_id, app_id);
        let r1: Value = self.client.get(&app_url).headers(self.headers()).send().await?.json().await?;
        let r2: Value = self.client.get(&pol_url).headers(self.headers()).send().await?.json().await?;
        Ok((r1, r2))
    }

    pub async fn get_access_logs(&self) -> Result<Value> {
        let url = format!("https://api.cloudflare.com/client/v4/accounts/{}/access/logs/access_requests?limit=10", self.account_id);
        let r: Value = self.client.get(&url).headers(self.headers()).send().await?.json().await?;
        Ok(r)
    }
}
