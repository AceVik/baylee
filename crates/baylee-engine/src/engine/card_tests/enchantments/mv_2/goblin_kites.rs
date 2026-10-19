//! `cards/enchantments/mv_2/goblin_kites.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kites_card() -> CardIndex {
    card_index("6efd11c8-be8a-4d44-a776-66d058638ad1")
}
fn raiders_card() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}
fn hill_giant_card() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Every flip the journal holds, in order, as "won".
fn flips(engine: &Engine<RegistryLookup>) -> Vec<bool> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { won, .. } => Some(won),
            _ => None,
        })
        .collect()
}

/// A board with Goblin Kites, a Mountain, a 1/1 Goblin and a 3/3 Giant, at
/// p0's main phase; Kites is activated on the Goblin, and the ability has
/// resolved. Returns the engine, the Goblin and the Giant.
fn kited(seed: u64) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(seed, forest())
        .battlefield(
            0,
            &[kites_card(), mountain(), raiders_card(), hill_giant_card()],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let goblin = on_battlefield(&e, P0, raiders_card()).expect("the Goblin");
    let giant = on_battlefield(&e, P0, hill_giant_card()).expect("the Giant");

    tap_all_mana(&mut e, P0);
    activate(&mut e, P0, kites_card(), 0);
    let menu = aim_at(&mut e, P0, goblin);
    assert!(menu.contains(&goblin), "toughness 1 is on the menu");
    assert!(!menu.contains(&giant), "toughness 3 is not");
    pass_until(&mut e, stack_is_empty);
    (e, goblin, giant)
}

/// Goblin Kites (6efd11c8-be8a-4d44-a776-66d058638ad1) — "{R}: Target creature
/// you control with toughness 2 or less gains flying until end of turn."
#[test]
fn goblin_kites_gives_flying_until_end_of_turn() {
    let mut checked = false;
    for seed in 1..=16u64 {
        let (mut e, goblin, giant) = kited(seed);
        assert!(
            keywords(&e, goblin).contains(KeywordSet::FLYING),
            "the Goblin flies"
        );
        assert!(
            !keywords(&e, giant).contains(KeywordSet::FLYING),
            "the Giant does not"
        );

        // Take a seed where the Goblin wins its flip and so is still there
        // on the next turn, to read the flying after the end of the turn.
        pass_until(&mut e, |e| !flips(e).is_empty() && stack_is_empty(e));
        if e.state().object(goblin).map(|o| o.zone) == Some(Zone::Battlefield) {
            reach_their_main_phase(&mut e, P1);
            assert!(
                !keywords(&e, goblin).contains(KeywordSet::FLYING),
                "flying lasted only until end of turn"
            );
            checked = true;
            break;
        }
    }
    assert!(checked, "some seed in sixteen wins the flip");
}

/// Goblin Kites — "Flip a coin at the beginning of the next end step. If you
/// lose the flip, sacrifice that creature." Over a spread of seeds the flip
/// is won and lost, and the Goblin is sacrificed exactly when it is lost.
#[test]
fn goblin_kites_sacrifices_the_creature_exactly_when_the_flip_is_lost() {
    let mut won_seen = false;
    let mut lost_seen = false;
    for seed in 1..=16u64 {
        let (mut e, goblin, _giant) = kited(seed);
        assert!(flips(&e).is_empty(), "no flip before the end step");
        pass_until(&mut e, |e| !flips(e).is_empty() && stack_is_empty(e));
        assert_eq!(flips(&e).len(), 1, "exactly one flip for one activation");
        let won = flips(&e)[0];
        let zone = e.state().object(goblin).map(|o| o.zone);
        if won {
            won_seen = true;
            assert_eq!(zone, Some(Zone::Battlefield), "seed {seed}: won, it stays");
        } else {
            lost_seen = true;
            assert_eq!(
                zone,
                Some(Zone::Graveyard),
                "seed {seed}: lost, it is sacrificed"
            );
        }
        assert!(
            on_battlefield(&e, P0, hill_giant_card()).is_some(),
            "the Giant was never involved"
        );
    }
    assert!(
        won_seen && lost_seen,
        "sixteen seeds show both sides of a coin"
    );
}
