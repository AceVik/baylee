//! `tmp/quickening_licid.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_engine::choice::GrantedActionKind;

fn quickening_licid() -> CardIndex {
    card_index("2bdd9ca3-542a-43c0-9831-c6add23890d6")
}

#[test]
fn quickening_licid_transforms_into_aura_grants_first_strike_and_reverts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                quickening_licid(),
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let licid =
        on_battlefield(&e, p0, quickening_licid()).expect("Quickening Licid on battlefield");
    let bear = on_battlefield(&e, p0, baylee_core::generated::index::GRIZZLY_BEARS)
        .expect("Bear on battlefield");

    // Transform Quickening Licid into an Aura attached to Grizzly Bears.
    lk::float(&mut e, p0, &[(ManaColor::White, 2)]);
    activate(&mut e, p0, quickening_licid(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .expect("target Bear");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(licid).unwrap().attached_to, Some(bear));
    assert!(
        !e.state()
            .object(licid)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert!(
        e.state()
            .object(licid)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::ENCHANTMENT)
    );
    assert!(
        keywords(&e, bear).contains(KeywordSet::FIRST_STRIKE),
        "Enchanted creature gains first strike"
    );

    // While an Aura, its transformation ability is lost and paying {W} ends the effect.
    lk::float(&mut e, p0, &[(ManaColor::White, 1)]);
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority");
    };
    assert!(!legal.abilities.contains(&(licid, 0)));
    let revert = legal
        .granted_actions
        .iter()
        .find(|g| matches!(g.effect, GrantedActionKind::EndContinuousEffects { target } if target.object == licid))
        .expect("pay {W} to end this effect offered")
        .id;
    e.apply(p0, PlayerAction::TakeGrantedAction { id: revert })
        .expect("end Licid effect");

    assert_eq!(e.state().object(licid).unwrap().zone, Zone::Battlefield);
    assert_eq!(e.state().object(licid).unwrap().attached_to, None);
    assert!(
        e.state()
            .object(licid)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
    assert!(!keywords(&e, bear).contains(KeywordSet::FIRST_STRIKE));
    let _ = p1;
}

#[test]
fn quickening_licid_goes_to_graveyard_when_enchanted_creature_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                quickening_licid(),
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let licid =
        on_battlefield(&e, p0, quickening_licid()).expect("Quickening Licid on battlefield");
    let bear = on_battlefield(&e, p0, baylee_core::generated::index::GRIZZLY_BEARS)
        .expect("Bear on battlefield");

    lk::float(&mut e, p0, &[(ManaColor::White, 2)]);
    activate(&mut e, p0, quickening_licid(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .expect("target Bear");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(licid).unwrap().attached_to, Some(bear));

    // Kill the enchanted Bear: Quickening Licid is an unattached Aura -> CR 704.5m bins it
    // and its EndContinuousEffects special action is pruned.
    lk::float(&mut e, p0, &[(ManaColor::Red, 1), (ManaColor::White, 1)]);
    cast_front_face(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![bear],
            players: vec![],
        },
    )
    .expect("bolt Bear");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        e.state().object(licid).unwrap().zone,
        Zone::Graveyard,
        "Unattached Licid-Aura goes to graveyard via CR 704.5m"
    );
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("expected priority");
    };
    assert!(
        !legal.granted_actions.iter().any(|g| matches!(
            g.effect,
            GrantedActionKind::EndContinuousEffects { target } if target.object == licid
        )),
        "Special action must be pruned once Licid leaves battlefield"
    );
    let _ = p1;
}
