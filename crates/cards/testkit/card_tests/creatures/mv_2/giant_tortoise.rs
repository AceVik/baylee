//! `cards/creatures/mv_2/giant_tortoise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Giant Tortoise` prints a static ability granting +0/+3 as long as it is untapped under `Coverage::Implemented`.
/// While standing untapped before combat, its projected `pt` is (1, 4).
/// Upon declaring an attack in combat, the Tortoise taps, losing the conditional toughness bonus
/// and reverting to its printed `pt` of (1, 1).
#[test]
fn giant_tortoise_gets_bonus_only_while_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1624, forest())
        .battlefield(0, &[giant_tortoise()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tortoise = on_battlefield(&engine, p0, giant_tortoise()).expect("tortoise is seated");
    assert!(!is_tapped(&engine, tortoise));
    assert_eq!(
        pt(&engine, tortoise),
        (1, 4),
        "untapped tortoise gets +0/+3"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { defenders, .. } = engine.pending().clone() else {
        panic!("expected ChooseAttackers prompt");
    };
    let defender = defenders.into_iter().next().expect("opponent is defender");
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tortoise, defender)],
            },
        )
        .unwrap();

    assert!(
        is_tapped(&engine, tortoise),
        "attacking tapped the tortoise"
    );
    assert_eq!(
        pt(&engine, tortoise),
        (1, 1),
        "tapped tortoise loses +0/+3 bonus"
    );
}

/// The tap that loses the bonus is part of the declaration (CR 508.1f), so
/// it is gone while the band question the declaration raises stands
/// (CR 508.1e): beside a Benalish Hero the Tortoise read 1/4 in that
/// question and 1/1 only once it was answered, and a player choosing the
/// band was shown a board the attack had already changed (CR 611.3a).
#[test]
fn giant_tortoise_has_lost_its_bonus_while_the_band_is_asked() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1624, forest())
        .battlefield(0, &[giant_tortoise(), benalish_hero()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tortoise = on_battlefield(&engine, p0, giant_tortoise()).expect("tortoise is seated");
    let hero = on_battlefield(&engine, p0, benalish_hero()).expect("hero is seated");
    assert_eq!(pt(&engine, tortoise), (1, 4), "untapped, it gets +0/+3");

    declare_band_attack(&mut engine, p0, p1, &[tortoise, hero]);
    assert!(
        matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Band { .. },
                ..
            }
        ),
        "the declaration asks for the band: {:?}",
        engine.pending()
    );
    assert!(is_tapped(&engine, tortoise), "attacking tapped it");
    assert_eq!(
        pt(&engine, tortoise),
        (1, 1),
        "tapped while the band is asked, it has lost the bonus"
    );
}
