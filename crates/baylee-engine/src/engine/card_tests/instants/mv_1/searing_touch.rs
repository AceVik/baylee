//! `cards/instants/mv_1/searing_touch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::LLANOWAR_ELVES;

fn searing_touch() -> CardIndex {
    card_index("1cd9a3ba-a115-4e59-983d-605e4435f3a8")
}

/// `Searing Touch` ({R}, Instant, `Coverage::Partial`):
/// deals 1 damage to any target.
#[test]
fn searing_touch_deals_one_damage_to_any_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .battlefield(1, &[LLANOWAR_ELVES])
        .hand(0, &[searing_touch()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p1, LLANOWAR_ELVES).unwrap();
    cast_from_hand(&mut engine, p0, searing_touch());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, LLANOWAR_ELVES).is_none());
    assert!(in_graveyard(&engine, p1, LLANOWAR_ELVES).is_some());
}
