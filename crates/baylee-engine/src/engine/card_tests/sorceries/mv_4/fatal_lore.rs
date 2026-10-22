//! `cards/sorceries/mv_4/fatal_lore.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fatal_lore() -> CardIndex {
    card_index("aeb85786-16fc-45d8-9383-60054a878c40")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn grizzly_bears() -> CardIndex {
    baylee_core::generated::index::GRIZZLY_BEARS
}

#[test]
fn fatal_lore_mode_one_draws_three_cards() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[fatal_lore()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, fatal_lore());
    engine
        .apply(p0, PlayerAction::ChooseMode(0))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1 + 3,
        "Fatal Lore mode 1 should draw three cards"
    );
}

#[test]
fn fatal_lore_mode_two_destroys_up_to_two_opponent_creatures_and_opponent_draws_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .hand(0, &[fatal_lore()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let p1_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, fatal_lore());
    engine
        .apply(p0, PlayerAction::ChooseMode(1))
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears, giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_none());
    assert!(on_battlefield(&engine, p1, hill_giant()).is_none());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        p1_hand_before + 3,
        "Fatal Lore mode 2 should destroy both target creatures and have the opponent draw three cards"
    );
}
