//! `cards/instants/mv_2/daze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Daze ({1}{U} Instant):
/// "Counter target spell unless its controller pays {1}."
#[test]
fn daze_counters_target_spell_unless_its_controller_pays_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island()])
        .hand(0, &[card_index("70486bee-6ee7-41ea-b834-8caf4699302b")])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    cast_from_hand(&mut engine, p1, grizzly_bears());
    let spell = on_stack(&engine, grizzly_bears()).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    cast_from_hand(
        &mut engine,
        p0,
        card_index("70486bee-6ee7-41ea-b834-8caf4699302b"),
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![spell],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
}
