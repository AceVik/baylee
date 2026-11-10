//! `cards/creatures/mv_5/gerrard_s_irregulars.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Gerrard's Irregulars` is a 4/2 Human Soldier under `Coverage::Implemented` with trample and haste.
/// When cast from hand off five Mountains, it arrives on the battlefield with its printed 4/2 body
/// and both keywords. In the combat phase of the same turn, haste enables it to attack immediately,
/// dealing 4 combat damage to the defending player when unblocked.
#[test]
fn gerrard_s_irregulars_haste_and_trample_attacks_on_arrival() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[gerrard_s_irregulars()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, gerrard_s_irregulars());
    pass_until(&mut engine, stack_is_empty);

    let troops = on_battlefield(&engine, p0, gerrard_s_irregulars())
        .expect("Gerrard's Irregulars resolved onto the battlefield");
    assert_eq!(pt(&engine, troops), (4, 2), "printed body is 4/2");
    let kw = keywords(&engine, troops);
    assert!(
        kw.contains(KeywordSet::HASTE),
        "Gerrard's Irregulars has haste"
    );
    assert!(
        kw.contains(KeywordSet::TRAMPLE),
        "Gerrard's Irregulars has trample"
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
        on_battlefield(&engine, p0, gerrard_s_irregulars()).is_some(),
        "Gerrard's Irregulars survives combat"
    );
}
