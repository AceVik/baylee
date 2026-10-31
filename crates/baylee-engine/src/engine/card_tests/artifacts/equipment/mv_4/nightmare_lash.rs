//! Nightmare Lash charges life to equip and tracks the Swamp count.

use super::*;

fn nightmare_lash() -> CardIndex {
    card_index("414f1c42-6b0e-49ce-8b44-17a75b996b68")
}

#[test]
fn nightmare_lash_pays_three_life_and_pumps_per_swamp() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[nightmare_lash(), gray_ogre(), swamp(), swamp(), swamp()],
        )
        .hand(0, &[swamp()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ogre = on_battlefield(&e, p0, gray_ogre()).unwrap();
    activate(&mut e, p0, nightmare_lash(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 17);
    assert_eq!(pt(&e, ogre), (5, 5));
    play_land(&mut e, p0, swamp());
    assert_eq!(pt(&e, ogre), (6, 6));
}
