use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use uuid::Uuid;

use crate::cloudflare::CloudflareClient;
use crate::db::{
    delete_machine, delete_route, get_cloudflare_settings, get_machine, get_setting,
    insert_machine, insert_route, list_machines, list_routes_for_machine, log_event,
    update_machine_status, update_route_dns, DbPool,
};
use crate::models::{CreateMachineRequest, CreateRouteRequest, IngressRoute, Machine};

pub async fn list_machines_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let cf_settings = get_cloudflare_settings(&pool).unwrap_or(None);
    match list_machines(&pool) {
        Ok(machines) => {
            let mut list_with_counts = Vec::new();
            for mut m in machines {
                // If not marked online, check if Cloudflare edge has active connections
                if m.status != "online" {
                    if let (Some(ref tunnel_id), Some(ref cf)) = (&m.tunnel_id, &cf_settings) {
                        let client = CloudflareClient::new(cf.api_token.clone(), cf.account_id.clone());
                        if let Ok((cf_status, colo)) = client.get_tunnel_status(tunnel_id).await {
                            if cf_status == "healthy" {
                                let _ = update_machine_status(
                                    &pool,
                                    &m.id,
                                    "online",
                                    colo.as_deref().map(|c| format!("CF Edge ({})", c)).as_deref(),
                                    Some("Linux / Systemd"),
                                    Some("Ubuntu/Debian"),
                                );
                                m.status = "online".to_string();
                            }
                        }
                    }
                }

                let routes = list_routes_for_machine(&pool, &m.id).unwrap_or_default();
                list_with_counts.push(json!({
                    "id": m.id,
                    "name": m.name,
                    "setup_token": m.setup_token,
                    "tunnel_id": m.tunnel_id,
                    "tunnel_name": m.tunnel_name,
                    "tunnel_token": m.tunnel_token,
                    "zone_id": m.zone_id,
                    "zone_name": m.zone_name,
                    "status": m.status,
                    "remote_ip": m.remote_ip,
                    "remote_hostname": m.remote_hostname,
                    "os_info": m.os_info,
                    "created_at": m.created_at,
                    "last_seen": m.last_seen,
                    "routes_count": routes.len(),
                }));
            }
            Json(json!({ "success": true, "machines": list_with_counts }))
        }
        Err(e) => Json(json!({ "success": false, "error": e.to_string() })),
    }
}

pub async fn get_machine_handler(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let machine = get_machine(&pool, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Machine not found" }))))?;

    let mut machine = machine;

    // Check Cloudflare edge status to auto-sync "online" state
    if let (Some(ref tunnel_id), Some(cf_settings)) = (&machine.tunnel_id, get_cloudflare_settings(&pool).unwrap_or(None)) {
        let client = CloudflareClient::new(cf_settings.api_token, cf_settings.account_id);
        if let Ok((cf_status, colo)) = client.get_tunnel_status(tunnel_id).await {
            if cf_status == "healthy" {
                let _ = update_machine_status(
                    &pool,
                    &id,
                    "online",
                    colo.as_deref().map(|c| format!("Cloudflare Edge ({})", c)).as_deref(),
                    Some("Linux / Systemd"),
                    Some("Ubuntu/Debian"),
                );
                machine.status = "online".to_string();
                if let Some(c) = colo {
                    machine.remote_ip = Some(format!("Edge: {}", c));
                }
            }
        }
    }

    let routes = list_routes_for_machine(&pool, &id).unwrap_or_default();
    let detected_ip = crate::detect_local_ip();
    let server_url = get_setting(&pool, "server_url")
        .unwrap_or(None)
        .filter(|s| !s.trim().is_empty() && !s.contains("localhost") && !s.contains("127.0.0.1"))
        .unwrap_or_else(|| format!("http://{}:8080", detected_ip));

    let curl_command = format!(
        "curl -fsSL {}/setup/{}.sh | sudo bash",
        server_url.trim_end_matches('/'),
        machine.setup_token
    );

    let token_str = machine.tunnel_token.as_deref().unwrap_or("");
    let direct_command = format!(
        "curl -L --output /tmp/cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb && sudo dpkg -i /tmp/cloudflared.deb && sudo cloudflared service install {}",
        token_str
    );

    Ok(Json(json!({
        "success": true,
        "machine": machine,
        "routes": routes,
        "curl_command": curl_command,
        "direct_command": direct_command,
        "tunnel_token": token_str,
    })))
}

pub async fn create_machine_handler(
    State(pool): State<DbPool>,
    Json(payload): Json<CreateMachineRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cf_settings = get_cloudflare_settings(&pool)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Cloudflare API token and Account ID must be configured first in Settings" })),
            )
        })?;

    let client = CloudflareClient::new(cf_settings.api_token, cf_settings.account_id);

    // Sanitize tunnel name (letters, numbers, underscores, dashes)
    let sanitized_name = payload
        .name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>();
    let tunnel_name = format!("{}-{}", sanitized_name, &Uuid::new_v4().to_string()[..6]);

    // Create Cloudflare Tunnel
    let (tunnel_id, tunnel_token) = client
        .create_tunnel(&tunnel_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Cloudflare API error: {}", e) })),
            )
        })?;

    let machine_id = Uuid::new_v4().to_string();
    let setup_token = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let machine = Machine {
        id: machine_id.clone(),
        name: payload.name.clone(),
        setup_token: setup_token.clone(),
        tunnel_id: Some(tunnel_id.clone()),
        tunnel_name: Some(tunnel_name),
        tunnel_token: Some(tunnel_token),
        zone_id: payload.zone_id,
        zone_name: payload.zone_name,
        status: "pending".to_string(),
        remote_ip: None,
        remote_hostname: None,
        os_info: None,
        created_at: now,
        last_seen: None,
    };

    insert_machine(&pool, &machine).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Database error: {}", e) })),
        )
    })?;

    log_event(
        &pool,
        "INFO",
        &format!("Created new machine entry '{}' (Tunnel ID: {})", payload.name, tunnel_id),
    );

    let detected_ip = crate::detect_local_ip();
    let server_url = payload
        .server_url
        .or_else(|| get_setting(&pool, "server_url").unwrap_or(None))
        .filter(|s| !s.trim().is_empty() && !s.contains("localhost") && !s.contains("127.0.0.1"))
        .unwrap_or_else(|| format!("http://{}:8080", detected_ip));

    let curl_command = format!(
        "curl -fsSL {}/setup/{}.sh | sudo bash",
        server_url.trim_end_matches('/'),
        setup_token
    );

    let token_str = machine.tunnel_token.as_deref().unwrap_or("");
    let direct_command = format!(
        "curl -L --output /tmp/cloudflared.deb https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb && sudo dpkg -i /tmp/cloudflared.deb && sudo cloudflared service install {}",
        token_str
    );

    Ok(Json(json!({
        "success": true,
        "machine": machine,
        "curl_command": curl_command,
        "direct_command": direct_command,
        "tunnel_token": token_str,
    })))
}

pub async fn delete_machine_handler(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let machine = get_machine(&pool, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Machine not found" }))))?;

    // Attempt to delete cloudflare resources if configured
    if let (Some(cf_settings), Some(tunnel_id)) = (
        get_cloudflare_settings(&pool).unwrap_or(None),
        machine.tunnel_id.as_ref(),
    ) {
        let client = CloudflareClient::new(cf_settings.api_token, cf_settings.account_id);
        
        // Delete DNS records for routes
        if let (Ok(routes), Some(zone_id)) = (list_routes_for_machine(&pool, &id), machine.zone_id.as_ref()) {
            for r in routes {
                if let Some(record_id) = r.dns_record_id {
                    let _ = client.delete_dns_record(zone_id, &record_id).await;
                }
            }
        }

        // Delete Tunnel
        let _ = client.delete_tunnel(tunnel_id).await;
    }

    delete_machine(&pool, &id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
    })?;

    log_event(&pool, "INFO", &format!("Deleted machine '{}' ({})", machine.name, id));

    Ok(Json(json!({ "success": true, "message": "Machine and cloudflare tunnel deleted" })))
}

pub async fn add_route_handler(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
    Json(payload): Json<CreateRouteRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let machine = get_machine(&pool, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Machine not found" }))))?;

    let zone_name = machine.zone_name.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Machine does not have an assigned Cloudflare Domain/Zone" })),
        )
    })?;

    let zone_id = machine.zone_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Machine missing zone_id" })),
        )
    })?;

    let tunnel_id = machine.tunnel_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Machine missing tunnel_id" })),
        )
    })?;

    let cf_settings = get_cloudflare_settings(&pool)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Cloudflare settings not found" })),
            )
        })?;

    // Format hostname: if subdomain is "@", use root zone_name, otherwise sub.zone_name
    let sub = payload.subdomain.trim().to_lowercase();
    let hostname = if sub == "@" || sub.is_empty() {
        zone_name.clone()
    } else {
        format!("{}.{}", sub, zone_name)
    };

    let route_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let new_route = IngressRoute {
        id: route_id.clone(),
        machine_id: id.clone(),
        subdomain: sub,
        hostname: hostname.clone(),
        service: payload.service.trim().to_string(),
        dns_record_id: None,
        status: "pending".to_string(),
        created_at: now,
    };

    insert_route(&pool, &new_route).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("DB error: {}", e) })),
        )
    })?;

    // Now synchronize with Cloudflare:
    let client = CloudflareClient::new(cf_settings.api_token, cf_settings.account_id);

    // 1. Upsert DNS CNAME
    let dns_result = client.upsert_dns_cname(&zone_id, &hostname, &tunnel_id).await;

    let mut dns_record_id = None;
    let mut status = "synced";

    match dns_result {
        Ok(rec_id) => {
            dns_record_id = Some(rec_id);
        }
        Err(e) => {
            status = "dns_failed";
            log_event(&pool, "ERROR", &format!("Failed to create DNS CNAME: {}", e));
        }
    }

    // 2. Fetch all current routes for this machine and push to Cloudflare Ingress config
    let all_routes = list_routes_for_machine(&pool, &id).unwrap_or_default();
    if let Err(e) = client.update_tunnel_ingress(&tunnel_id, &all_routes).await {
        status = "config_failed";
        log_event(&pool, "ERROR", &format!("Failed to update tunnel ingress: {}", e));
    }

    let _ = update_route_dns(&pool, &route_id, dns_record_id.as_deref(), status);

    log_event(
        &pool,
        "INFO",
        &format!("Added route {} -> {} on machine {}", hostname, payload.service, machine.name),
    );

    Ok(Json(json!({
        "success": true,
        "route_id": route_id,
        "hostname": hostname,
        "status": status,
    })))
}

pub async fn delete_route_handler(
    State(pool): State<DbPool>,
    Path((id, route_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let machine = get_machine(&pool, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Machine not found" }))))?;

    let routes = list_routes_for_machine(&pool, &id).unwrap_or_default();
    let target_route = routes.into_iter().find(|r| r.id == route_id);

    if let (Some(route), Some(cf_settings), Some(zone_id), Some(tunnel_id)) = (
        target_route,
        get_cloudflare_settings(&pool).unwrap_or(None),
        machine.zone_id.as_ref(),
        machine.tunnel_id.as_ref(),
    ) {
        let client = CloudflareClient::new(cf_settings.api_token, cf_settings.account_id);

        // Delete DNS record if exists
        if let Some(ref rec_id) = route.dns_record_id {
            let _ = client.delete_dns_record(zone_id, rec_id).await;
        }

        // Delete from DB first
        let _ = delete_route(&pool, &route_id);

        // Re-push remaining routes to Cloudflare Ingress config
        let remaining_routes = list_routes_for_machine(&pool, &id).unwrap_or_default();
        let _ = client.update_tunnel_ingress(tunnel_id, &remaining_routes).await;
    } else {
        let _ = delete_route(&pool, &route_id);
    }

    Ok(Json(json!({ "success": true, "message": "Route deleted" })))
}

pub async fn sync_machine_handler(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let machine = get_machine(&pool, &id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "Machine not found" }))))?;

    let cf_settings = get_cloudflare_settings(&pool)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Cloudflare settings not found" })),
            )
        })?;

    let tunnel_id = machine.tunnel_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Machine missing tunnel_id" })),
        )
    })?;

    let zone_id = machine.zone_id.clone().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Machine missing zone_id" })),
        )
    })?;

    let client = CloudflareClient::new(cf_settings.api_token, cf_settings.account_id);
    let routes = list_routes_for_machine(&pool, &id).unwrap_or_default();

    // 1. Sync all DNS CNAME records
    for r in &routes {
        if let Ok(dns_id) = client.upsert_dns_cname(&zone_id, &r.hostname, &tunnel_id).await {
            let _ = update_route_dns(&pool, &r.id, Some(&dns_id), "synced");
        }
    }

    // 2. Sync Ingress configuration
    client
        .update_tunnel_ingress(&tunnel_id, &routes)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to update tunnel ingress: {}", e) })),
            )
        })?;

    log_event(&pool, "INFO", &format!("Manually re-synced routes for machine {}", machine.name));

    Ok(Json(json!({ "success": true, "message": "Successfully synchronized routes and DNS with Cloudflare" })))
}

