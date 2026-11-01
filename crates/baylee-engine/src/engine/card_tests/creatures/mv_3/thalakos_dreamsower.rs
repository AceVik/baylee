//! `cards/creatures/mv_3/thalakos_dreamsower.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn thalakos_dreamsower() -> CardIndex {
    card_index("5f71c09a-47fb-4bb2-9329-78f5bb195e5d")
}

/// `Thalakos Dreamsower` ({2}{U}, 1/1 Creature — Thalakos Wizard,
/// `Coverage::Implemented`):
/// `Shadow`
/// `You may choose not to untap this creature during your untap step.`
/// `Whenever this creature deals damage to an opponent, tap target creature.
/// That creature doesn't untap during its controller's untap step for as long
/// as this creature remains tapped.`
#[test]
fn thalakos_dreamsower_taps_target_creature_on_damage_and_keeps_it_tapped_while_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[thalakos_dreamsower()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let sower = on_battlefield(&engine, p0, thalakos_dreamsower()).unwrap();
    let bears = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    assert!(
        engine
            .state()
            .object(sower)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::SHADOW)
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(sower, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(is_tapped(&engine, bears));

    // On p1's next turn, Grizzly Bears does not untap while Thalakos Dreamsower remains tapped.
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, bears));
}

#[test]
fn choosing_to_untap_the_source_releases_the_creature() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(SEED, &[thalakos_dreamsower()], &[GRIZZLY_BEARS], &[], &[]);
    let source = on_battlefield(&e, p0, thalakos_dreamsower()).unwrap();
    let bears = on_battlefield(&e, p1, GRIZZLY_BEARS).unwrap();
    lk::attack(&mut e, &[source]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[bears], &[]);
    lk::through_combat(&mut e);
    reach_their_main_phase(&mut e, p1);
    assert!(is_tapped(&e, bears));
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    // Choosing no permanents to keep tapped lets the source untap.
    e.apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    reach_their_main_phase(&mut e, p0);
    assert!(!is_tapped(&e, source));
    reach_their_main_phase(&mut e, p1);
    assert!(!is_tapped(&e, bears));
}
