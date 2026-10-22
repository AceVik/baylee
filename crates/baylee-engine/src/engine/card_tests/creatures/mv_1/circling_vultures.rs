//! `cards/creatures/mv_1/circling_vultures.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CIRCLING_VULTURES, GRIZZLY_BEARS};

/// `Circling Vultures` ({B}, Creature — Bird 3/2) has flying and sacrifices on
/// upkeep unless you exile a creature card from your graveyard.
#[test]
fn circling_vultures_exiles_creature_from_graveyard_on_upkeep_then_sacrifices_when_empty() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[CIRCLING_VULTURES, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = hand_to_graveyard(&mut engine, p0, GRIZZLY_BEARS);
    cast_from_hand(&mut engine, p0, CIRCLING_VULTURES);
    pass_until(&mut engine, stack_is_empty);

    let vultures = on_battlefield(&engine, p0, CIRCLING_VULTURES).unwrap();
    assert!(keywords(&engine, vultures).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        e.state().turn.number == 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, CIRCLING_VULTURES).is_some());
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p0))
            .contains(&bears)
    );

    pass_until(&mut engine, |e| {
        e.state().turn.number == 5
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
    });

    assert!(in_graveyard(&engine, p0, CIRCLING_VULTURES).is_some());
}
