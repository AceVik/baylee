//! `cards/creatures/mv_3/raging_gorilla.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MONS_S_GOBLIN_RAIDERS;

fn raging_gorilla() -> CardIndex {
    card_index("1dd6df3f-eac4-4f30-b832-4a3f3a97632f")
}

/// `Raging Gorilla` ({2}{R}, 2/3 Creature — Ape, `Coverage::Implemented`):
/// `Whenever this creature blocks or becomes blocked, it gets +2/-2 until end
/// of turn.`
#[test]
fn raging_gorilla_gets_plus_2_minus_2_when_blocked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[MONS_S_GOBLIN_RAIDERS])
        .hand(0, &[raging_gorilla()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, raging_gorilla());
    pass_until(&mut engine, stack_is_empty);
    let gorilla = on_battlefield(&engine, p0, raging_gorilla()).unwrap();
    let goblin = on_battlefield(&engine, p1, MONS_S_GOBLIN_RAIDERS).unwrap();
    assert_eq!(pt(&engine, gorilla), (2, 3));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(gorilla, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(goblin, gorilla)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, gorilla), (4, 1));
}
