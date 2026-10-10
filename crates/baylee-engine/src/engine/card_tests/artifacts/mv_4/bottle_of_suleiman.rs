//! `cards/artifacts/mv_4/bottle_of_suleiman.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::event::GameEvent;
use baylee_cards_dsl::KeywordSet;

fn bottle_of_suleiman() -> CardIndex {
    card_index("f32d19d6-8ac1-4744-b2c2-5c9d4cd0da70")
}

/// Seeds whose first coin flip, in the game below, is won and lost.
const WON_SEED: u64 = 3;
const LOST_SEED: u64 = 1;

/// Every flip the journal holds, in order, as (who, won).
fn flips(engine: &Engine<RegistryLookup>) -> Vec<(PlayerId, bool)> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { player, won } => Some((player, won)),
            _ => None,
        })
        .collect()
}

/// Seat 0 pays `{1}` and sacrifices the Bottle; the engine rests after the
/// ability has resolved. Returns the engine and the Bottle's old id.
fn bottle_activated(seed: u64) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[bottle_of_suleiman(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, bottle_of_suleiman()).is_some());
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, bottle_of_suleiman(), 0);
    assert!(
        on_battlefield(&engine, p0, bottle_of_suleiman()).is_none(),
        "the sacrifice is a cost: the Bottle is gone before the flip"
    );
    pass_until(&mut engine, |e| !flips(e).is_empty() && stack_is_empty(e));
    engine
}

/// Every object `seat` controls on the battlefield.
fn battlefield_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == seat)
        })
        .collect()
}

/// "If you win the flip, create a 5/5 colorless Djinn artifact creature
/// token with flying." The Bottle is sacrificed either way.
#[test]
fn a_won_flip_makes_a_five_five_flying_djinn_and_costs_no_life() {
    let p0 = PlayerId::new(0);
    let engine = bottle_activated(WON_SEED);
    assert_eq!(flips(&engine), vec![(p0, true)], "one flip, by me, won");
    assert!(
        in_graveyard(&engine, p0, bottle_of_suleiman()).is_some(),
        "the Bottle was sacrificed"
    );

    let djinn = battlefield_of(&engine, p0)
        .into_iter()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_none() && o.token.is_some())
        })
        .expect("a token entered");
    let c = engine
        .state()
        .object(djinn)
        .expect("the token is an object")
        .characteristics();
    assert!(
        c.types.contains(TypeSet::ARTIFACT) && c.types.contains(TypeSet::CREATURE),
        "an artifact creature: {:?}",
        c.types
    );
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::DJINN),
        "a Djinn"
    );
    assert_eq!((c.power, c.toughness), (Some(5), Some(5)), "5/5");
    assert!(c.keywords.contains(KeywordSet::FLYING), "with flying");
    assert!(c.colors.is_empty(), "colorless");
    assert_eq!(engine.state().players[0].life, 20, "no damage on a win");
}

/// "If you lose the flip, this artifact deals 5 damage to you." Exactly 5,
/// and no token.
#[test]
fn a_lost_flip_deals_five_damage_to_its_controller_and_makes_nothing() {
    let p0 = PlayerId::new(0);
    let engine = bottle_activated(LOST_SEED);
    assert_eq!(flips(&engine), vec![(p0, false)], "one flip, by me, lost");
    assert_eq!(engine.state().players[0].life, 15, "20 - 5");
    assert_eq!(engine.state().players[1].life, 20, "and not the opponent");
    assert!(
        in_graveyard(&engine, p0, bottle_of_suleiman()).is_some(),
        "the Bottle was sacrificed all the same"
    );
    assert!(
        battlefield_of(&engine, p0)
            .into_iter()
            .all(|id| engine.state().object(id).is_some_and(|o| o.token.is_none())),
        "no Djinn on a loss"
    );
}
