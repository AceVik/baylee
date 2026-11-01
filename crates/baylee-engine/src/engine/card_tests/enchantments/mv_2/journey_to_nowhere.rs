//! `cards/enchantments/mv_2/journey_to_nowhere.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn journey_to_nowhere() -> CardIndex {
    card_index("2bfec482-be0d-4768-84e7-1c2c74ea1373")
}

/// The arrival trigger exiles its target; destroying Journey then plays the
/// linked leaves trigger and returns that card under its owner's control.
#[test]
fn journey_exiles_a_creature_and_returns_it_to_its_owner_when_it_leaves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[journey_to_nowhere()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let ogre = on_battlefield(&e, p1, gray_ogre()).expect("their creature");
    cast_from_hand(&mut e, p0, journey_to_nowhere());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut e, p0, ogre);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(ogre).expect("exiled Ogre").zone,
        Zone::Exile
    );

    let journey = on_battlefield(&e, p0, journey_to_nowhere()).expect("the enchantment");
    kill(&mut e, journey);
    assert!(in_graveyard(&e, p0, journey_to_nowhere()).is_some());
    let returned = on_battlefield(&e, p1, gray_ogre()).expect("returned under its owner");
    assert_eq!(returned, ogre, "the linked card is the one that returns");
}
