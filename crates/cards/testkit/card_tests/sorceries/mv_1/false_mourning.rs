//! `cards/sorceries/mv_1/false_mourning.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn false_mourning() -> CardIndex {
    card_index("31a331a0-3065-4ecb-9e6a-ab11e241833b")
}

/// False Mourning (31a331a0-3065-4ecb-9e6a-ab11e241833b) — {G} Sorcery:
/// "Put target card from your graveyard on top of your library."
#[test]
fn false_mourning_puts_target_card_from_your_graveyard_on_top_of_library() {
    let mut engine = lk::game(
        SEED,
        &[forest()],
        &[],
        &[false_mourning(), llanowar_elves(), counterspell()],
        &[aurochs()],
    );
    let my_elf = hand_to_graveyard(&mut engine, P0, llanowar_elves());
    let my_spell = hand_to_graveyard(&mut engine, P0, counterspell());
    let their_beast = hand_to_graveyard(&mut engine, P1, aurochs());

    cast_from_hand(&mut engine, P0, false_mourning());
    let menu = aim_at(&mut engine, P0, my_spell);
    assert!(menu.contains(&my_elf) && menu.contains(&my_spell));
    assert!(!menu.contains(&their_beast));
    lk::settle(&mut engine);

    assert_eq!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(P0))
            .last()
            .copied(),
        Some(my_spell)
    );
    assert_eq!(lk::zone_of(&engine, my_elf), Zone::Graveyard);
    assert!(in_graveyard(&engine, P0, false_mourning()).is_some());
}

/// False Mourning — cannot be cast when your graveyard is empty.
#[test]
fn false_mourning_cannot_be_cast_when_your_graveyard_is_empty() {
    let mut engine = lk::game(SEED, &[forest()], &[], &[false_mourning()], &[aurochs()]);
    hand_to_graveyard(&mut engine, P1, aurochs());
    tap_all_mana(&mut engine, P0);

    assert!(!lk::castable(&engine, P0, false_mourning()));
}
