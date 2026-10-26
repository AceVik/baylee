//! `cards/creatures/mv_1/netter_en_dal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn netter_en_dal_prevents_target_creature_from_attacking_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[index::GRIZZLY_BEARS])
        .battlefield(
            1,
            &[card_index("8cd65dcd-e36d-47b1-b011-961c79004cb0"), plains()],
        )
        .hand(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("Grizzly Bears");
    let discard_card = in_hand(&engine, p1, plains()).expect("Plains in p1 hand");
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    tap_all_mana(&mut engine, p1);
    activate(
        &mut engine,
        p1,
        card_index("8cd65dcd-e36d-47b1-b011-961c79004cb0"),
        0,
    );
    aim_at(&mut engine, p1, bears);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![discard_card],
            },
        )
        .expect("discard card");
    pass_until(&mut engine, stack_is_empty);
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
