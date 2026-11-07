//! The ways through the client a player takes again and again, as one call
//! each, so a test spells out only what it is about. Each goes through the
//! interface as a player would (clicks on what is drawn) and returns the
//! screen it arrives at, at rest.
use crate::{
    client::Client,
    screen::{Screen, press},
};
use anyhow::Result;
use std::time::Duration;

impl Client {
    /// From the front door to the offline lobby: no account, no gateway.
    pub fn play_offline(&self) -> Result<Screen> {
        self.click(press("Front(PlayOffline)"))?;
        self.wait_until("the offline lobby", |s| {
            s.flag("/account/offline") && s.control("Play(PlayHouse)").is_some()
        })?;
        self.settled()
    }

    /// From the front door to a duel against the house AI (`LocalHost`),
    /// standing at the opening hand's question.
    pub fn play_the_house(&self) -> Result<Screen> {
        self.play_offline()?;
        self.sit_down_with_the_house()
    }

    /// From the offline lobby to the same place.
    pub fn sit_down_with_the_house(&self) -> Result<Screen> {
        self.click(press("Play(PlayHouse)"))?;
        self.wait_for(
            "the house game's first view",
            Duration::from_secs(60),
            |s| s.in_duel() && s.at_rest() && s.cards_in("hand").next().is_some(),
        )?;
        self.settled()
    }
}
