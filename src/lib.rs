use std::{collections::HashMap, future};

use futures::{Stream, StreamExt};
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tracing::{info, warn};

pub mod fmc;
pub mod nominatim;

#[derive(Debug)]
pub struct Subscriptions {
    races: HashMap<String, watch::Receiver<fmc::Update>>,
    caps: HashMap<(String, u64), UpdateStream<nominatim::Location>>,
}

impl Subscriptions {
    pub fn new() -> Self {
        Self {
            races: Default::default(),
            caps: Default::default(),
        }
    }

    pub fn race(&mut self, race: &str) -> watch::Receiver<fmc::Update> {
        info!("Subscribing to {race}");

        self.races
            .entry(race.to_owned())
            .or_insert_with(|| fmc::subscribe(race))
            .clone()
    }

    pub fn cap(&mut self, race_name: &str, cap: u64) -> UpdateStream<nominatim::Location> {
        let race = self.race(race_name);

        info!("Subscribing to {race_name} cap {cap}");

        self.caps
            .entry((race_name.to_owned(), cap))
            .or_insert_with(|| location_updates(race_name, race, cap))
            .clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update<T> {
    Pending,
    New(T),
}

impl<T> Update<T> {
    pub fn into_inner(self) -> Option<T> {
        match self {
            Update::Pending => None,
            Update::New(x) => Some(x),
        }
    }
}

type UpdateStream<T> = watch::Receiver<Update<T>>;

fn location_updates(
    race_name: &str,
    race: watch::Receiver<fmc::Update>,
    cap: u64,
) -> watch::Receiver<Update<nominatim::Location>> {
    let race_name = race_name.to_owned();
    let (tx, rx) = tokio::sync::watch::channel(Update::Pending);
    tokio::task::spawn(async move {
        WatchStream::new(race)
            .flat_map(|update| futures::stream::iter(update.as_object().cloned()))
            .flat_map(|o| futures::stream::iter(o))
            .map(|(_, v)| v)
            .flat_map(|cap| futures::stream::iter(cap.as_object().cloned()))
            .filter(|update| future::ready(team_number(update) == Some(cap)))
            .scan((0.0, 0.0), |state, update| {
                let lat = update["latitude"].as_f64().expect("latitude not an f64") as f32;
                let lon = update["longitude"].as_f64().expect("longitude not an f64") as f32;
                let last_coordinates = *state;
                *state = (lat, lon);
                async move {
                    if last_coordinates != (lat, lon) {
                        let location = nominatim::address_info(lat, lon)
                            .await
                            .expect("Failed to get location");
                        Some(futures::stream::iter(Some(location)))
                    } else {
                        Some(futures::stream::iter(Option::<nominatim::Location>::None))
                    }
                }
            })
            .flatten()
            .for_each(|location| async {
                info!("New Location for {race_name} {cap}: {location:?}");
                tx.send(Update::New(location)).expect("Send failed")
            })
            .await;
    });

    rx
}

fn team_number(rider: &serde_json::Map<String, serde_json::Value>) -> Option<u64> {
    rider
        .get("teamNumber")
        .and_then(|j| j.as_str())
        .and_then(|s| u64::from_str_radix(s, 10).ok())
}
