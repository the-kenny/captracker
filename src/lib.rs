use std::collections::HashMap;

use tokio::sync::watch;

pub mod fmc;

#[derive(Debug)]
pub struct Subscriptions(HashMap<String, watch::Receiver<fmc::Update>>);

impl Subscriptions {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn get(&mut self, race: &str) -> watch::Receiver<fmc::Update> {
        self.0
            .entry(race.to_owned())
            .or_insert_with(|| fmc::subscribe(race))
            // .or_insert_with(|| todo!())
            .clone()
    }
}
