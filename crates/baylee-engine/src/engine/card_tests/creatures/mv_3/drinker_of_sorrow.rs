//! `cards/creatures/mv_3/drinker_of_sorrow.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Drinker of Sorrow (`6208534d-e7c7-43ef-b8b2-fea1d82459e8`) — {2}{B} Creature — Horror 5/3:
/// "This creature can't block.
/// Whenever this creature deals combat damage, sacrifice a permanent."
#[test]
fn drinker_of_sorrow_cant_block_and_sacrifices_a_permanent_on_combat_damage() {
    let drinker_card = card_index("6208534d-e7c7-43ef-b8b2-fea1d82459e8");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[drinker_card, swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let drinker = on_battlefield(&engine, p0, drinker_card).unwrap();
    let land = on_battlefield(&engine, p0, swamp()).unwrap();
    assert!(keywords(&engine, drinker).contains(KeywordSet::CANT_BLOCK));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drinker, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert_eq!(engine.state().players[1].life, 15);
    assert!(in_graveyard(&engine, p0, swamp()).is_some());
    assert!(on_battlefield(&engine, p0, drinker_card).is_some());
}
