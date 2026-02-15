use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub monitoring: MonitoringConfig,
    pub security: SecurityConfig,
    pub display: DisplayConfig,
    pub alerts: AlertConfig,
    pub logging: LoggingConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            monitoring: MonitoringConfig::default(),
            security: SecurityConfig::default(),
            display: DisplayConfig::default(),
            alerts: AlertConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Option<Self> {
        // Try to load from config.toml or config.json
        if let Ok(content) = std::fs::read_to_string("config.toml") {
            if let Ok(config) = toml::from_str(&content) {
                return Some(config);
            }
        }
        
        if let Ok(content) = std::fs::read_to_string("config.json") {
            if let Ok(config) = serde_json::from_str(&content) {
                return Some(config);
            }
        }
        
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub window_size: usize,
    pub update_interval: u64,
    pub anomaly_threshold: f64,
    pub monitored_hosts: Vec<String>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            window_size: 60,
            update_interval: 5,
            anomaly_threshold: 3.0,
            monitored_hosts: vec![
                "8.8.8.8".to_string(),
                "1.1.1.1".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub password_hash: String,
    pub session_timeout: u64,
    pub max_login_attempts: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            // Default hash for "admin" - should be changed in production
            password_hash: "$2b$12$kLxCe90oN9uXVqPkbSCoKuP.9z0gWgtjsGzPHVRE9e5V3xCiBJ4x2".to_string(),
            session_timeout: 3600,
            max_login_attempts: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub stat_visibility: HashMap<String, bool>,
    pub refresh_rate: u64,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        let mut visibility = HashMap::new();
        visibility.insert("cpu".to_string(), true);
        visibility.insert("ram".to_string(), true);
        visibility.insert("disk".to_string(), true);
        visibility.insert("temp".to_string(), true);
        visibility.insert("ping".to_string(), true);
        visibility.insert("net".to_string(), true);
        visibility.insert("fail".to_string(), true);
        
        Self {
            stat_visibility: visibility,
            refresh_rate: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub high_temp_threshold: f64,
    pub email_notifications: bool,
    pub webhook_url: Option<String>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            high_temp_threshold: 80.0,
            email_notifications: false,
            webhook_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub max_file_size: String,
    pub backup_count: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "INFO".to_string(),
            max_file_size: "10MB".to_string(),
            backup_count: 5,
        }
    }
}
