use rusqlite::{params, Connection, Result};
use std::sync::{Arc, Mutex};
use crate::models::{Machine, IngressRoute, CloudflareSettings};

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
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
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

    match (api_token, account_id) {
        (Some(token), Some(acc_id)) if !token.is_empty() && !acc_id.is_empty() => {
            Ok(Some(CloudflareSettings {
                api_token: token,
                account_id: acc_id,
                server_url,
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
    conn.execute("DELETE FROM machines WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn list_routes_for_machine(pool: &DbPool, machine_id: &str) -> Result<Vec<IngressRoute>> {
    let conn = pool.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, machine_id, subdomain, hostname, service, dns_record_id, status, created_at
         FROM routes WHERE machine_id = ?1 ORDER BY created_at ASC"
    )?;

    let rows = stmt.query_map(params![machine_id], |row| {
        Ok(IngressRoute {
            id: row.get(0)?,
            machine_id: row.get(1)?,
            subdomain: row.get(2)?,
            hostname: row.get(3)?,
            service: row.get(4)?,
            dns_record_id: row.get(5)?,
            status: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;

    let mut list = Vec::new();
    for route in rows {
        list.push(route?);
    }
    Ok(list)
}

pub fn insert_route(pool: &DbPool, route: &IngressRoute) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute(
        "INSERT INTO routes (id, machine_id, subdomain, hostname, service, dns_record_id, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            route.id,
            route.machine_id,
            route.subdomain,
            route.hostname,
            route.service,
            route.dns_record_id,
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

pub fn delete_route(pool: &DbPool, route_id: &str) -> Result<()> {
    let conn = pool.lock().unwrap();
    conn.execute("DELETE FROM routes WHERE id = ?1", params![route_id])?;
    Ok(())
}

pub fn log_event(pool: &DbPool, level: &str, message: &str) {
    if let Ok(conn) = pool.lock() {
        let now = chrono::Utc::now().to_rfc3339();
        let _ = conn.execute(
            "INSERT INTO audit_logs (timestamp, level, message) VALUES (?1, ?2, ?3)",
            params![now, level, message],
        );
    }
}

