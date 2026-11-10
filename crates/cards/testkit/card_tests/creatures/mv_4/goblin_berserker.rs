//! `cards/creatures/mv_4/goblin_berserker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Berserker is a 2/2 Goblin Berserker with first strike and haste under `Coverage::Implemented` costing {3}{R}.
/// Its printed keywords reach the permanent through the continuous layer system.
/// Because it has haste, it can attack immediately during the combat phase of the turn it was cast under `CR 302.6`.
/// An unblocked attack deals its 2 damage directly to the defending player's life total.
#[test]
fn goblin_berserker_has_first_strike_and_attacks_immediately_with_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[goblin_berserker()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, goblin_berserker());
    pass_until(&mut engine, stack_is_empty);

    let goblin =
        on_battlefield(&engine, p0, goblin_berserker()).expect("Goblin Berserker resolved");
    assert_eq!(pt(&engine, goblin), (2, 2), "printed body is 2/2");
    let kw = keywords(&engine, goblin);
    assert!(
        kw.contains(KeywordSet::FIRST_STRIKE),
        "Goblin Berserker has first strike"
    );
    assert!(kw.contains(KeywordSet::HASTE), "Goblin Berserker has haste");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on choose-attackers")
    };
    assert_eq!(player, p0, "active seat declares attackers");
    assert!(
        attackers.contains(&goblin),
        "CR 302.6: creature with haste is offered to attack on the turn it arrived: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(goblin, Defender::Player(p1))],
            },
        )
        .expect("attack declared");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player: blocker_player,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected block choice, got {:?}", engine.pending())
    };
    engine
        .apply(
            blocker_player,
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .unwrap();

    pass_until(&mut engine, |e| e.state().players[1].life != 20);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "two combat damage dealt to defending seat"
    );
    assert!(
        on_battlefield(&engine, p0, goblin_berserker()).is_some(),
        "Goblin Berserker survives combat"
    );
}
