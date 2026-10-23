//! `cards/enchantments/mv_2/oath_of_ghouls.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn oath_of_ghouls() -> CardIndex {
    card_index("b9490b52-6c3f-437b-b2c2-9e81c4b43b61")
}

#[test]
fn oath_of_ghouls_returns_creature_card_from_graveyard_to_hand_on_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, gray_ogre())
        .battlefield(0, &[oath_of_ghouls()])
        .start();
    keep_mulligans(&mut engine);
    seed_graveyard(&mut engine, p0, 1);

    let dead_bears = in_graveyard(&engine, p0, gray_ogre()).unwrap();
    kit::drive(
        &mut engine,
        |e| in_hand(e, p0, gray_ogre()).is_some() && stack_is_empty(e),
        |_, p| {
            if matches!(p, Pending::YesNo { .. }) {
                Some(PlayerAction::YesNo(true))
            } else {
                kit::choose_targets(p, &[dead_bears], &[])
            }
        },
    );

    assert!(in_hand(&engine, p0, gray_ogre()).is_some());
}
