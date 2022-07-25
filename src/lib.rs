use std::collections::HashMap;

use tokio::sync::watch;
use tracing::log::warn;

pub mod fmc;
pub mod nominatim;

#[derive(Debug)]
pub struct Subscriptions {
    races: HashMap<String, watch::Receiver<fmc::Update>>,
    caps: HashMap<(String, u64), watch::Receiver<nominatim::Update>>,
}

impl Subscriptions {
    pub fn new() -> Self {
        Self {
            races: Default::default(),
            caps: Default::default(),
        }
    }

    pub fn race(&mut self, race: &str) -> watch::Receiver<fmc::Update> {
        self.races
            .entry(race.to_owned())
            .or_insert_with(|| fmc::subscribe(race))
            // .or_insert_with(|| todo!())
            .clone()
    }

    pub fn cap(&mut self, race_name: &str, cap: u64) -> watch::Receiver<nominatim::Update> {
        let race = self.race(race_name);
        self.caps
            .entry((race_name.to_owned(), cap))
            .or_insert_with(|| location_updates(race_name, race, cap))
            // .or_insert_with(|| todo!())
            .clone()
    }
}

fn location_updates(
    race_name: &str,
    race: watch::Receiver<fmc::Update>,
    cap: u64,
) -> watch::Receiver<nominatim::Update> {
    let mut race = race;
    let race_name = race_name.to_owned();
    let (tx, rx) = tokio::sync::watch::channel(serde_json::Value::Null);
    tokio::task::spawn(async move {
        let mut last_coords = (0.0, 0.0);

        loop {
            let update = race.borrow().to_owned();
            if let serde_json::Value::Object(update) = update {
                for update in update.values() {
                    let update_cap = update
                        .get("teamNumber")
                        .and_then(|j| j.as_str())
                        .and_then(|s| u64::from_str_radix(s, 10).ok());
                    let update_cap = if let Some(c) = update_cap {
                        c
                    } else {
                        warn!(
                            "Couldn't parse 'teamNumber': '{:?}'",
                            update.get("teamNumber")
                        );
                        continue;
                    };

                    if update_cap == cap {
                        let lat = update["latitude"].as_f64().expect("latitude not an f64") as f32;
                        let lon =
                            update["longitude"].as_f64().expect("longitude not an f64") as f32;

                        if (lat, lon) != last_coords {
                            let location = nominatim::address_info(lat, lon)
                                .await
                                .expect("Failed to get location");
                            tx.send(location).expect("Send failed");
                            last_coords = (lat, lon);
                        }

                        break;
                    }
                }
            }

            if let Some(err) = race.changed().await.err() {
                warn!("Error in location_updates for {race_name} {cap}: {err:?}");
                break;
            }
        }
    });

    rx
}
