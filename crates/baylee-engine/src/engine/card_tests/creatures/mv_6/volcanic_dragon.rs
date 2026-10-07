//! `cards/creatures/mv_6/volcanic_dragon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Volcanic Dragon` is a 4/4 Dragon costing `{4}{R}{R}` under `Coverage::Implemented` with flying and haste.
/// When cast from hand off six Mountains, it resolves onto the battlefield with its printed 4/4 body,
/// flying, and haste. Haste allows it to attack immediately in the same turn's combat phase, dealing 4 combat
/// damage to the opponent when unblocked.
#[test]
fn volcanic_dragon_has_flying_and_haste_attacks_on_arrival() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[volcanic_dragon()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, volcanic_dragon());
    pass_until(&mut engine, stack_is_empty);

    let dragon = on_battlefield(&engine, p0, volcanic_dragon()).expect("Volcanic Dragon resolved");
    assert_eq!(pt(&engine, dragon), (4, 4), "printed body is 4/4");
    let kw = keywords(&engine, dragon);
    assert!(
        kw.contains(KeywordSet::FLYING),
        "Volcanic Dragon has flying"
    );
    assert!(kw.contains(KeywordSet::HASTE), "Volcanic Dragon has haste");

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
        attackers.contains(&dragon),
        "haste allows attacking on turn of arrival: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(p1))],
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
        "four combat damage dealt to opponent"
    );
    assert!(
        on_battlefield(&engine, p0, volcanic_dragon()).is_some(),
        "Volcanic Dragon survives combat"
    );
}
