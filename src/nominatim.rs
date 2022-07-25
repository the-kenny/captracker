use reqwest::{header::USER_AGENT, RequestBuilder};
use tracing::log::info;

pub type Update = serde_json::Value;

pub async fn address_info(lat: f32, lon: f32) -> Result<Update, reqwest::Error> {
    let url =
        format!("https://nominatim.openstreetmap.org/reverse?lat={lat}&lon={lon}&format=json");

    info!("Fetching {url}");
    let client = reqwest::Client::new();
    client
        .get(url)
        .header(USER_AGENT, "captracker 1.0")
        .send()
        .await?
        .json()
        .await
}
