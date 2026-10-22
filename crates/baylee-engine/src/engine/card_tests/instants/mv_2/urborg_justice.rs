//! `cards/instants/mv_2/urborg_justice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{LIGHTNING_BOLT, MOUNTAIN};

fn urborg_justice() -> CardIndex {
    card_index("d8e2dea9-0e96-4ce4-a070-9893f44cf0cc")
}

/// `Urborg Justice` ({B}{B}, Instant, Partial) forces target opponent to
/// sacrifice a creature if a creature was put into your graveyard from the
/// battlefield this turn.
#[test]
fn urborg_justice_forces_target_opponent_to_sacrifice_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), MOUNTAIN, quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .hand(0, &[LIGHTNING_BOLT, urborg_justice()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let own_creature = on_battlefield(&engine, p0, quiet_creature()).expect("own creature");
    let opp_creature = on_battlefield(&engine, p1, quiet_creature()).expect("opp creature");
    cast_from_hand(&mut engine, p0, LIGHTNING_BOLT);
    aim_at(&mut engine, p0, own_creature);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, p0, urborg_justice());
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![opp_creature],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, quiet_creature()).is_none());
    assert!(in_graveyard(&engine, p1, quiet_creature()).is_some());
}
