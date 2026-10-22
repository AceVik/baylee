//! `cards/creatures/mv_4/mistmoon_griffin.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, LIGHTNING_BOLT, MISTMOON_GRIFFIN};

/// `Mistmoon Griffin` ({3}{W}, Creature — Griffin 2/2, Partial) has flying and
/// exiles itself when it dies, returning a creature card from your graveyard to
/// the battlefield.
#[test]
fn mistmoon_griffin_exiles_self_and_returns_creature_from_graveyard_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[MISTMOON_GRIFFIN, mountain()])
        .hand(0, &[LIGHTNING_BOLT, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let griffin = on_battlefield(&engine, p0, MISTMOON_GRIFFIN).unwrap();
    let bears = hand_to_graveyard(&mut engine, p0, GRIZZLY_BEARS);
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, griffin);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
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

    assert!(in_graveyard(&engine, p0, MISTMOON_GRIFFIN).is_none());
    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}
