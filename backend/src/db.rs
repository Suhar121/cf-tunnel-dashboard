use rusqlite::{params, Connection, Result};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use crate::models::{CloudflareSettings, DiscoveredService, DiscoveredServiceItem, IngressRoute, Machine};

pub type DbPool = Arc<Mutex<Connection>>;

pub fn init_db(path: &str) -> Result<DbPool> {
    let conn = Connection::open(path)?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS machines (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            setup_token TEXT NOT NULL UNIQUE,
            tunnel_id TEXT,
            tunnel_name TEXT,
            tunnel_token TEXT,
            zone_id TEXT,
            zone_name TEXT,
            status TEXT NOT NULL,
            remote_ip TEXT,
            remote_hostname TEXT,
            os_info TEXT,
            created_at TEXT NOT NULL,
            last_seen TEXT
        );

        CREATE TABLE IF NOT EXISTS routes (
            id TEXT PRIMARY KEY,
            machine_id TEXT NOT NULL,
            subdomain TEXT NOT NULL,
            hostname TEXT NOT NULL,
            service TEXT NOT NULL,
            dns_record_id TEXT,
            access_app_id TEXT,
            is_protected INTEGER DEFAULT 0,
            allowed_emails TEXT,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(machine_id) REFERENCES machines(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS discovered_services (
            id TEXT PRIMARY KEY,
            machine_id TEXT NOT NULL,
            name TEXT NOT NULL,
            port INTEGER NOT NULL,
            protocol TEXT NOT NULL,
            source TEXT NOT NULL,
            suggested_subdomain TEXT NOT NULL,
            is_exposed INTEGER DEFAULT 0,
            last_seen TEXT NOT NULL,
            UNIQUE(machine_id, port, protocol),
            FOREIGN KEY(machine_id) REFERENCES machines(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS audit_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL,
            message TEXT NOT NULL
        );
        "
    )?;

    // Safe schema migrations for existing routes table
    {
        let mut stmt = conn.prepare("PRAGMA table_info(routes)")?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get(1))?
            .filter_map(|r| r.ok())
            .collect();

        if !cols.contains(&"access_app_id".to_string()) {
            let _ = conn.execute("ALTER TABLE routes ADD COLUMN access_app_id TEXT", []);
        }
        if !cols.contains(&"is_protected".to_string()) {
            let _ = conn.execute("ALTER TABLE routes ADD COLUMN is_protected INTEGER DEFAULT 0", []);
        }
        if !cols.contains(&"allowed_emails".to_string()) {
            let _ = conn.execute("ALTER TABLE routes ADD COLUMN allowed_emails TEXT", []);
        }
    }

    Ok(Arc::new(Mutex::new(conn)))
}

pub fn get_setting(pool: &DbPool, key: &str) -> Result<Option<String>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn set_setting(pool: &DbPool, key: &str, value: &str) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_cloudflare_settings(pool: &DbPool) -> Result<Option<CloudflareSettings>> {
    let api_token = get_setting(pool, "cf_api_token")?;
    let account_id = get_setting(pool, "cf_account_id")?;
    let server_url = get_setting(pool, "server_url")?;
    let admin_email = get_setting(pool, "cf_admin_email")?;

    match (api_token, account_id) {
        (Some(token), Some(acc_id)) if !token.is_empty() && !acc_id.is_empty() => {
            Ok(Some(CloudflareSettings {
                api_token: token,
                account_id: acc_id,
                server_url,
                admin_email,
            }))
        }
        _ => Ok(None),
    }
}

pub fn list_machines(pool: &DbPool) -> Result<Vec<Machine>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, setup_token, tunnel_id, tunnel_name, tunnel_token, zone_id, zone_name, status, remote_ip, remote_hostname, os_info, created_at, last_seen
         FROM machines ORDER BY created_at DESC"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(Machine {
            id: row.get(0)?,
            name: row.get(1)?,
            setup_token: row.get(2)?,
            tunnel_id: row.get(3)?,
            tunnel_name: row.get(4)?,
            tunnel_token: row.get(5)?,
            zone_id: row.get(6)?,
            zone_name: row.get(7)?,
            status: row.get(8)?,
            remote_ip: row.get(9)?,
            remote_hostname: row.get(10)?,
            os_info: row.get(11)?,
            created_at: row.get(12)?,
            last_seen: row.get(13)?,
        })
    })?;

    let mut list = Vec::new();
    for machine in rows {
        list.push(machine?);
    }
    Ok(list)
}

pub fn get_machine(pool: &DbPool, id: &str) -> Result<Option<Machine>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, setup_token, tunnel_id, tunnel_name, tunnel_token, zone_id, zone_name, status, remote_ip, remote_hostname, os_info, created_at, last_seen
         FROM machines WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Machine {
            id: row.get(0)?,
            name: row.get(1)?,
            setup_token: row.get(2)?,
            tunnel_id: row.get(3)?,
            tunnel_name: row.get(4)?,
            tunnel_token: row.get(5)?,
            zone_id: row.get(6)?,
            zone_name: row.get(7)?,
            status: row.get(8)?,
            remote_ip: row.get(9)?,
            remote_hostname: row.get(10)?,
            os_info: row.get(11)?,
            created_at: row.get(12)?,
            last_seen: row.get(13)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_machine_by_token(pool: &DbPool, token: &str) -> Result<Option<Machine>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, setup_token, tunnel_id, tunnel_name, tunnel_token, zone_id, zone_name, status, remote_ip, remote_hostname, os_info, created_at, last_seen
         FROM machines WHERE setup_token = ?1"
    )?;

    let mut rows = stmt.query(params![token])?;
    if let Some(row) = rows.next()? {
        Ok(Some(Machine {
            id: row.get(0)?,
            name: row.get(1)?,
            setup_token: row.get(2)?,
            tunnel_id: row.get(3)?,
            tunnel_name: row.get(4)?,
            tunnel_token: row.get(5)?,
            zone_id: row.get(6)?,
            zone_name: row.get(7)?,
            status: row.get(8)?,
            remote_ip: row.get(9)?,
            remote_hostname: row.get(10)?,
            os_info: row.get(11)?,
            created_at: row.get(12)?,
            last_seen: row.get(13)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn insert_machine(pool: &DbPool, machine: &Machine) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "INSERT INTO machines (id, name, setup_token, tunnel_id, tunnel_name, tunnel_token, zone_id, zone_name, status, remote_ip, remote_hostname, os_info, created_at, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            machine.id,
            machine.name,
            machine.setup_token,
            machine.tunnel_id,
            machine.tunnel_name,
            machine.tunnel_token,
            machine.zone_id,
            machine.zone_name,
            machine.status,
            machine.remote_ip,
            machine.remote_hostname,
            machine.os_info,
            machine.created_at,
            machine.last_seen,
        ],
    )?;
    Ok(())
}

pub fn update_machine_token_and_tunnel(
    pool: &DbPool,
    id: &str,
    tunnel_id: &str,
    tunnel_name: &str,
    tunnel_token: &str,
) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "UPDATE machines SET tunnel_id = ?1, tunnel_name = ?2, tunnel_token = ?3 WHERE id = ?4",
        params![tunnel_id, tunnel_name, tunnel_token, id],
    )?;
    Ok(())
}

pub fn update_machine_status(
    pool: &DbPool,
    id: &str,
    status: &str,
    ip: Option<&str>,
    hostname: Option<&str>,
    os: Option<&str>,
) -> Result<()> {
    let conn = pool.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE machines SET status = ?1, remote_ip = COALESCE(?2, remote_ip), remote_hostname = COALESCE(?3, remote_hostname), os_info = COALESCE(?4, os_info), last_seen = ?5 WHERE id = ?6",
        params![status, ip, hostname, os, now, id],
    )?;
    Ok(())
}

pub fn delete_machine(pool: &DbPool, id: &str) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute("DELETE FROM routes WHERE machine_id = ?1", params![id])?;
    conn.execute("DELETE FROM discovered_services WHERE machine_id = ?1", params![id])?;
    conn.execute("DELETE FROM machines WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn list_routes_for_machine(pool: &DbPool, machine_id: &str) -> Result<Vec<IngressRoute>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, machine_id, subdomain, hostname, service, dns_record_id, access_app_id, is_protected, allowed_emails, status, created_at
         FROM routes WHERE machine_id = ?1 ORDER BY created_at ASC"
    )?;

    let rows = stmt.query_map(params![machine_id], |row| {
        let is_prot_int: i64 = row.get(7).unwrap_or(0);
        Ok(IngressRoute {
            id: row.get(0)?,
            machine_id: row.get(1)?,
            subdomain: row.get(2)?,
            hostname: row.get(3)?,
            service: row.get(4)?,
            dns_record_id: row.get(5)?,
            access_app_id: row.get(6)?,
            is_protected: is_prot_int == 1,
            allowed_emails: row.get(8)?,
            status: row.get(9)?,
            created_at: row.get(10)?,
        })
    })?;

    let mut list = Vec::new();
    for route in rows {
        list.push(route?);
    }
    Ok(list)
}

pub fn get_route(pool: &DbPool, route_id: &str) -> Result<Option<IngressRoute>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, machine_id, subdomain, hostname, service, dns_record_id, access_app_id, is_protected, allowed_emails, status, created_at
         FROM routes WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![route_id])?;
    if let Some(row) = rows.next()? {
        let is_prot_int: i64 = row.get(7).unwrap_or(0);
        Ok(Some(IngressRoute {
            id: row.get(0)?,
            machine_id: row.get(1)?,
            subdomain: row.get(2)?,
            hostname: row.get(3)?,
            service: row.get(4)?,
            dns_record_id: row.get(5)?,
            access_app_id: row.get(6)?,
            is_protected: is_prot_int == 1,
            allowed_emails: row.get(8)?,
            status: row.get(9)?,
            created_at: row.get(10)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn insert_route(pool: &DbPool, route: &IngressRoute) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "INSERT INTO routes (id, machine_id, subdomain, hostname, service, dns_record_id, access_app_id, is_protected, allowed_emails, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            route.id,
            route.machine_id,
            route.subdomain,
            route.hostname,
            route.service,
            route.dns_record_id,
            route.access_app_id,
            if route.is_protected { 1 } else { 0 },
            route.allowed_emails,
            route.status,
            route.created_at,
        ],
    )?;
    Ok(())
}

pub fn update_route_dns(pool: &DbPool, route_id: &str, dns_record_id: Option<&str>, status: &str) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "UPDATE routes SET dns_record_id = ?1, status = ?2 WHERE id = ?3",
        params![dns_record_id, status, route_id],
    )?;
    Ok(())
}

pub fn update_route_access(
    pool: &DbPool,
    route_id: &str,
    access_app_id: Option<&str>,
    is_protected: bool,
    allowed_emails: Option<&str>,
) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "UPDATE routes SET access_app_id = ?1, is_protected = ?2, allowed_emails = ?3 WHERE id = ?4",
        params![
            access_app_id,
            if is_protected { 1 } else { 0 },
            allowed_emails,
            route_id
        ],
    )?;
    Ok(())
}

pub fn delete_route(pool: &DbPool, route_id: &str) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute("DELETE FROM routes WHERE id = ?1", params![route_id])?;
    Ok(())
}

// ---------------- Discovered Services ----------------

pub fn list_discovered_services(pool: &DbPool, machine_id: &str) -> Result<Vec<DiscoveredService>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, machine_id, name, port, protocol, source, suggested_subdomain, is_exposed, last_seen
         FROM discovered_services WHERE machine_id = ?1 ORDER BY is_exposed ASC, port ASC"
    )?;

    let rows = stmt.query_map(params![machine_id], |row| {
        let is_exp_int: i64 = row.get(7).unwrap_or(0);
        Ok(DiscoveredService {
            id: row.get(0)?,
            machine_id: row.get(1)?,
            name: row.get(2)?,
            port: row.get::<_, u16>(3)?,
            protocol: row.get(4)?,
            source: row.get(5)?,
            suggested_subdomain: row.get(6)?,
            is_exposed: is_exp_int == 1,
            last_seen: row.get(8)?,
        })
    })?;

    let mut list = Vec::new();
    for item in rows {
        list.push(item?);
    }
    Ok(list)
}

pub fn get_discovered_service(pool: &DbPool, service_id: &str) -> Result<Option<DiscoveredService>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, machine_id, name, port, protocol, source, suggested_subdomain, is_exposed, last_seen
         FROM discovered_services WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![service_id])?;
    if let Some(row) = rows.next()? {
        let is_exp_int: i64 = row.get(7).unwrap_or(0);
        Ok(Some(DiscoveredService {
            id: row.get(0)?,
            machine_id: row.get(1)?,
            name: row.get(2)?,
            port: row.get::<_, u16>(3)?,
            protocol: row.get(4)?,
            source: row.get(5)?,
            suggested_subdomain: row.get(6)?,
            is_exposed: is_exp_int == 1,
            last_seen: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn upsert_discovered_services(
    pool: &DbPool,
    machine_id: &str,
    services: &[DiscoveredServiceItem],
) -> Result<()> {
    let conn = pool.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    // Check existing routes to determine if any port is already exposed
    let mut routes_stmt = conn.prepare("SELECT service FROM routes WHERE machine_id = ?1")?;
    let active_services: Vec<String> = routes_stmt
        .query_map(params![machine_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    for item in services {
        let protocol = item.protocol.as_deref().unwrap_or("http");
        let source = item.source.as_deref().unwrap_or("system");
        let default_sub = item
            .suggested_subdomain
            .as_deref()
            .unwrap_or_else(|| &item.name)
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();
        let sub = if default_sub.is_empty() {
            format!("port-{}", item.port)
        } else {
            default_sub
        };

        // Check if port is in active routes
        let port_pattern = format!(":{}", item.port);
        let is_exposed = active_services.iter().any(|s| s.contains(&port_pattern));

        let id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO discovered_services (id, machine_id, name, port, protocol, source, suggested_subdomain, is_exposed, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(machine_id, port, protocol) DO UPDATE SET
               name = excluded.name,
               source = excluded.source,
               suggested_subdomain = excluded.suggested_subdomain,
               is_exposed = excluded.is_exposed,
               last_seen = excluded.last_seen",
            params![
                id,
                machine_id,
                item.name,
                item.port,
                protocol,
                source,
                sub,
                if is_exposed { 1 } else { 0 },
                now
            ],
        )?;
    }

    Ok(())
}

pub fn mark_service_exposure(pool: &DbPool, machine_id: &str, port: u16, is_exposed: bool) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "UPDATE discovered_services SET is_exposed = ?1 WHERE machine_id = ?2 AND port = ?3",
        params![if is_exposed { 1 } else { 0 }, machine_id, port],
    )?;
    Ok(())
}

// ---------------- Audit Logs ----------------

pub fn log_event(pool: &DbPool, level: &str, message: &str) {
    if let Ok(conn) = pool.lock() {
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO audit_logs (timestamp, level, message) VALUES (?1, ?2, ?3)",
            params![now, level, message],
        );
    }
}
