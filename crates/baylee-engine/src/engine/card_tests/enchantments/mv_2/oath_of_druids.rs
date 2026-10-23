//! `cards/enchantments/mv_2/oath_of_druids.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn oath_of_druids() -> CardIndex {
    card_index("cf0dcb27-f02e-4065-81dc-75fbbb09ea3f")
}

#[test]
fn oath_of_druids_reveals_until_creature_and_puts_onto_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(1, gray_ogre())
        .battlefield(0, &[oath_of_druids()])
        .start();
    keep_mulligans(&mut engine);

    kit::drive(
        &mut engine,
        |e| on_battlefield(e, p0, gray_ogre()).is_some() && stack_is_empty(e),
        |_, p| {
            if matches!(p, Pending::YesNo { .. }) {
                Some(PlayerAction::YesNo(true))
            } else {
                kit::choose_targets(p, &[], &[p1])
            }
        },
    );

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
