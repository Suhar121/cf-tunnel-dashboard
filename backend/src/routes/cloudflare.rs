use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::cloudflare::CloudflareClient;
use crate::db::{get_cloudflare_settings, get_setting, set_setting, DbPool};

#[derive(Debug, Deserialize)]
pub struct SaveConfigRequest {
    pub api_token: String,
    pub account_id: String,
    pub server_url: Option<String>,
    pub admin_email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestTokenRequest {
    pub api_token: String,
}

pub async fn get_config_handler(State(pool): State<DbPool>) -> impl IntoResponse {
    let settings = get_cloudflare_settings(&pool).unwrap_or(None);
    let server_url = get_setting(&pool, "server_url").unwrap_or(None);
    let detected_ip = crate::detect_local_ip();

    let effective_url = match server_url {
        Some(ref u) if !u.trim().is_empty() && !u.contains("localhost") && !u.contains("127.0.0.1") => {
            u.clone()
        }
        _ => format!("http://{}:8080", detected_ip),
    };

    if let Some(s) = settings {
        let masked_token = if s.api_token.len() > 8 {
            format!("{}...{}", &s.api_token[..4], &s.api_token[s.api_token.len() - 4..])
        } else {
            "******".to_string()
        };

        Json(json!({
            "configured": true,
            "account_id": s.account_id,
            "masked_token": masked_token,
            "server_url": effective_url,
            "admin_email": s.admin_email,
            "detected_lan_ip": detected_ip,
        }))
    } else {
        Json(json!({
            "configured": false,
            "server_url": effective_url,
            "admin_email": null,
            "detected_lan_ip": detected_ip,
        }))
    }
}

pub async fn save_config_handler(
    State(pool): State<DbPool>,
    Json(payload): Json<SaveConfigRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let new_token = payload.api_token.trim();
    let existing_token = get_setting(&pool, "cf_api_token").unwrap_or(None);

    if !new_token.is_empty() {
        let valid = CloudflareClient::verify_token(new_token)
            .await
            .map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("Token verification failed: {}", e) })),
                )
            })?;

        if !valid {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid Cloudflare API token" })),
            ));
        }

        set_setting(&pool, "cf_api_token", new_token)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;
    } else if existing_token.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "API Token is required" })),
        ));
    }

    set_setting(&pool, "cf_account_id", payload.account_id.trim())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    if let Some(ref url) = payload.server_url {
        set_setting(&pool, "server_url", url.trim())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;
    }

    if let Some(ref email) = payload.admin_email {
        set_setting(&pool, "cf_admin_email", email.trim())
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;
    }

    Ok(Json(json!({
        "success": true,
        "message": "Cloudflare configuration saved successfully"
    })))
}

pub async fn test_token_handler(
    State(pool): State<DbPool>,
    Json(payload): Json<TestTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let token = if !payload.api_token.trim().is_empty() {
        payload.api_token.trim().to_string()
    } else {
        get_setting(&pool, "cf_api_token")
            .unwrap_or(None)
            .ok_or_else(|| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "No API token provided or saved in settings" })),
                )
            })?
    };

    let valid = CloudflareClient::verify_token(&token)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Failed to reach Cloudflare API: {}", e) })),
            )
        })?;

    if !valid {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Token is invalid or unauthorized" })),
        ));
    }

    let accounts = CloudflareClient::list_accounts(&token)
        .await
        .unwrap_or_default();

    Ok(Json(json!({
        "valid": true,
        "accounts": accounts
    })))
}

pub async fn list_zones_handler(
    State(pool): State<DbPool>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let settings = get_cloudflare_settings(&pool)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Cloudflare settings not configured yet" })),
            )
        })?;

    let client = CloudflareClient::new(settings.api_token, settings.account_id);
    let zones = client
        .list_zones()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    Ok(Json(json!({
        "success": true,
        "zones": zones
    })))
}
