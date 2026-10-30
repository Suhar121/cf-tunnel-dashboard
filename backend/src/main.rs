use axum::{
    http::HeaderMap,
    response::Html,
    routing::{delete, get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod agent_script;
mod cloudflare;
mod db;
mod models;
mod routes;

use db::{init_db, DbPool};

pub fn detect_local_ip() -> String {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(local_addr) = socket.local_addr() {
                let ip = local_addr.ip().to_string();
                if ip != "0.0.0.0" && ip != "127.0.0.1" {
                    return ip;
                }
            }
        }
    }
    "127.0.0.1".to_string()
}

async fn root_handler(headers: HeaderMap) -> Html<String> {
    let host = headers
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");

    let hostname = host.split(':').next().unwrap_or("localhost");
    let target_dashboard = format!("http://{}:3000", hostname);

    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <meta http-equiv="refresh" content="1; url={target_dashboard}">
  <title>Cloudflare Tunnel Central API</title>
  <style>
    body {{
      background: #09090b;
      color: #f4f4f5;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      display: flex;
      align-items: center;
      justify-content: center;
      height: 100vh;
      margin: 0;
    }}
    .card {{
      background: #18181b;
      border: 1px solid #27272a;
      border-radius: 16px;
      padding: 32px;
      text-align: center;
      max-width: 480px;
      box-shadow: 0 10px 25px rgba(0,0,0,0.5);
    }}
    h2 {{ color: #fb923c; margin-top: 0; font-size: 20px; }}
    p {{ color: #a1a1aa; font-size: 14px; line-height: 1.6; }}
    .btn {{
      display: inline-block;
      margin-top: 16px;
      padding: 10px 24px;
      background: #f97316;
      color: #fff;
      text-decoration: none;
      border-radius: 10px;
      font-weight: 600;
      font-size: 14px;
      transition: background 0.2s;
    }}
    .btn:hover {{ background: #ea580c; }}
  </style>
</head>
<body>
  <div class="card">
    <h2>Cloudflare Tunnel Backend API (:8080)</h2>
    <p>This port is the Rust backend REST API. The Web UI Dashboard is running on port <strong>3000</strong>.</p>
    <p>Redirecting you to the dashboard...</p>
    <a class="btn" href="{target_dashboard}">Open Web Dashboard (Port 3000)</a>
  </div>
</body>
</html>"#
    ))
}

async fn network_info_handler() -> Json<serde_json::Value> {
    let local_ip = detect_local_ip();
    Json(serde_json::json!({
        "success": true,
        "local_ip": local_ip,
        "backend_url": format!("http://{}:8080", local_ip),
        "frontend_url": format!("http://{}:3000", local_ip),
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Cloudflare Tunnel Dashboard Backend...");

    // Initialize SQLite DB
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "cf_dashboard.db".to_string());
    let pool = init_db(&db_path)?;
    tracing::info!("Database initialized at {}", db_path);

    let local_ip = detect_local_ip();
    tracing::info!("Detected Host LAN IP: {}", local_ip);

    // Spawn background worker to check machine offline status
    let monitor_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(45));
        loop {
            interval.tick().await;
            check_machine_heartbeats(&monitor_pool);
        }
    });

    let app = Router::new()
        // Root page handler (redirects browser from :8080 to :3000)
        .route("/", get(root_handler))
        // Health and system network detection
        .route("/api/health", get(|| async { "OK" }))
        .route("/api/system/network", get(network_info_handler))
        // Agent dynamic installer scripts and callbacks
        .route("/setup/:token", get(routes::agent::get_setup_script_handler))
        .route("/api/setup/:token", get(routes::agent::get_setup_script_handler))
        .route("/api/agent/register", post(routes::agent::register_agent_handler))
        .route("/api/agent/heartbeat", post(routes::agent::heartbeat_handler))
        .route("/api/agent/services", post(routes::agent::report_services_handler))
        .route("/api/logs", get(routes::agent::list_logs_handler))
        // Cloudflare settings & domain management
        .route("/api/cloudflare/config", get(routes::cloudflare::get_config_handler))
        .route("/api/cloudflare/config", post(routes::cloudflare::save_config_handler))
        .route("/api/cloudflare/test", post(routes::cloudflare::test_token_handler))
        .route("/api/cloudflare/zones", get(routes::cloudflare::list_zones_handler))
        // Machines & Tunnels
        .route("/api/machines", get(routes::machines::list_machines_handler))
        .route("/api/machines", post(routes::machines::create_machine_handler))
        .route("/api/machines/:id", get(routes::machines::get_machine_handler))
        .route("/api/machines/:id", delete(routes::machines::delete_machine_handler))
        .route("/api/machines/:id/routes", post(routes::machines::add_route_handler))
        .route("/api/machines/:id/routes/:route_id", delete(routes::machines::delete_route_handler))
        .route("/api/machines/:id/routes/:route_id/toggle-protect", post(routes::machines::toggle_route_protect_handler))
        .route("/api/machines/:id/discovered", get(routes::machines::list_discovered_services_handler))
        .route("/api/machines/:id/expose-discovered", post(routes::machines::expose_discovered_handler))
        .route("/api/machines/:id/sync", post(routes::machines::sync_machine_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Backend server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn check_machine_heartbeats(pool: &DbPool) {
    if let Ok(conn) = pool.lock() {
        let threshold = chrono::Utc::now() - chrono::Duration::minutes(3);
        let threshold_str = threshold.to_rfc3339();

        let _ = conn.execute(
            "UPDATE machines SET status = 'offline' 
             WHERE status = 'online' AND last_seen IS NOT NULL AND last_seen < ?1",
            rusqlite::params![threshold_str],
        );
    }
}
