use reqwest::{header::USER_AGENT, RequestBuilder};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Address {
    city: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    county: Option<String>,
    leisure: Option<String>,
    municipality: Option<String>,
    postcode: Option<String>,
    region: Option<String>,
    road: Option<String>,
    state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Location {
    address: Address,
    lat: String,
    lon: String,
    osm_id: Option<u64>,
    osm_type: Option<String>,
    place_id: Option<u64>,
    boundingbox: Option<Vec<String>>,
}

impl Location {
    pub fn as_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

pub async fn address_info(lat: f32, lon: f32) -> Result<Location, reqwest::Error> {
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
