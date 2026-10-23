//! `cards/enchantments/mv_3/mana_breach.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn mana_breach() -> CardIndex {
    card_index("221d0305-94db-4791-9dc7-89ee96c7fcd6")
}

#[test]
fn mana_breach_returns_a_land_to_hand_when_a_player_casts_a_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(0, &[mana_breach(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let land_id = on_battlefield(&engine, p0, mountain()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    kit::settle(&mut engine, p0, |_, p| {
        kit::choose_targets(p, &[], &[p0]).or_else(|| kit::choose_cards(p, &[land_id]))
    });

    assert!(on_battlefield(&engine, p0, mountain()).is_none());
    assert!(in_hand(&engine, p0, mountain()).is_some());
}
