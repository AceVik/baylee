//! `cards/creatures/mv_4/yavimaya_ants.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const YAVIMAYA_ANTS: &str = "3f5bbfaa-cbd7-47af-9aa2-abb49d672734";

#[test]
fn yavimaya_ants_attacks_with_haste_and_requires_cumulative_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index(YAVIMAYA_ANTS);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    let ants = on_battlefield(&engine, p0, card).unwrap();

    // Haste lets Yavimaya Ants attack immediately for 5 combat damage.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ants, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 15);

    // On p0's next upkeep, decline cumulative upkeep {G}{G} -> Yavimaya Ants is sacrificed.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, card).is_none());
    assert!(in_graveyard(&engine, p0, card).is_some());
}
