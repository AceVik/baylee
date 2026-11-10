//! Windscouter — {3}{U} — Creature — Human Scout
//! Oracle: Flying
//! Oracle: When this creature attacks or blocks, return it to its owner's hand at end of combat.

use super::*;

#[test]
fn windscouter_returns_to_hand_at_end_of_combat_after_attacking() {
    let windscouter = card_index("58232c49-f880-41ef-b808-62a6e804c715");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[windscouter])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, windscouter);
    pass_until(&mut engine, stack_is_empty);
    assert!(walk_to_own_main(&mut engine, p1));
    assert!(walk_to_own_main(&mut engine, p0));

    let scouter_id = on_battlefield(&engine, p0, windscouter).expect("Windscouter on board");
    assert!(keywords(&engine, scouter_id).contains(KeywordSet::FLYING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(scouter_id, Defender::Player(p1))],
            },
        )
        .expect("declare Windscouter as attacker");

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 17);
    assert!(on_battlefield(&engine, p0, windscouter).is_none());
    assert!(in_hand(&engine, p0, windscouter).is_some());
}

#[test]
fn windscouter_returns_to_hand_at_end_of_combat_after_blocking() {
    let windscouter = card_index("58232c49-f880-41ef-b808-62a6e804c715");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(2, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[windscouter])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, windscouter);
    pass_until(&mut engine, stack_is_empty);
    assert!(walk_to_own_main(&mut engine, p1));

    let scouter_id = on_battlefield(&engine, p0, windscouter).expect("Windscouter on board");
    let bears_id = on_battlefield(&engine, p1, grizzly_bears()).expect("Bears on board");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears_id, Defender::Player(p0))],
            },
        )
        .expect("declare Bears as attacker");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(scouter_id, bears_id)],
            },
        )
        .expect("declare Windscouter as blocker");

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p0, windscouter).is_none());
    assert!(in_hand(&engine, p0, windscouter).is_some());
}
