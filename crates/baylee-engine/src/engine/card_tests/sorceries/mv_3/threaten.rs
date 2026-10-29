//! `cards/sorceries/mv_3/threaten.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Threaten (`3dc8d15a-3ca7-4d1f-a819-c1b7f947a59e`) — {2}{R} Sorcery:
/// "Untap target creature and gain control of it until end of turn. That creature gains haste until end of turn."
#[test]
fn threaten_untaps_gains_control_and_grants_haste_until_end_of_turn() {
    let threaten_card = card_index("3dc8d15a-3ca7-4d1f-a819-c1b7f947a59e");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[threaten_card])
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, threaten_card);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(giant).unwrap().controller, p0);
    assert!(keywords(&engine, giant).contains(KeywordSet::HASTE));

    // Attack p1 with the stolen Hill Giant, then verify control returns at end of turn.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[1].life, 17);
    assert_eq!(engine.state().object(giant).unwrap().controller, p1);
}
