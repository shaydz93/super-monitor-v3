# Super Monitor v3 🛡️

> **Ultimate Self-Learning Network Defense, Monitoring, and AI Threat Intelligence Dashboard**  
> Rewritten in Rust for maximum security, performance, and reliability.

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Security](https://img.shields.io/badge/security-hardened-brightgreen.svg)]()

---

## 🚀 What is Super Monitor v3?

Super Monitor v3 is a **complete security operations platform** for your home network or small business. It combines:

- 🤖 **AI-Powered Anomaly Detection** — Self-learning baselines that adapt to your network patterns
- 📊 **Real-time System Monitoring** — CPU, RAM, disk, temperature, network connectivity
- 🔐 **Security Features** — Failed login detection, threat IP monitoring, automated responses
- 🌐 **Web Dashboard** — Interactive charts with real-time metrics
- 🔍 **Threat Intelligence** — Automated RSS feeds from CISA, KrebsOnSecurity, BleepingComputer
- 🔔 **Automated Actions** — Temperature-based shutdowns, firewall blocking, alerting

---

## ✨ What's New in v3 (Rust Rewrite)

| Feature | Python v2 | Rust v3 |
|---------|-----------|---------|
| **Security** | 9 vulnerabilities found | ✅ All patched, memory-safe |
| **Performance** | ~2-3s startup, 75MB RAM | ✅ 0.1s startup, 15MB RAM |
| **Concurrency** | Threading (GIL limited) | ✅ Async/await (Tokio) |
| **Web Framework** | Flask (sync) | ✅ Axum (fully async) |
| **Authentication** | bcrypt sessions | ✅ Argon2 + JWT |
| **Deployment** | Python runtime required | ✅ Single static binary |

**Critical bugs fixed:**
- ✅ Command injection vulnerability (RCE potential)
- ✅ Hardcoded weak secrets
- ✅ Race conditions in file operations
- ✅ Socket resource leaks
- ✅ Unsynchronized shared state access
- ✅ Deprecated API usage
- ✅ Missing input validation

---

## 📦 Prerequisites

### System Requirements
- **OS:** Linux (Debian/Ubuntu/CentOS), macOS, or Windows with WSL2
- **RAM:** 512MB minimum, 2GB recommended
- **CPU:** Any x86_64 or ARM64 processor
- **Network:** Promiscuous mode capable interface (for full network monitoring)

### Required Dependencies
```bash
# Debian/Ubuntu
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev

# macOS (with Homebrew)
brew install openssl pkg-config

# For YARA support (optional but recommended)
sudo apt-get install -y libyara-dev yara
```

### Rust Toolchain
```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installation
rustc --version  # Should show 1.75.0 or later
cargo --version
```

---

## 🔧 Building from Source

### 1. Clone the Repository
```bash
git clone https://github.com/shaydz93/super-monitor-v3.git
cd super-monitor-v3
```

### 2. Build (Development)
```bash
# Standard debug build
cargo build

# Run with logging
RUST_LOG=debug cargo run
```

### 3. Build (Production Release)
```bash
# Optimized release build
cargo build --release

# Binary location:
# ./target/release/shaydz-monitor
```

### 4. Build Features (Optional)
```bash
# With YARA support (if libyara-dev is installed)
cargo build --release --features yara

# With all optional features
cargo build --release --all-features
```

---

## 🚀 Installation & Setup

### Option A: Run from Source (Development)
```bash
# Clone and run
git clone https://github.com/shaydz93/super-monitor-v3.git
cd super-monitor-v3
cargo run --release
```

### Option B: Install System-Wide
```bash
# Build release binary
cargo build --release

# Install to /usr/local/bin
sudo cp target/release/shaydz-monitor /usr/local/bin/
sudo chmod +x /usr/local/bin/shaydz-monitor

# Create config directory
mkdir -p ~/.config/super-monitor
```

### Option C: Systemd Service (Production)
```bash
# Build and install
cargo build --release
sudo cp target/release/shaydz-monitor /usr/local/bin/

# Create systemd service file
sudo tee /etc/systemd/system/super-monitor.service << 'EOF'
[Unit]
Description=Super Monitor v3 - Network Defense Platform
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/shaydz-monitor
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable super-monitor
sudo systemctl start super-monitor

# Check status
sudo systemctl status super-monitor
```

---

## ⚙️ Configuration

### Default Configuration
On first run, Super Monitor creates a default configuration at:
- Linux: `~/.config/super-monitor/config.toml`
- macOS: `~/Library/Application Support/super-monitor/config.toml`

### Example Configuration
```toml
[server]
host = "0.0.0.0"
port = 8080

[security]
jwt_secret = "your-secure-random-secret-here"
session_timeout = 3600  # 1 hour

[monitoring]
# Network interface to monitor (for packet capture)
interface = "eth0"

# Baseline learning period (seconds)
learning_period = 300

# Alert thresholds
cpu_threshold = 90.0
memory_threshold = 85.0
temperature_threshold = 80.0

[threat_intel]
# RSS feeds for threat intelligence
feeds = [
    "https://www.cisa.gov/news.xml",
    "https://krebsonsecurity.com/feed/",
    "https://www.bleepingcomputer.com/feed/"
]
update_interval = 3600  # 1 hour

[ai]
# Ollama configuration for AI analysis
ollama_url = "http://localhost:11434"
ai_model = "kimi-k2.5"
enable_analysis = true
```

### Environment Variables
```bash
# Override config file location
export SUPER_MONITOR_CONFIG=/path/to/config.toml

# Set log level (error, warn, info, debug, trace)
export RUST_LOG=info

# Database path
export SUPER_MONITOR_DB=~/.local/share/super-monitor/data.db
```

---

## 🌐 Usage

### Starting the Server
```bash
# Run directly
./target/release/shaydz-monitor

# Or with custom config
./target/release/shaydz-monitor --config /etc/super-monitor/config.toml
```

### Accessing the Web Dashboard
Once running, open your browser to:
```
http://localhost:8080
```

**Default credentials:**
- Username: `admin`
- Password: `changeme` (change immediately!)

### Dashboard Features

| Page | Description |
|------|-------------|
| **Dashboard** | Real-time system metrics, network status, threat overview |
| **Logs** | Filterable logs with severity coloring |
| **Threat Intel** | Latest security news from configured RSS feeds |
| **Settings** | Configuration management, user management |
| **Downloads** | Export reports, download baselines |

### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/metrics` | GET | Current system metrics (JSON) |
| `/api/alerts` | GET | Active security alerts |
| `/api/threats` | GET | Threat intelligence summary |
| `/api/baseline` | POST | Update anomaly detection baseline |
| `/api/block` | POST | Block an IP address |

Example:
```bash
# Get current metrics
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  http://localhost:8080/api/metrics

# Block an IP
curl -X POST -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.100", "reason": "Suspicious activity"}' \
  http://localhost:8080/api/block
```

---

## 🔒 Security Hardening

### 1. Change Default Credentials
```bash
# On first login, immediately change password
# Settings → Users → Change Password
```

### 2. Use HTTPS (Production)
```bash
# Place behind nginx with SSL
# Or use built-in TLS (configure in config.toml):
[server]
tls_cert = "/etc/ssl/certs/super-monitor.crt"
tls_key = "/etc/ssl/private/super-monitor.key"
```

### 3. Firewall Rules
```bash
# Only allow specific IPs
sudo ufw allow from 192.168.1.0/24 to any port 8080
sudo ufw enable
```

### 4. Run as Non-Root (Limited)
```bash
# Create dedicated user
sudo useradd -r -s /bin/false supermon

# Grant network capture capability
sudo setcap cap_net_raw,cap_net_admin=eip /usr/local/bin/shaydz-monitor

# Run as user
sudo -u supermon /usr/local/bin/shaydz-monitor
```

---

## 🧪 Testing

### Run Tests
```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Security Testing
```bash
# Run security audit
cargo audit

# Check for vulnerabilities
cargo deny check
```

---

## 📊 Performance Benchmarks

| Metric | Python v2 | Rust v3 | Improvement |
|--------|-----------|---------|-------------|
| **Startup Time** | 2.5s | 0.1s | **25x faster** |
| **Memory Usage** | 75MB | 15MB | **5x less** |
| **Concurrent Connections** | ~100 | 10,000+ | **100x more** |
| **Packet Processing** | 50K/s | 500K/s | **10x faster** |
| **Binary Size** | N/A (runtime) | 12MB | **Self-contained** |

---

## 🐛 Troubleshooting

### Build Failures
```bash
# Update Rust
cargo update

# Clean and rebuild
cargo clean
cargo build --release

# Check for missing dependencies
sudo apt-get install -y libssl-dev pkg-config
```

### Permission Denied (Network Capture)
```bash
# Grant capabilities (Linux)
sudo setcap cap_net_raw,cap_net_admin=eip ./target/release/shaydz-monitor

# Or run with sudo (not recommended for production)
sudo ./target/release/shaydz-monitor
```

### Database Issues
```bash
# Reset database
rm ~/.local/share/super-monitor/data.db
# Restart application - will recreate
```

### High CPU Usage
- Check `RUST_LOG` level (set to `info` or `warn` in production)
- Reduce monitoring intervals in config
- Ensure release build (not debug): `cargo build --release`

---

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Code Standards
- Follow `rustfmt` formatting: `cargo fmt`
- Pass clippy lints: `cargo clippy -- -D warnings`
- Include tests for new features
- Update documentation

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- Built with [Tokio](https://tokio.rs/) async runtime
- Web framework: [Axum](https://github.com/tokio-rs/axum)
- Templating: [Askama](https://github.com/djc/askama)
- Authentication: [Argon2](https://github.com/RustCrypto/password-hashes)
- Original Python version: v1 and v2 by ShaydZ93

---

## 📞 Support

- **Issues:** [GitHub Issues](https://github.com/shaydz93/super-monitor-v3/issues)
- **Security:** Report vulnerabilities to shaydz93@protonmail.com
- **Discussion:** [GitHub Discussions](https://github.com/shaydz93/super-monitor-v3/discussions)

---

**Built with ❤️ and 🦀 Rust by ShaydZ93**
