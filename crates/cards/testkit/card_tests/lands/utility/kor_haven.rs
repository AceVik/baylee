//! `cards/lands/utility/kor_haven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Kor Haven` is a legendary land under `Coverage::Implemented`.
/// It prints "{T}: Add {C}." and "{1}{W}, {T}: Prevent all combat damage that would be dealt by target attacking creature this turn."
/// When an opponent attacks with `desert_drake()`, activating `Kor Haven`'s second ability prevents
/// all combat damage from that creature, preserving the defending player's life total at 20.
#[test]
fn kor_haven_prevents_combat_damage_from_attacking_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kor_haven(), plains(), forest()])
        .battlefield(1, &[desert_drake()])
        .start();
    keep_mulligans(&mut engine);

    reach_their_main_phase(&mut engine, p1);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseAttackers prompt, got {:?}",
            engine.pending()
        );
    };
    let def = defenders.into_iter().next().expect("defender exists");
    let drake = on_battlefield(&engine, p1, desert_drake()).expect("drake deployed");
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(drake, def)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );

    tap_all_mana_but(&mut engine, p0, Some(kor_haven()));
    activate(&mut engine, p0, kor_haven(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected ChooseTargets prompt for Kor Haven, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&drake),
        "attacking creature is a legal target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![drake],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[0].life,
        20,
        "combat damage from the attacking creature was completely prevented"
    );
    let haven = on_battlefield(&engine, p0, kor_haven()).expect("Kor Haven on battlefield");
    assert!(is_tapped(&engine, haven), "Kor Haven is tapped");
}
