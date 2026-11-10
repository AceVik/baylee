//! `cards/instants/mv_1/off_balance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn off_balance_prevents_target_creature_from_attacking_or_blocking_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[card_index("38d9401e-2840-455d-8478-8c976588d4ab")])
        .battlefield(0, &[plains(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    cast_from_hand(
        &mut engine,
        p0,
        card_index("38d9401e-2840-455d-8478-8c976588d4ab"),
    );
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::CANT_ATTACK));
    assert!(keywords(&engine, bears).contains(KeywordSet::CANT_BLOCK));

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(bears, Defender::Player(p1))],
                },
            )
            .is_err(),
        "Grizzly Bears cannot attack this turn"
    );
}
