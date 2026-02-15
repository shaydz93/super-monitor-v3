use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

mod handlers;
mod models;
mod services;
mod utils;

use handlers::create_app;
use models::config::AppConfig;
use services::monitor::MonitorService;
use services::threat_intel::ThreatIntelService;
use utils::logging::init_logging;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging()?;
    
    info!("Starting ShaydZ Super Monitor v2.0 (Rust)");
    
    // Load configuration
    let config = AppConfig::load().unwrap_or_default();
    info!("Configuration loaded successfully");
    
    // Initialize shared state
    let monitor = Arc::new(RwLock::new(MonitorService::new(config.monitoring.clone())));
    let threat_intel = Arc::new(RwLock::new(ThreatIntelService::new()));
    
    // Start background monitoring task
    let monitor_clone = Arc::clone(&monitor);
    tokio::spawn(async move {
        background_monitor_loop(monitor_clone, config.monitoring.update_interval).await;
    });
    
    // Start threat intelligence refresh task
    let threat_intel_clone = Arc::clone(&threat_intel);
    tokio::spawn(async move {
        threat_intel_refresh_loop(threat_intel_clone, 1800).await;
    });
    
    // Create and run the web server
    let app = create_app(monitor, threat_intel, config);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 5001));
    info!("Web server listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn background_monitor_loop(monitor: Arc<RwLock<MonitorService>>, interval_secs: u64) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
    
    loop {
        interval.tick().await;
        
        let mut monitor_guard = monitor.write().await;
        
        // Update metrics
        if let Err(e) = monitor_guard.update().await {
            warn!("Monitor update error: {}", e);
        }
        
        // Learn baseline
        monitor_guard.learn_baseline();
        
        // Check for anomalies
        let (anomalies, has_anomaly) = monitor_guard.detect_anomalies();
        
        if has_anomaly {
            info!("Anomalies detected: {:?}", anomalies);
            // Trigger actions
            if let Err(e) = monitor_guard.trigger_actions(&anomalies).await {
                warn!("Action trigger error: {}", e);
            }
        }
        
        // Save baseline periodically
        if let Err(e) = monitor_guard.save_baseline().await {
            warn!("Baseline save error: {}", e);
        }
        
        drop(monitor_guard);
    }
}

async fn threat_intel_refresh_loop(threat_intel: Arc<RwLock<ThreatIntelService>>, interval_secs: u64) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
    
    loop {
        interval.tick().await;
        
        let mut intel_guard = threat_intel.write().await;
        
        if let Err(e) = intel_guard.fetch_all().await {
            warn!("Threat intel fetch error: {}", e);
        } else {
            info!("Threat intelligence updated");
        }
        
        drop(intel_guard);
    }
}
