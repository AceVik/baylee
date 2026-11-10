//! Nim Shambler's artifact pump and sacrifice-funded regeneration.

use super::*;

fn nim_shambler() -> CardIndex {
    card_index("83d2ff7f-4d6e-4cc0-b8a9-3c5b3236eea0")
}

#[test]
fn nim_shambler_pumps_and_regenerates_after_sacrificing_a_creature() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[nim_shambler(), gray_ogre(), quiet_artifact(), mountain()],
        )
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let shambler = on_battlefield(&e, p0, nim_shambler()).unwrap();
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    assert_eq!(pt(&e, shambler), (3, 1));
    activate(&mut e, p0, nim_shambler(), 1);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p0, gray_ogre()).is_some());

    cast_from_hand(&mut e, p0, lightning_bolt());
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![shambler],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, nim_shambler()).is_some());
    assert!(is_tapped(&e, shambler), "regeneration taps the survivor");
}
