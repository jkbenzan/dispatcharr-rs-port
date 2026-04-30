use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::net::SocketAddr;
use std::sync::Arc;
use ipnet::IpNet;

use crate::AppState;
use crate::entities::core_settings;

pub async fn network_access_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();

    // Determine the category based on the path
    let category = if path.starts_with("/api/core/m3u/")
        || path.starts_with("/api/core/epg/")
        || path.starts_with("/m3u/")
        || path.starts_with("/epg/")
    {
        "M3U_EPG"
    } else if path.starts_with("/api/channels/stream/") || path.contains("/stream/") {
        "STREAMS"
    } else if path.starts_with("/api/xc/")
        || path.contains("player_api.php")
        || path.contains("xmltv.php")
    {
        "XC_API"
    } else {
        "UI"
    };

    // Check if network_access is configured
    let setting = core_settings::Entity::find()
        .filter(core_settings::Column::Key.eq("network_access"))
        .one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut allowed = true;

    if let Some(s) = setting {
        if let Some(ips_str) = s.value.get(category).and_then(|v| v.as_str()) {
            let ips_str = ips_str.trim();
            if !ips_str.is_empty() {
                allowed = false; // Block by default if rules exist
                let client_ip = addr.ip();

                for part in ips_str.split(',') {
                    let part = part.trim();
                    if part.is_empty() {
                        continue;
                    }

                    // check if CIDR or simple IP
                    if let Ok(net) = part.parse::<IpNet>() {
                        if net.contains(&client_ip) {
                            allowed = true;
                            break;
                        }
                    } else if let Ok(ip) = part.parse::<std::net::IpAddr>() {
                        if ip == client_ip {
                            allowed = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    if !allowed {
        tracing::warn!(
            "Blocked request from {} to {} (category: {}) due to network_access settings",
            addr.ip(),
            path,
            category
        );
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}
