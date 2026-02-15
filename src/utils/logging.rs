use anyhow::Result;
use tracing::{info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() -> Result<()> {
    // Create logs directory
    std::fs::create_dir_all("logs")?;
    
    // Build the subscriber
    let subscriber = tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=warn,hyper=warn")),
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true),
        )
        .with(
            fmt::layer()
                .with_writer(|| std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/app.log")
                    .unwrap_or_else(|_| std::fs::File::create("/dev/null").unwrap()))
                .with_ansi(false)
                .json(),
        );
    
    subscriber.init();
    
    info!("Logging initialized");
    Ok(())
}
