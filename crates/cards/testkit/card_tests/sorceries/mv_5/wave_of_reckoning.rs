//! `cards/sorceries/mv_5/wave_of_reckoning.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn wave_of_reckoning() -> CardIndex {
    card_index("24f7143c-d43a-4120-baa0-0235b15d8186")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn gray_ogre() -> CardIndex {
    card_index("83c8a3a6-2e1a-4e26-8847-6d066f42d906")
}

fn ironroot_treefolk() -> CardIndex {
    card_index("b7c0bb85-fb87-4c73-bc1b-7b4dc763c7e8")
}

fn wall_of_stone() -> CardIndex {
    card_index("cd4cadb4-3156-49bd-b36e-12ba5c85938b")
}

#[test]
fn wave_of_reckoning_has_each_creature_deal_power_damage_to_itself() {
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                gray_ogre(),
                wall_of_stone(),
            ],
        )
        .battlefield(1, &[ironroot_treefolk()])
        .hand(0, &[wave_of_reckoning()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    let wall = on_battlefield(&e, P0, wall_of_stone()).unwrap();
    let treefolk = on_battlefield(&e, P1, ironroot_treefolk()).unwrap();

    cast_from_hand(&mut e, P0, wave_of_reckoning());
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, P0, gray_ogre()).is_none(),
        "2/2 dies to 2"
    );
    assert_eq!(
        e.state().object(wall).unwrap().damage,
        0,
        "0/8 Wall takes 0"
    );
    assert_eq!(
        e.state().object(treefolk).unwrap().damage,
        3,
        "3/5 Treefolk takes 3"
    );
}
