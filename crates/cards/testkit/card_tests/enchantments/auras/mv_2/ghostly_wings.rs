//! `cards/enchantments/auras/mv_2/ghostly_wings.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn ghostly_wings() -> CardIndex {
    card_index("b07d8095-c5e2-4bf8-b055-af97ce5df4a4")
}

fn control_magic() -> CardIndex {
    card_index("cd0d7141-46d2-4aa3-bc77-6b3b4513803e")
}

/// Enchanted creature gets +1/+1 and flying, and discarding a card bounces it
/// to its owner's hand even if another player currently controls it.
#[test]
fn ghostly_wings_grants_plus_one_flying_and_bounces_to_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), plains()],
        &[grizzly_bears()],
        &[ghostly_wings(), forest(), forest()],
        &[control_magic()],
    );
    let my_bear = lk::obj(&e, p0, grizzly_bears());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::Blue, 2)]);
    lk::cast_at(&mut e, p0, ghostly_wings(), &[my_bear], &[]);
    assert_eq!(pt(&e, my_bear), (3, 3));
    assert!(keywords(&e, my_bear).contains(KeywordSet::FLYING));
    assert_eq!(pt(&e, opp_bear), (2, 2));

    let blocks = lk::attack(&mut e, &[my_bear]);
    assert!(!lk::may_block(&blocks, opp_bear, my_bear));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, p1), 17);

    // P1 steals my Bear with Control Magic, then P0 activates Ghostly Wings to bounce it.
    reach_their_main_phase(&mut e, p1);
    lk::float(&mut e, p1, &[(ManaColor::Blue, 4)]);
    lk::cast_at(&mut e, p1, control_magic(), &[my_bear], &[]);
    assert_eq!(e.state().object(my_bear).unwrap().controller, p1);

    reach_their_main_phase(&mut e, p0);
    let discard = in_hand(&e, p0, forest()).unwrap();
    activate(&mut e, p0, ghostly_wings(), 3);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![discard],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    assert!(in_hand(&e, p0, grizzly_bears()).is_some());
    assert!(in_graveyard(&e, p0, ghostly_wings()).is_some());
}
