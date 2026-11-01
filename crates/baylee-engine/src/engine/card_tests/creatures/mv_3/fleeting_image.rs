#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn fleeting_image() -> CardIndex {
    card_index("623f3efb-5431-42d4-98aa-9caac40e4e83")
}

#[test]
fn fleeting_image_bounces_itself_to_owners_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[fleeting_image()])
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, fleeting_image());
    pass_until(&mut engine, stack_is_empty);

    let img = on_battlefield(&engine, p0, fleeting_image()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: img,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, fleeting_image()).is_none());
    assert!(in_hand(&engine, p0, fleeting_image()).is_some());
}

#[test]
fn fleeting_image_flying_excludes_a_ground_blocker() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[fleeting_image()],
        &[gray_ogre(), shivan_dragon()],
        &[],
        &[],
    );
    let image = lk::obj(&engine, p0, fleeting_image());
    let ogre = lk::obj(&engine, p1, gray_ogre());
    let dragon = lk::obj(&engine, p1, shivan_dragon());
    let blocks = lk::attack(&mut engine, &[image]);
    assert!(!lk::may_block(&blocks, ogre, image));
    assert!(lk::may_block(&blocks, dragon, image));
}
