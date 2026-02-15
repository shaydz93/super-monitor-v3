use crate::models::metrics::{Anomaly, AnomalySeverity, BaselineStats, SystemMetrics};
use crate::models::config::MonitoringConfig;
use anyhow::{anyhow, Result};
use chrono::Utc;
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use sysinfo::{Disks, Networks, System};
use tokio::fs;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{info, warn};

const BASELINE_FILE: &str = "data/baseline.json";

pub struct MonitorService {
    config: MonitoringConfig,
    system: System,
    metrics_history: VecDeque<SystemMetrics>,
    baselines: HashMap<String, BaselineStats>,
    feedback: HashMap<String, bool>,
    current_iocs: HashSet<String>,
    file_lock: Arc<Mutex<()>>,
}

impl MonitorService {
    pub fn new(config: MonitoringConfig) -> Self {
        let mut service = Self {
            config,
            system: System::new_all(),
            metrics_history: VecDeque::with_capacity(100),
            baselines: HashMap::new(),
            feedback: HashMap::new(),
            current_iocs: HashSet::new(),
            file_lock: Arc::new(Mutex::new(())),
        };
        
        // Load existing baseline if available
        let _ = service.load_baseline();
        
        service
    }
    
    pub async fn update(&mut self) -> Result<()> {
        self.system.refresh_all();
        
        let mut metrics = SystemMetrics::new();
        
        // CPU usage
        metrics.cpu_percent = self.system.global_cpu_info().cpu_usage() as f64;
        
        // RAM usage
        let total_memory = self.system.total_memory() as f64;
        let used_memory = self.system.used_memory() as f64;
        if total_memory > 0.0 {
            metrics.ram_percent = (used_memory / total_memory) * 100.0;
        }
        
        // Disk usage
        let disks = Disks::new_with_refreshed_list();
        let mut total_space = 0u64;
        let mut used_space = 0u64;
        for disk in &disks {
            total_space += disk.total_space();
            used_space += disk.total_space() - disk.available_space();
        }
        if total_space > 0 {
            metrics.disk_percent = (used_space as f64 / total_space as f64) * 100.0;
        }
        
        // Temperature
        metrics.temperature = self.get_temperature().await;
        
        // Network connections
        let networks = Networks::new_with_refreshed_list();
        metrics.net_connections = networks.len();
        
        // Ping gateway
        metrics.ping_ms = self.ping_gateway().await;
        
        // Failed logins
        metrics.failed_logins = self.failed_logins().await;
        
        // Host status
        for host in &self.config.monitored_hosts {
            let ping_time = self.ping_host(host).await;
            metrics.host_status.insert(host.clone(), ping_time);
        }
        
        // Add to history
        if self.metrics_history.len() >= self.config.window_size {
            self.metrics_history.pop_front();
        }
        self.metrics_history.push_back(metrics);
        
        Ok(())
    }
    
    async fn get_temperature(&self) -> f64 {
        // Try Raspberry Pi vcgencmd first
        if let Ok(output) = Command::new("vcgencmd")
            .args(["measure_temp"])
            .output()
            .await
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(temp_str) = stdout.split('=').nth(1) {
                let temp_clean = temp_str.replace("'C", "").trim().to_string();
                if let Ok(temp) = temp_clean.parse::<f64>() {
                    return temp;
                }
            }
        }
        
        // Try thermal zone files
        for i in 0..5 {
            let path = format!("/sys/class/thermal/thermal_zone{}/temp", i);
            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(temp_milli) = content.trim().parse::<f64>() {
                    return temp_milli / 1000.0;
                }
            }
        }
        
        // Simulate temperature based on CPU usage
        let cpu = self.system.global_cpu_info().cpu_usage() as f64;
        let base_temp = 35.0 + (cpu * 0.3);
        let variation = (chrono::Utc::now().timestamp() as f64 / 100.0).sin() * 5.0;
        (base_temp + variation).round()
    }
    
    async fn ping_gateway(&self) -> f64 {
        // Determine gateway
        let gateway = self.get_default_gateway().await;
        self.ping_host(&gateway).await
    }
    
    async fn get_default_gateway(&self) -> String {
        // Try to get default gateway from routing table
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("ip")
                .args(["route", "show", "default"])
                .output()
                .await
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Some(gw) = line.split_whitespace().nth(2) {
                        return gw.to_string();
                    }
                }
            }
        }
        
        // Fallback to common gateway addresses
        "192.168.1.1".to_string()
    }
    
    async fn ping_host(&self, host: &str) -> f64 {
        // Use system ping command
        let cmd = if cfg!(target_os = "windows") {
            vec!["ping", "-n", "1", "-w", "1000", host]
        } else {
            vec!["ping", "-c", "1", "-W", "1", host]
        };
        
        match Command::new(cmd[0])
            .args(&cmd[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
        {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse ping time from output
                if let Some(time_ms) = parse_ping_time(&stdout) {
                    return time_ms;
                }
            }
            _ => {}
        }
        
        // Fallback: try TCP connection
        self.tcp_ping(host).await
    }
    
    async fn tcp_ping(&self, host: &str) -> f64 {
        use tokio::net::TcpStream;
        use tokio::time::{timeout, Duration};
        
        let start = tokio::time::Instant::now();
        
        match timeout(Duration::from_secs(2), TcpStream::connect((host, 80))).await {
            Ok(Ok(_)) => {
                let elapsed = start.elapsed().as_millis() as f64;
                return elapsed;
            }
            _ => {
                // Try DNS resolution
                match dns_lookup::lookup_host(host) {
                    Ok(_) => 50.0, // Assume reasonable latency
                    Err(_) => -1.0,
                }
            }
        }
    }
    
    async fn failed_logins(&self) -> u32 {
        let log_files = vec![
            "/var/log/auth.log",
            "/var/log/secure",
            "/var/log/messages",
        ];
        
        let mut count = 0u32;
        
        for log_file in &log_files {
            if let Ok(content) = fs::read_to_string(log_file).await {
                let lines: Vec<&str> = content.lines().collect();
                let recent_lines = lines.iter().rev().take(500);
                
                for line in recent_lines {
                    if line.contains("Failed password") && !line.contains("invalid user") {
                        count += 1;
                    }
                }
            }
        }
        
        // Simulate occasional failed logins if no logs available
        if count == 0 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.1) {
                count = rng.gen_range(0..3);
            }
        }
        
        count
    }
    
    pub fn learn_baseline(&mut self) {
        if self.metrics_history.len() < 20 {
            return;
        }
        
        // Learn CPU baseline
        let cpu_values: Vec<f64> = self.metrics_history.iter().map(|m| m.cpu_percent).collect();
        if let Some(stats) = calculate_stats(&cpu_values) {
            self.baselines.insert("cpu".to_string(), stats);
        }
        
        // Learn RAM baseline
        let ram_values: Vec<f64> = self.metrics_history.iter().map(|m| m.ram_percent).collect();
        if let Some(stats) = calculate_stats(&ram_values) {
            self.baselines.insert("ram".to_string(), stats);
        }
        
        // Learn Disk baseline
        let disk_values: Vec<f64> = self.metrics_history.iter().map(|m| m.disk_percent).collect();
        if let Some(stats) = calculate_stats(&disk_values) {
            self.baselines.insert("disk".to_string(), stats);
        }
        
        // Learn Temperature baseline
        let temp_values: Vec<f64> = self.metrics_history.iter().map(|m| m.temperature).collect();
        if let Some(stats) = calculate_stats(&temp_values) {
            self.baselines.insert("temp".to_string(), stats);
        }
        
        // Learn Ping baseline
        let ping_values: Vec<f64> = self.metrics_history.iter().map(|m| m.ping_ms).collect();
        if let Some(stats) = calculate_stats(&ping_values) {
            self.baselines.insert("ping".to_string(), stats);
        }
        
        // Learn Net baseline
        let net_values: Vec<f64> = self.metrics_history.iter().map(|m| m.net_connections as f64).collect();
        if let Some(stats) = calculate_stats(&net_values) {
            self.baselines.insert("net".to_string(), stats);
        }
        
        // Learn Failed Logins baseline
        let fail_values: Vec<f64> = self.metrics_history.iter().map(|m| m.failed_logins as f64).collect();
        if let Some(stats) = calculate_stats(&fail_values) {
            self.baselines.insert("fail".to_string(), stats);
        }
        
        // Learn baselines for monitored hosts
        for host in &self.config.monitored_hosts {
            let values: Vec<f64> = self.metrics_history
                .iter()
                .filter_map(|m| m.host_status.get(host).copied())
                .collect();
            
            if let Some(stats) = calculate_stats(&values) {
                self.baselines.insert(host.clone(), stats);
            }
        }
    }
    
    pub fn detect_anomalies(&self) -> (Vec<String>, bool) {
        if self.metrics_history.len() < 20 {
            return (vec!["Learning...".to_string()], false);
        }
        
        let mut anomalies = Vec::new();
        let latest = self.metrics_history.back().unwrap();
        let threshold = self.config.anomaly_threshold;
        
        // Check system metrics
        let checks = vec![
            ("cpu", latest.cpu_percent, "CPU"),
            ("ram", latest.ram_percent, "RAM"),
            ("disk", latest.disk_percent, "Disk"),
            ("temp", latest.temperature, "Temp"),
            ("ping", latest.ping_ms, "Ping"),
            ("net", latest.net_connections as f64, "Connections"),
            ("fail", latest.failed_logins as f64, "Failed Login"),
        ];
        
        for (metric, value, label) in checks {
            if let Some(baseline) = self.baselines.get(metric) {
                let feedback_key = format!("{}-{:.0}", metric, value);
                if baseline.std > 0.0 
                    && (value - baseline.mean).abs() > threshold * baseline.std
                    && !self.feedback.get(&feedback_key).copied().unwrap_or(false) 
                {
                    anomalies.push(format!(
                        "Anomaly: {} {:.1} (Normal: {:.1}±{:.1})",
                        label, value, baseline.mean, baseline.std
                    ));
                }
            }
        }
        
        // Check host status
        for (host, &ping_time) in &latest.host_status {
            if ping_time < 0.0 {
                anomalies.push(format!("Device Down: {}", host));
            } else if let Some(baseline) = self.baselines.get(host) {
                if baseline.std > 0.0 && (ping_time - baseline.mean).abs() > threshold * baseline.std {
                    anomalies.push(format!(
                        "Anomaly: {} {:.1}ms (Normal: {:.1}±{:.1})",
                        host, ping_time, baseline.mean, baseline.std
                    ));
                }
            }
        }
        
        // Check for threat IPs
        for ip in &self.current_iocs {
            anomalies.push(format!("Threat IP: {}", ip));
        }
        
        let has_anomaly = !anomalies.is_empty();
        if anomalies.is_empty() {
            anomalies.push("All Normal".to_string());
        }
        
        (anomalies, has_anomaly)
    }
    
    pub fn status_report(&self) -> Vec<String> {
        use chrono::Local;
        
        let now = Local::now();
        let time_str = now.format("%H:%M:%S").to_string();
        
        if let Some(latest) = self.metrics_history.back() {
            vec![
                time_str,
                format!("CPU:{:.1}% RAM:{:.1}%", latest.cpu_percent, latest.ram_percent),
                format!("Disk:{:.1}% Tmp:{:.1}C", latest.disk_percent, latest.temperature),
                format!("Ping:{:.1}ms Net:{}", latest.ping_ms, latest.net_connections),
                format!("Fails:{}", latest.failed_logins),
            ]
        } else {
            vec![time_str, "No data available".to_string()]
        }
    }
    
    pub async fn trigger_actions(&self, anomalies: &[String]) -> Result<()> {
        for anomaly in anomalies {
            if anomaly.contains("Device Down") {
                info!("Device down detected: {}", anomaly);
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("wall")
                        .arg("Device Down Detected!")
                        .spawn();
                }
            }
            
            if anomaly.contains("Temp") {
                // Extract temperature value
                let re = Regex::new(r"Tmp:(\d+(?:\.\d+)?)").unwrap();
                if let Some(caps) = re.captures(anomaly) {
                    if let Ok(temp) = caps[1].parse::<f64>() {
                        if temp > 80.0 {
                            warn!("High temperature detected: {}°C - initiating shutdown", temp);
                            #[cfg(target_os = "linux")]
                            {
                                let _ = Command::new("sudo")
                                    .args(["shutdown", "now"])
                                    .spawn();
                            }
                        }
                    }
                }
            }
            
            if anomaly.contains("Threat IP:") {
                // Extract and validate IP
                let re = Regex::new(r"Threat IP:\s*([\d.]+)").unwrap();
                if let Some(caps) = re.captures(anomaly) {
                    let ip = &caps[1];
                    if self.is_valid_ip(ip) {
                        info!("Blocking threat IP: {}", ip);
                        #[cfg(target_os = "linux")]
                        {
                            let _ = Command::new("sudo")
                                .args(["iptables", "-A", "INPUT", "-s", ip, "-j", "DROP"])
                                .spawn();
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn is_valid_ip(&self, ip: &str) -> bool {
        let re = Regex::new(r"^(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$").unwrap();
        re.is_match(ip)
    }
    
    pub async fn save_baseline(&self) -> Result<()> {
        let _guard = self.file_lock.lock().await;
        
        // Ensure data directory exists
        if let Err(e) = fs::create_dir_all("data").await {
            warn!("Failed to create data directory: {}", e);
        }
        
        let data = serde_json::json!({
            "baseline": self.baselines,
            "feedback": self.feedback,
        });
        
        // Atomic write: write to temp file then rename
        let temp_file = format!("{}.tmp", BASELINE_FILE);
        fs::write(&temp_file, serde_json::to_string_pretty(&data)?).await?;
        fs::rename(&temp_file, BASELINE_FILE).await?;
        
        Ok(())
    }
    
    fn load_baseline(&mut self) -> Result<()> {
        if let Ok(content) = std::fs::read_to_string(BASELINE_FILE) {
            let data: serde_json::Value = serde_json::from_str(&content)?;
            
            if let Some(baseline) = data.get("baseline") {
                self.baselines = serde_json::from_value(baseline.clone())?;
            }
            
            if let Some(feedback) = data.get("feedback") {
                self.feedback = serde_json::from_value(feedback.clone())?;
            }
        }
        
        Ok(())
    }
    
    pub fn get_metrics_history(&self) -> &VecDeque<SystemMetrics> {
        &self.metrics_history
    }
}

fn calculate_stats(values: &[f64]) -> Option<BaselineStats> {
    if values.is_empty() {
        return None;
    }
    
    let sum: f64 = values.iter().sum();
    let mean = sum / values.len() as f64;
    
    let variance_sum: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
    let std = (variance_sum / values.len() as f64).sqrt();
    
    Some(BaselineStats {
        mean,
        std,
        sample_count: values.len(),
    })
}

fn parse_ping_time(output: &str) -> Option<f64> {
    // Parse time=XX.Xms or time=XX ms patterns
    for line in output.lines() {
        if let Some(pos) = line.find("time=") {
            let time_part = &line[pos + 5..];
            let time_str: String = time_part.chars().take_while(|c| c.is_digit(10) || *c == '.').collect();
            if let Ok(time_ms) = time_str.parse::<f64>() {
                return Some(time_ms);
            }
        }
    }
    None
}
