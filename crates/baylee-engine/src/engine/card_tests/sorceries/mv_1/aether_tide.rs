//! `cards/sorceries/mv_1/aether_tide.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aether_tide() -> CardIndex {
    card_index("2fbf95b4-bcf4-4b5e-b5dc-0294f2b48d3e")
}

/// Aether Tide (`2fbf95b4-bcf4-4b5e-b5dc-0294f2b48d3e`) — {X}{U} Sorcery (Partial):
/// "As an additional cost to cast this spell, discard X creature cards. Return X target creatures to their owners' hands."
#[test]
fn aether_tide_returns_x_target_creatures_to_owners_hands() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(0, &[aether_tide()])
        .start();
    assert!(walk_to_own_main(&mut engine, P0));

    let bear = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, aether_tide());
    kit::settle(&mut engine, P0, |_, p| {
        if matches!(p, Pending::ChooseNumber { .. }) {
            return Some(PlayerAction::ChooseNumber(1));
        }
        kit::choose_targets(p, &[bear], &[])
    });

    assert!(on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_none());
    assert!(in_hand(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
