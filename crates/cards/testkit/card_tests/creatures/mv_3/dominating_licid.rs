//! `exo/dominating_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::choice::GrantedActionKind;

fn dominating_licid() -> CardIndex {
    card_index("ae2804c6-fb38-4845-ae21-7f41a766cbba")
}

#[test]
fn dominating_licid_steals_target_creature_and_returns_control_on_revert() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, island())
        .battlefield(0, &[dominating_licid()])
        .battlefield(1, &[baylee_core::generated::index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let licid = on_battlefield(&e, p0, dominating_licid()).expect("Dominating Licid out");
    let opp_bear = on_battlefield(&e, p1, baylee_core::generated::index::GRIZZLY_BEARS)
        .expect("opponent Bear out");
    assert_eq!(e.state().object(opp_bear).unwrap().controller, p1);

    // Transform Dominating Licid into Aura enchanting opponent's Bear -> p0 gains control!
    lk::float(&mut e, p0, &[(ManaColor::Blue, 3)]);
    activate(&mut e, p0, dominating_licid(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![opp_bear],
            players: vec![],
        },
    )
    .expect("target opponent Bear");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(licid).unwrap().attached_to, Some(opp_bear));
    assert_eq!(
        e.state().object(opp_bear).unwrap().controller,
        p0,
        "Dominating Licid grants control of enchanted creature"
    );

    // Pay {U} special action to end the effect -> unattaches and control returns to p1.
    lk::float(&mut e, p0, &[(ManaColor::Blue, 1)]);
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority");
    };
    let revert = legal
        .granted_actions
        .iter()
        .find(|g| matches!(g.effect, GrantedActionKind::EndContinuousEffects { target } if target.object == licid))
        .expect("pay {U} to end this effect offered")
        .id;
    e.apply(p0, PlayerAction::TakeGrantedAction { id: revert })
        .expect("end Dominating Licid effect");

    assert_eq!(e.state().object(licid).unwrap().attached_to, None);
    assert!(
        e.state()
            .object(licid)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert_eq!(
        e.state().object(opp_bear).unwrap().controller,
        p1,
        "Control of Bear returns to p1 once Dominating Licid reverts"
    );
}
