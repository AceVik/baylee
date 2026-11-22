//! Host-side ranked admission and the only supported ranked table configuration.

use baylee_cards::decks::LoadedDeck;
use baylee_core::preset::{FormatId, GamePreset, HouseRules, LoopPolicy, SeatController};
use central_api::ranked::{Format, PoolEntry};

use crate::{ErrorBody, Json, Shared, StatusCode, db_down, err, err_saying};

type Failure = (StatusCode, Json<ErrorBody>);

async fn legal(state: &Shared, format: Format, deck: &LoadedDeck) -> Result<(), Failure> {
    state
        .blocks
        .check_loaded(deck, time::OffsetDateTime::now_utc())?;
    let unavailable = || {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "ranked legality catalog unavailable",
        )
    };
    let catalog = state.catalog.as_ref().ok_or_else(unavailable)?;
    let held = state
        .pool_facts
        .get(catalog)
        .await
        .map_err(|e| db_down(&e))?;
    let facts = held.table.as_deref().ok_or_else(unavailable)?;
    baylee_cards::ranked::admit(format.as_str(), deck, facts)
        .map_err(|why| err_saying(StatusCode::BAD_REQUEST, why))
}

/// The host computes both proof and rating pool from the cards it resolved.
pub(super) async fn validate(
    state: &Shared,
    format: Format,
    cards: &[String],
    sideboard: &[String],
    commanders: &[String],
) -> Result<(LoadedDeck, [u8; 32], Vec<PoolEntry>), Failure> {
    let reg = baylee_cards::ranked::registration(format.as_str(), cards, sideboard, commanders)
        .map_err(|why| err_saying(StatusCode::BAD_REQUEST, why))?;
    legal(state, format, &reg.deck).await?;
    // Verify alternate physical printings before accepting even the first seat.
    let mut single = baylee_cards::decks::preset_for_all(0, &[&reg.deck]);
    crate::routes::hydrate_rules(state, &mut single).await?;
    let pool = baylee_cards::ranked::rating_pool(&reg)
        .into_iter()
        .map(|(card, copies)| PoolEntry { card, copies })
        .collect();
    Ok((reg.deck, reg.digest, pool))
}

fn duel(format: Format, decks: [&LoadedDeck; 2], seed: u64) -> GamePreset {
    let mut preset = baylee_cards::decks::preset_for_all(seed, &decks);
    let (engine_format, life) = match format {
        Format::Standard => (FormatId::Freeform, 20),
        Format::Highlander => (FormatId::Highlander, 20),
        Format::Commander => (FormatId::Commander, 40),
    };
    preset.format = engine_format;
    preset.house_rules = HouseRules {
        ranked_duel: true,
        mulligan_free_first: false,
        free_mulligans: Some(0),
        loop_policy: LoopPolicy::CompRulesDraw,
        time_extension_votes: false,
        ..HouseRules::for_a_table()
    };
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
        seat.starting_life = Some(life);
    }
    preset
}

pub(super) async fn preset(
    state: &Shared,
    format: Format,
    decks: [&LoadedDeck; 2],
    seed: u64,
) -> Result<GamePreset, Failure> {
    // Recheck both registrations after the second player arrives. A catalog
    // revision or a gateway block may have changed while the first waited.
    for deck in decks {
        legal(state, format, deck).await?;
    }
    let mut preset = duel(format, decks, seed);
    crate::routes::hydrate_rules(state, &mut preset).await?;
    preset
        .validate()
        .map_err(|_| err(StatusCode::BAD_REQUEST, "invalid ranked table"))?;
    Ok(preset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranked_duels_have_two_ordinary_human_seats_and_immutable_rules() {
        let deck = baylee_cards::ranked::registration("standard", &["60 Forest".into()], &[], &[])
            .unwrap()
            .deck;
        for (format, engine_format, life) in [
            (Format::Standard, FormatId::Freeform, 20),
            (Format::Highlander, FormatId::Highlander, 20),
            (Format::Commander, FormatId::Commander, 40),
        ] {
            let preset = duel(format, [&deck, &deck], 789);
            assert_eq!(preset.format, engine_format);
            assert_eq!(preset.seats.len(), 2);
            for seat in &preset.seats {
                assert!(matches!(seat.controller, SeatController::Open));
                assert_eq!(seat.starting_life, Some(life));
                assert!(seat.team.is_none());
                assert!(!seat.capabilities.see_hidden && !seat.capabilities.dev_commands);
                assert!(seat.starting_hand.is_none() && seat.starting_battlefield.is_empty());
            }
            assert!(preset.house_rules.ranked_duel);
            assert!(preset.house_rules.starting_player.is_none());
            assert_eq!(preset.house_rules.free_mulligan_count(), 0);
            assert_eq!(preset.house_rules.loop_policy, LoopPolicy::CompRulesDraw);
            assert!(!preset.house_rules.takebacks && !preset.house_rules.time_extension_votes);
        }
    }
}
