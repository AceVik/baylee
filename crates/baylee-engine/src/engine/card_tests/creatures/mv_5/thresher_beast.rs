//! Thresher Beast — {3}{G}{G} — Creature — Beast
//! Oracle: Whenever this creature becomes blocked, defending player sacrifices a land of their choice.

use super::*;

#[test]
fn thresher_beast_forces_defending_player_to_sacrifice_a_land_when_blocked() {
    let thresher_beast = card_index("e88915fd-7867-45e6-b4b9-ca0af1823c0d");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[thresher_beast])
        .battlefield(1, &[grizzly_bears(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, thresher_beast);
    pass_until(&mut engine, stack_is_empty);
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));

    let beast_id = on_battlefield(&engine, p0, thresher_beast).expect("Thresher Beast on board");
    let bears_id = on_battlefield(&engine, p1, grizzly_bears()).expect("Bears on board");
    let plains_id = on_battlefield(&engine, p1, plains()).expect("Plains on board");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(beast_id, Defender::Player(p1))],
            },
        )
        .expect("declare Thresher Beast as attacker");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears_id, beast_id)],
            },
        )
        .expect("declare Bears as blocker");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![plains_id],
            },
        )
        .expect("defending player chooses Plains to sacrifice");

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, p1, plains()).is_some());
    assert!(on_battlefield(&engine, p1, island()).is_some());
}
