use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: DateTime<Utc>,
    pub cpu_percent: f64,
    pub ram_percent: f64,
    pub disk_percent: f64,
    pub temperature: f64,
    pub ping_ms: f64,
    pub net_connections: usize,
    pub failed_logins: u32,
    pub host_status: HashMap<String, f64>, // host -> ping time in ms
}

impl SystemMetrics {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            cpu_percent: 0.0,
            ram_percent: 0.0,
            disk_percent: 0.0,
            temperature: 0.0,
            ping_ms: -1.0,
            net_connections: 0,
            failed_logins: 0,
            host_status: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub metric: String,
    pub value: f64,
    pub expected_mean: f64,
    pub expected_std: f64,
    pub severity: AnomalySeverity,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for AnomalySeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnomalySeverity::Info => write!(f, "Info"),
            AnomalySeverity::Warning => write!(f, "Warning"),
            AnomalySeverity::Critical => write!(f, "Critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineStats {
    pub mean: f64,
    pub std: f64,
    pub sample_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIntel {
    pub source: String,
    pub title: String,
    pub url: String,
    pub published: Option<DateTime<Utc>>,
}
