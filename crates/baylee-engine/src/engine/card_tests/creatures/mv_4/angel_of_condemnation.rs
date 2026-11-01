//! `cards/creatures/mv_4/angel_of_condemnation.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn angel_of_condemnation() -> CardIndex {
    card_index("2317a33f-665e-4cbc-bb5c-16b912ac2eef")
}

fn condemnation_table() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[angel_of_condemnation(), plains(), plains(), plains()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let angel = on_battlefield(&e, p0, angel_of_condemnation()).expect("the Angel");
    let ogre = on_battlefield(&e, p1, gray_ogre()).expect("another creature");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    assert!(keywords(&e, angel).contains(KeywordSet::VIGILANCE));
    (e, angel, ogre)
}

#[test]
fn angel_of_condemnation_flickers_another_creature_at_the_next_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let (mut e, angel, ogre) = condemnation_table();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, angel_of_condemnation(), 0);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("expected another-creature target, got {:?}", e.pending())
    };
    assert!(options.contains(&ogre));
    assert!(!options.contains(&angel));
    aim_at(&mut e, p0, ogre);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(ogre).expect("exiled Ogre").zone,
        Zone::Exile
    );
    pass_until(&mut e, |e| {
        e.state()
            .object(ogre)
            .is_some_and(|o| o.zone == Zone::Battlefield)
    });
    assert_eq!(
        e.state().object(ogre).expect("returned Ogre").controller,
        p1
    );
}

#[test]
fn angel_of_condemnation_exert_exiles_until_it_leaves_and_skips_one_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut e, angel, ogre) = condemnation_table();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, angel_of_condemnation(), 1);
    aim_at(&mut e, p0, ogre);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, angel));
    assert_eq!(
        e.state().object(ogre).expect("exiled Ogre").zone,
        Zone::Exile
    );

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert!(is_tapped(&e, angel), "exert skips the next untap");
    assert_eq!(
        e.state().object(ogre).expect("still exiled").zone,
        Zone::Exile
    );
    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert!(!is_tapped(&e, angel), "the following untap is normal");
    kill(&mut e, angel);
    assert_eq!(on_battlefield(&e, p1, gray_ogre()), Some(ogre));
}
