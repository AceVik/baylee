//! `cards/creatures/mv_5/renegade_troops.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Renegade Troops` is a 4/2 creature under `Coverage::Implemented` with haste and no activated abilities.
/// When cast from hand off five Mountains, it resolves onto the battlefield with its printed 4/2 stats and haste.
/// In the combat phase of the same turn, haste allows it to attack immediately and deal 4 combat damage to the opponent.
#[test]
fn renegade_troops_has_haste_and_attacks_the_turn_it_enters() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[renegade_troops()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, renegade_troops());
    pass_until(&mut engine, stack_is_empty);

    let troops = on_battlefield(&engine, p0, renegade_troops()).expect("Renegade Troops resolved");
    assert_eq!(pt(&engine, troops), (4, 2), "printed body is 4/2");
    assert!(
        keywords(&engine, troops).contains(KeywordSet::HASTE),
        "Renegade Troops has haste"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });

    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "active seat declares attackers");
    assert!(
        attackers.contains(&troops),
        "haste allows attacking on the turn of arrival: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(troops, Defender::Player(p1))],
            },
        )
        .expect("declaring attack is legal");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers {
        player: blocker_player,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseBlockers prompt, got {:?}", engine.pending());
    };
    engine
        .apply(
            blocker_player,
            PlayerAction::DeclareBlockers { blockers: vec![] },
        )
        .expect("declaring no blockers is legal");

    pass_until(&mut engine, |e| e.state().players[1].life != 20);

    assert_eq!(
        engine.state().players[1].life,
        16,
        "four combat damage dealt to defending player"
    );
    assert!(
        on_battlefield(&engine, p0, renegade_troops()).is_some(),
        "Renegade Troops survives combat"
    );
}
