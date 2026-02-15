use crate::models::auth::{DashboardData, LoginRequest, PasswordChangeRequest};
use crate::models::config::AppConfig;
use crate::services::auth::AuthService;
use crate::services::monitor::MonitorService;
use crate::services::threat_intel::ThreatIntelService;
use anyhow::Result;
use askama::Template;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

// Templates
#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate;

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    status: Vec<String>,
    anomalies: Vec<String>,
    has_anomaly: bool,
    graphs: String,
}

#[derive(Template)]
#[template(path = "downloads.html")]
struct DownloadsTemplate {
    files: Vec<DownloadFileInfo>,
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
    toggles: HashMap<String, bool>,
}

struct DownloadFileInfo {
    name: String,
    size: u64,
    modified: String,
}

// State
#[derive(Clone)]
pub struct AppState {
    pub monitor: Arc<RwLock<MonitorService>>,
    pub threat_intel: Arc<RwLock<ThreatIntelService>>,
    pub auth: Arc<AuthService>,
    pub config: AppConfig,
}

pub fn create_app(
    monitor: Arc<RwLock<MonitorService>>,
    threat_intel: Arc<RwLock<ThreatIntelService>>,
    config: AppConfig,
) -> Router {
    let auth = Arc::new(AuthService::new());
    
    let state = AppState {
        monitor,
        threat_intel,
        auth,
        config,
    };
    
    Router::new()
        .route("/", get(root))
        .route("/login", get(login_page).post(login_handler))
        .route("/logout", get(logout_handler))
        .route("/dashboard", get(dashboard_page))
        .route("/downloads", get(downloads_page))
        .route("/download/:filename", get(download_file))
        .route("/settings", get(settings_page).post(settings_handler))
        .route("/api/status", get(api_status))
        .route("/api/metrics", get(api_metrics))
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CookieManagerLayer::new())
        .with_state(state)
}

// Routes
async fn root() -> impl IntoResponse {
    Redirect::to("/login")
}

async fn login_page() -> impl IntoResponse {
    let template = LoginTemplate;
    Html(template.render().unwrap_or_else(|_| "Template error".to_string()))
}

async fn login_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(req): Form<LoginRequest>,
) -> impl IntoResponse {
    match state.auth.login(req).await {
        Ok(response) => {
            if let Some(token) = response.token {
                // Set session cookie
                let cookie = Cookie::build(("session", token))
                    .http_only(true)
                    .secure(true)
                    .path("/")
                    .max_age(tower_cookies::cookie::time::Duration::hours(1));
                cookies.add(cookie.into());
                
                Redirect::to("/dashboard").into_response()
            } else {
                Redirect::to("/login").into_response()
            }
        }
        Err(_) => Redirect::to("/login").into_response(),
    }
}

async fn logout_handler(
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    if let Some(token) = cookies.get("session") {
        let _ = state.auth.logout(token.value()).await;
    }
    
    cookies.remove(Cookie::new("session", ""));
    Redirect::to("/login")
}

async fn dashboard_page(
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    // Verify session
    if let Some(token) = cookies.get("session") {
        if state.auth.verify_token(token.value()).await.is_err() {
            return Redirect::to("/login").into_response();
        }
    } else {
        return Redirect::to("/login").into_response();
    }
    
    let monitor = state.monitor.read().await;
    let (anomalies, has_anomaly) = monitor.detect_anomalies();
    let status = monitor.status_report();
    let history = monitor.get_metrics_history();
    
    // Build graph data
    let graphs = json!({
        "cpu": history.iter().map(|m| m.cpu_percent).collect::<Vec<_>>(),
        "ram": history.iter().map(|m| m.ram_percent).collect::<Vec<_>>(),
        "disk": history.iter().map(|m| m.disk_percent).collect::<Vec<_>>(),
        "temp": history.iter().map(|m| m.temperature).collect::<Vec<_>>(),
        "ping": history.iter().map(|m| m.ping_ms).collect::<Vec<_>>(),
        "net": history.iter().map(|m| m.net_connections).collect::<Vec<_>>(),
        "fail": history.iter().map(|m| m.failed_logins).collect::<Vec<_>>(),
    });
    
    drop(monitor);
    
    let template = DashboardTemplate {
        status,
        anomalies,
        has_anomaly,
        graphs: graphs.to_string(),
    };
    
    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

async fn downloads_page(
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    // Verify session
    if let Some(token) = cookies.get("session") {
        if state.auth.verify_token(token.value()).await.is_err() {
            return Redirect::to("/login").into_response();
        }
    } else {
        return Redirect::to("/login").into_response();
    }
    
    let mut files = Vec::new();
    
    if let Ok(mut entries) = fs::read_dir("logs").await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".log") || name.ends_with(".txt") || name.ends_with(".json") {
                        let modified = metadata.modified()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        
                        files.push(DownloadFileInfo {
                            name,
                            size: metadata.len(),
                            modified: chrono::DateTime::from_timestamp(modified, 0)
                                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_default(),
                        });
                    }
                }
            }
        }
    }
    
    let template = DownloadsTemplate { files };
    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

async fn download_file(
    State(state): State<AppState>,
    cookies: Cookies,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    // Verify session
    if let Some(token) = cookies.get("session") {
        if state.auth.verify_token(token.value()).await.is_err() {
            return Redirect::to("/login").into_response();
        }
    } else {
        return Redirect::to("/login").into_response();
    }
    
    // Security: Only allow safe filenames
    let safe_filename = std::path::Path::new(&filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    
    if !safe_filename.ends_with(".log") 
        && !safe_filename.ends_with(".txt") 
        && !safe_filename.ends_with(".json") 
    {
        return (StatusCode::FORBIDDEN, "Invalid file type").into_response();
    }
    
    let file_path = format!("logs/{}", safe_filename);
    
    if let Ok(content) = fs::read(&file_path).await {
        Response::builder()
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", safe_filename))
            .body(Body::from(content))
            .unwrap()
            .into_response()
    } else {
        (StatusCode::NOT_FOUND, "File not found").into_response()
    }
}

async fn settings_page(
    State(state): State<AppState>,
    cookies: Cookies,
) -> impl IntoResponse {
    // Verify session
    if let Some(token) = cookies.get("session") {
        if state.auth.verify_token(token.value()).await.is_err() {
            return Redirect::to("/login").into_response();
        }
    } else {
        return Redirect::to("/login").into_response();
    }
    
    let toggles = state.config.display.stat_visibility.clone();
    
    let template = SettingsTemplate { toggles };
    Html(template.render().unwrap_or_else(|_| "Template error".to_string())).into_response()
}

async fn settings_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(req): Form<PasswordChangeRequest>,
) -> impl IntoResponse {
    // Verify session
    let username = if let Some(token) = cookies.get("session") {
        match state.auth.verify_token(token.value()).await {
            Ok(user) => user,
            Err(_) => return Redirect::to("/login").into_response(),
        }
    } else {
        return Redirect::to("/login").into_response();
    };
    
    match state.auth.change_password(&username, req).await {
        Ok(_) => Redirect::to("/dashboard").into_response(),
        Err(_) => Redirect::to("/settings").into_response(),
    }
}

async fn api_status(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let monitor = state.monitor.read().await;
    let status = monitor.status_report();
    let (anomalies, has_anomaly) = monitor.detect_anomalies();
    
    Json(DashboardData {
        status,
        anomalies,
        has_anomaly,
        graphs: json!({}),
    })
}

#[derive(Deserialize)]
struct MetricsQuery {
    limit: Option<usize>,
}

async fn api_metrics(
    State(state): State<AppState>,
    Query(params): Query<MetricsQuery>,
) -> impl IntoResponse {
    let monitor = state.monitor.read().await;
    let history = monitor.get_metrics_history();
    
    let limit = params.limit.unwrap_or(60);
    let metrics: Vec<_> = history.iter().rev().take(limit).collect();
    
    Json(json!({
        "metrics": metrics,
        "count": metrics.len(),
    }))
}
