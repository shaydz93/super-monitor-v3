use crate::models::metrics::ThreatIntel;
use anyhow::{anyhow, Result};
use rss::Channel;
use std::collections::HashMap;

pub struct ThreatIntelService {
    feeds: Vec<(String, String)>,
    data: HashMap<String, Vec<ThreatIntel>>,
}

impl ThreatIntelService {
    pub fn new() -> Self {
        let feeds = vec![
            ("CISA".to_string(), "https://www.cisa.gov/news-events/cybersecurity-advisories.xml".to_string()),
            ("BleepingComputer".to_string(), "https://www.bleepingcomputer.com/feed/".to_string()),
            ("KrebsOnSecurity".to_string(), "https://krebsonsecurity.com/feed/".to_string()),
            ("TheHackerNews".to_string(), "https://thehackernews.com/feeds/posts/default".to_string()),
        ];
        
        Self {
            feeds,
            data: HashMap::new(),
        }
    }
    
    pub async fn fetch_all(&mut self) -> Result<()> {
        let client = reqwest::Client::builder()
            .user_agent("ShaydZ-SuperMonitor/2.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        for (name, url) in &self.feeds {
            match self.fetch_feed(&client, name, url).await {
                Ok(items) => {
                    self.data.insert(name.clone(), items);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch feed {}: {}", name, e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn fetch_feed(&self, client: &reqwest::Client, name: &str, url: &str) -> Result<Vec<ThreatIntel>> {
        let response = client.get(url).send().await?;
        let content = response.text().await?;
        
        let channel = Channel::read_from(content.as_bytes())
            .map_err(|e| anyhow!("Failed to parse RSS feed: {:?}", e))?;
        
        let mut items = Vec::new();
        
        for item in channel.items().iter().take(5) {
            let intel = ThreatIntel {
                source: name.to_string(),
                title: item.title().unwrap_or("Untitled").to_string(),
                url: item.link().unwrap_or("#").to_string(),
                published: item.pub_date().and_then(|d| {
                    chrono::DateTime::parse_from_rfc2822(d).ok().map(|dt| dt.with_timezone(&chrono::Utc))
                }),
            };
            items.push(intel);
        }
        
        Ok(items)
    }
    
    pub fn get_data(&self) -> &HashMap<String, Vec<ThreatIntel>> {
        &self.data
    }
}
