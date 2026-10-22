//! `cards/creatures/mv_6/commander_greven_il_vec.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn commander_greven_il_vec() -> CardIndex {
    card_index("08fa1bee-cedf-4a29-9b02-cbdcc0926f8f")
}

/// Commander Greven il-Vec (08fa1bee-cedf-4a29-9b02-cbdcc0926f8f) — {3}{B}{B}{B} Legendary Creature — Phyrexian Human Warrior:
/// "Fear. When Commander Greven il-Vec enters, sacrifice a creature."
#[test]
fn commander_greven_il_vec_sacrifices_creature_when_entering() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
            ],
        )
        .hand(0, &[commander_greven_il_vec()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let fodder = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    cast_from_hand(&mut engine, P0, commander_greven_il_vec());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, commander_greven_il_vec()).is_some());
    assert!(in_graveyard(&engine, P0, quiet_creature()).is_some());
}
