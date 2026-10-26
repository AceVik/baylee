//! `cards/creatures/mv_2/spineless_thug.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn spineless_thug_cant_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[card_index("78bbd213-e8f4-40da-aef5-853244e95756")])
        .battlefield(0, &[swamp(); 2])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(
        &mut engine,
        p0,
        card_index("78bbd213-e8f4-40da-aef5-853244e95756"),
    );
    pass_until(&mut engine, stack_is_empty);

    let thug = on_battlefield(
        &engine,
        p0,
        card_index("78bbd213-e8f4-40da-aef5-853244e95756"),
    )
    .unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();
    assert!(keywords(&engine, thug).contains(KeywordSet::CANT_BLOCK));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(thug, bears)],
                },
            )
            .is_err()
    );
}
