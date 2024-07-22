use std::time::Duration;

use anyhow::anyhow;
use tracing::{info, warn};

pub type Update = serde_json::Value;

const SLEEP_DURATION: Duration = Duration::from_secs(60);

pub fn subscribe(race: &str) -> tokio::sync::watch::Receiver<Update> {
    info!("Starting subscription to {race}");

    let (tx, rx) = tokio::sync::watch::channel(serde_json::Value::Null);

    let url = format!("https://www2.followmychallenge.com/live/{race}/");

    tokio::task::spawn(async move {
        loop {
            match poll(&url).await {
                Ok(json) => tx.send(json).expect("Failed to send msg to channel"),
                Err(err) => warn!("Failure while polling {url}: {}", err),
            };

            info!("Sleeping {SLEEP_DURATION:?} before polling {url}");
            tokio::time::sleep(SLEEP_DURATION).await;
        }
    });

    rx
}

async fn poll(url: &str) -> Result<serde_json::Value, anyhow::Error> {
    const PREFIX: &'static str = "var ridersArray = ";
    let text = reqwest::get(url).await?.text().await?;
    let line = text
        .lines()
        .find_map(|line| line.trim().strip_prefix(PREFIX))
        .ok_or(anyhow!("Failed to find {PREFIX:?} line"))?;

    let json = serde_json::StreamDeserializer::new(serde_json::de::StrRead::new(line))
        .next()
        .ok_or(anyhow!("Failed to parse JSON"))??;

    Ok(json)
}
