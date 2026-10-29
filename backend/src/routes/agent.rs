use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::agent_script::generate_bash_script;
use crate::db::{
    get_machine_by_token, get_setting, log_event, update_machine_status, DbPool,
};
use crate::models::{HeartbeatPayload, RegisterAgentPayload};

pub async fn get_setup_script_handler(
    State(pool): State<DbPool>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Response {
    // Strip possible ".sh" suffix from token path
    let clean_token = token.trim_end_matches(".sh");

    let machine = match get_machine_by_token(&pool, clean_token) {
        Ok(Some(m)) => m,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                [("content-type", "text/plain; charset=utf-8")],
                format!("#!/bin/bash\necho 'Error: Invalid or expired setup token: {}'\nexit 1\n", clean_token),
            )
                .into_response();
        }
    };

    let tunnel_token = match machine.tunnel_token.as_ref() {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                [("content-type", "text/plain; charset=utf-8")],
                "#!/bin/bash\necho 'Error: Machine tunnel has not been created in Cloudflare yet'\nexit 1\n",
            )
                .into_response();
        }
    };

    let tunnel_id = machine.tunnel_id.as_deref().unwrap_or("unknown");

    // Determine the dashboard server URL to embed in script:
    // If setting is localhost or empty, auto-substitute detected LAN IP so remote machine can reach it!
    let local_lan_ip = crate::detect_local_ip();
    let server_url = get_setting(&pool, "server_url")
        .unwrap_or(None)
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            if (s.contains("localhost") || s.contains("127.0.0.1")) && local_lan_ip != "127.0.0.1" {
                format!("http://{}:8080", local_lan_ip)
            } else {
                s
            }
        })
        .unwrap_or_else(|| {
            if let Some(host_val) = headers.get("host").and_then(|h| h.to_str().ok()) {
                if (host_val.starts_with("localhost") || host_val.starts_with("127.0.0.1")) && local_lan_ip != "127.0.0.1" {
                    format!("http://{}:8080", local_lan_ip)
                } else {
                    format!("http://{}", host_val)
                }
            } else {
                format!("http://{}:8080", local_lan_ip)
            }
        });

    let script = generate_bash_script(
        &server_url,
        &machine.name,
        &machine.setup_token,
        &tunnel_token,
        tunnel_id,
    );

    (
        StatusCode::OK,
        [
            ("content-type", "text/plain; charset=utf-8"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        script,
    )
        .into_response()
}

pub async fn register_agent_handler(
    State(pool): State<DbPool>,
    headers: HeaderMap,
    Json(payload): Json<RegisterAgentPayload>,
) -> impl IntoResponse {
    let machine = match get_machine_by_token(&pool, &payload.setup_token) {
        Ok(Some(m)) => m,
        _ => return (StatusCode::NOT_FOUND, Json(json!({ "error": "Invalid token" }))),
    };

    // Determine IP: payload.public_ip or client socket / X-Forwarded-For
    let remote_ip = payload.public_ip.or_else(|| {
        headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    });

    let _ = update_machine_status(
        &pool,
        &machine.id,
        "online",
        remote_ip.as_deref(),
        payload.hostname.as_deref(),
        payload.os_info.as_deref(),
    );

    log_event(
        &pool,
        "INFO",
        &format!(
            "Node '{}' registered online! Hostname: {:?}, OS: {:?}",
            machine.name, payload.hostname, payload.os_info
        ),
    );

    (StatusCode::OK, Json(json!({ "success": true, "message": "Machine registered online" })))
}

pub async fn heartbeat_handler(
    State(pool): State<DbPool>,
    Json(payload): Json<HeartbeatPayload>,
) -> impl IntoResponse {
    let machine = match get_machine_by_token(&pool, &payload.setup_token) {
        Ok(Some(m)) => m,
        _ => return (StatusCode::NOT_FOUND, Json(json!({ "error": "Invalid token" }))),
    };

    let _ = update_machine_status(&pool, &machine.id, "online", None, None, None);

    (StatusCode::OK, Json(json!({ "success": true })))
}

pub async fn list_logs_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let conn = pool.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, timestamp, level, message FROM audit_logs ORDER BY id DESC LIMIT 50")
        .unwrap();

    let logs = stmt
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "timestamp": row.get::<_, String>(1)?,
                "level": row.get::<_, String>(2)?,
                "message": row.get::<_, String>(3)?,
            }))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();

    Json(json!({ "success": true, "logs": logs }))
}
