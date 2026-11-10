//! `cards/creatures/mv_7/angel_of_serenity.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn angel_of_serenity() -> CardIndex {
    card_index("f4ce6078-8c7b-4f68-b324-13130c63a983")
}

#[test]
fn angel_of_serenity_exiles_mixed_creature_targets_and_returns_owners_hands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[angel_of_serenity(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let own_graveyard = hand_to_graveyard(&mut e, p0, llanowar_elves());
    let their_graveyard = hand_to_graveyard(&mut e, p1, gray_ogre());
    let their_battlefield = on_battlefield(&e, p1, gray_ogre()).expect("their creature");
    cast_from_hand(&mut e, p0, angel_of_serenity());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!("the arrival trigger targets")
    };
    assert_eq!((min, max), (0, 3));
    for id in [own_graveyard, their_graveyard, their_battlefield] {
        assert!(options.contains(&id), "mixed target {id:?} is offered");
    }
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![their_battlefield, own_graveyard, their_graveyard],
        },
    )
    .expect("battlefield and either graveyard are one target pool");
    pass_until(&mut e, stack_is_empty);
    for id in [own_graveyard, their_graveyard, their_battlefield] {
        assert_eq!(
            e.state().object(id).expect("exiled target").zone,
            Zone::Exile
        );
    }
    let angel = on_battlefield(&e, p0, angel_of_serenity()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    kill(&mut e, angel);
    for (id, owner) in [
        (own_graveyard, p0),
        (their_graveyard, p1),
        (their_battlefield, p1),
    ] {
        let object = e.state().object(id).expect("returned target");
        assert_eq!(object.zone, Zone::Hand);
        assert_eq!(object.owner, owner);
        assert!(
            e.state()
                .zones
                .list(ZoneLocation::Hand(owner))
                .contains(&id)
        );
    }
}

/// "You may exile": the controller can decline as the trigger resolves, with
/// its targets already chosen. Without the "may" the targets were exiled.
#[test]
fn angel_of_serenity_may_decline_its_exile_on_resolution() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 7])
        .hand(0, &[angel_of_serenity()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ogre = on_battlefield(&e, p1, gray_ogre()).expect("their creature");
    cast_from_hand(&mut e, p0, angel_of_serenity());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .expect("one target of up to three");
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    assert!(
        matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
        "the exile is optional: {:?}",
        e.pending()
    );
    e.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(on_battlefield(&e, p1, gray_ogre()), Some(ogre));
}
