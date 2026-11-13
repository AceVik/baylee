use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use baylee_cards::deckgen::{Archetype, generate};
use baylee_cards::decks::{house_deck, preset_for};
use baylee_cards::formats::{self, Shape};
use baylee_core::preset::{AIProfile, GamePreset, HouseRules, SeatController};

use super::Kind;
use crate::working::{Working, repo_root};

pub(super) fn build(kind: Kind, seed: u64) -> Result<(GamePreset, String)> {
    let working = Working::scan(&repo_root()).context("working card evidence")?;
    let (mut preset, deck) = match kind {
        Kind::Daily => daily(seed)?,
        Kind::Weekly => {
            let pool = crate::deckgen::pool_for_working(&working, &BTreeSet::new());
            let player = generate(
                &pool,
                Shape::Constructed,
                Archetype::Aggro,
                baylee_challenges::seed(b"player deck", seed),
            )?;
            let opponent = generate(
                &pool,
                Shape::Constructed,
                Archetype::Midrange,
                baylee_challenges::seed(b"opponent deck", seed),
            )?;
            ensure!(
                pool.entries
                    .iter()
                    .any(|entry| player.main.contains(&entry.card)
                        && entry.profile.roles & baylee_cards::deckgen::profile::ROLE_BURN_DRAIN
                            != 0),
                "deck has no direct-damage candidate for a second victory family"
            );
            ensure!(
                formats::check(Shape::Constructed, &player.main, &[]).is_empty(),
                "illegal player deck"
            );
            ensure!(
                formats::check(Shape::Constructed, &opponent.main, &[]).is_empty(),
                "illegal opponent deck"
            );
            (
                preset_for(seed, &player.loaded(), &opponent.loaded()),
                player.text(),
            )
        }
    };
    for seat in &preset.seats {
        for card in seat
            .deck
            .iter()
            .chain(seat.starting_hand.iter().flatten())
            .chain(&seat.starting_battlefield)
        {
            ensure!(
                working.check(card.card).is_ok(),
                "candidate contains a card without working evidence: {}",
                card.card
            );
        }
    }
    preset.seats[0].controller = SeatController::Open;
    preset.seats[1].controller = SeatController::Ai(AIProfile {
        foresight: 0,
        ..AIProfile::STEADY
    });
    preset.house_rules = HouseRules {
        challenge_policy_seed: Some(baylee_challenges::seed(b"opponent policy", seed)),
        starting_player: Some(0),
        decision_timeout_secs: 0,
        reconnect_window_secs: 0,
        time_extension_votes: false,
        takebacks: false,
        ..HouseRules::for_a_table()
    };
    Ok((preset, deck))
}

fn daily(seed: u64) -> Result<(GamePreset, String)> {
    // All physical identities come through the ordinary deck reader, including prints.
    let text = "[deck:Crossroads]\n24 Mountain\n16 Forest\n4 Grizzly Bears\n4 Lightning Bolt\n4 Shock\n4 Giant Growth\n4 Fling\n";
    let player = house_deck(text).map_err(anyhow::Error::msg)?;
    let opponent =
        house_deck("[deck:Pressure]\n56 Forest\n4 Grizzly Bears\n").map_err(anyhow::Error::msg)?;
    let mut preset = preset_for(seed, &player, &opponent);
    let entry = |name: &str| -> Result<_> {
        let card = baylee_cards::decks::by_name(name).context("tactical card")?;
        preset.seats[0]
            .deck
            .iter()
            .find(|entry| entry.card == card)
            .copied()
            .context("tactical printing")
    };
    let hand = [
        "Lightning Bolt",
        "Shock",
        "Giant Growth",
        "Fling",
        "Mountain",
        "Forest",
    ]
    .map(entry)
    .into_iter()
    .collect::<Result<Vec<_>>>()?;
    let field = [
        "Mountain",
        "Mountain",
        "Mountain",
        "Forest",
        "Forest",
        "Grizzly Bears",
        "Grizzly Bears",
    ]
    .map(entry)
    .into_iter()
    .collect::<Result<Vec<_>>>()?;
    preset.seats[0].starting_hand = Some(hand);
    preset.seats[0].starting_battlefield = field;
    preset.seats[0].starting_life = Some(2);
    let bear = baylee_cards::decks::by_name("Grizzly Bears").context("opponent creature")?;
    preset.seats[1].starting_battlefield = vec![
        *preset.seats[1]
            .deck
            .iter()
            .find(|entry| entry.card == bear)
            .context("opponent printing")?,
    ];
    preset.seats[1].starting_hand = Some(Vec::new());
    preset.seats[1].starting_life = Some(5);
    Ok((preset, text.into()))
}
