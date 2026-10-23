//! `cards/instants/mv_4/reins_of_power.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn reins_of_power() -> CardIndex {
    card_index("96381388-1e82-411c-8290-1f3e909d3b5f")
}

/// Reins of Power (`96381388-1e82-411c-8290-1f3e909d3b5f`) — {2}{U}{U} Instant (Partial):
/// "Untap all creatures you control and all creatures target opponent controls... Those creatures gain haste until end of turn."
#[test]
fn reins_of_power_untaps_your_and_opponent_creatures_and_grants_haste() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[index::GRIZZLY_BEARS, island(), island(), island(), island()],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[reins_of_power()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(bear)
        .unwrap()
        .status
        .insert(Status::TAPPED);
    engine
        .dev_state_mut(P0)
        .unwrap()
        .object_mut(giant)
        .unwrap()
        .status
        .insert(Status::TAPPED);

    cast_from_hand(&mut engine, P0, reins_of_power());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        !engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(
        !engine
            .state()
            .object(giant)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
    assert!(keywords(&engine, bear).contains(KeywordSet::HASTE));
    assert!(keywords(&engine, giant).contains(KeywordSet::HASTE));
}
