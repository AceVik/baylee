//! `cards/creatures/mv_3/cephalid_pathmage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Cephalid Pathmage (`c5c5f3f4-42e3-4d60-b6b1-8411ac51ce4e`) — {2}{U} Creature — Octopus Wizard 1/2:
/// "This creature can't be blocked.
/// {T}, Sacrifice this creature: Target creature can't be blocked this turn."
#[test]
fn cephalid_pathmage_is_unblockable_and_grants_unblockable_on_sacrifice() {
    let pathmage_card = card_index("c5c5f3f4-42e3-4d60-b6b1-8411ac51ce4e");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[pathmage_card, grizzly_bears()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pathmage = on_battlefield(&engine, p0, pathmage_card).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    assert!(keywords(&engine, pathmage).contains(KeywordSet::UNBLOCKABLE));
    assert!(!keywords(&engine, bear).contains(KeywordSet::UNBLOCKABLE));

    activate(&mut engine, p0, pathmage_card, 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, pathmage_card).is_some());
    assert!(keywords(&engine, bear).contains(KeywordSet::UNBLOCKABLE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18);
}
