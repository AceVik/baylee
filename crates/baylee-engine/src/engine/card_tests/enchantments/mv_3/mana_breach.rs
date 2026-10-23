//! `cards/enchantments/mv_3/mana_breach.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

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

#[test]
fn mana_breach_asks_an_opponent_to_return_their_own_land_when_they_cast() {
    use crate::engine::card_tests::legends_kit as lk;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[mana_breach(), forest()],
        &[mountain(), index::GRIZZLY_BEARS],
        &[],
        &[lightning_bolt()],
    );
    reach_their_main_phase(&mut e, p1);
    let land = lk::obj(&e, p1, mountain());
    let mine = lk::obj(&e, p0, forest());
    let creature = lk::obj(&e, p1, index::GRIZZLY_BEARS);
    cast_from_hand(&mut e, p1, lightning_bolt());
    kit::settle(&mut e, p1, |_, p| {
        if let Pending::ChooseCards {
            player, options, ..
        } = p
        {
            assert_eq!(*player, p1);
            assert!(options.contains(&land));
            assert!(!options.contains(&mine));
            assert!(!options.contains(&creature));
        }
        kit::choose_targets(p, &[], &[p0]).or_else(|| kit::choose_cards(p, &[land]))
    });
    assert!(in_hand(&e, p1, mountain()).is_some());
    assert!(on_battlefield(&e, p0, forest()).is_some());
}
