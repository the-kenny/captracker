use std::time::Duration;

use tracing::{info, warn};

pub type Update = serde_json::Value;

const SLEEP_DURATION: Duration = Duration::from_secs(60);

pub fn subscribe(race: &str) -> tokio::sync::watch::Receiver<Update> {
    info!("Starting subscription to {race}");

    let (tx, rx) = tokio::sync::watch::channel(serde_json::Value::Null);

    let url = format!("https://www.followmychallenge.com/live/{race}/data/ridersArray.json");

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
    let json = reqwest::get(url).await?.json::<serde_json::Value>().await?;

    Ok(json)
}
