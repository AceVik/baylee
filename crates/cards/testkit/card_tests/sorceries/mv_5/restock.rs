//! `cards/sorceries/mv_5/restock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn restock() -> CardIndex {
    card_index("8b641cd3-bb08-4f2e-9dad-24f58a9c32e1")
}

/// Restock (8b641cd3-bb08-4f2e-9dad-24f58a9c32e1) — {3}{G}{G} Sorcery:
/// "Return two target cards from your graveyard to your hand. Exile Restock."
#[test]
fn restock_returns_two_cards_from_your_graveyard_to_hand_and_exiles_itself() {
    let mut engine = lk::game(
        SEED,
        &[forest(); 5],
        &[],
        &[restock(), llanowar_elves(), counterspell(), sol_ring()],
        &[aurochs()],
    );
    let spell_id = in_hand(&engine, P0, restock()).unwrap();
    let my_elf = hand_to_graveyard(&mut engine, P0, llanowar_elves());
    let my_spell = hand_to_graveyard(&mut engine, P0, counterspell());
    let my_ring = hand_to_graveyard(&mut engine, P0, sol_ring());
    let their_beast = hand_to_graveyard(&mut engine, P1, aurochs());

    cast_from_hand(&mut engine, P0, restock());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (2, 2));
    assert!(options.contains(&my_elf) && options.contains(&my_spell) && options.contains(&my_ring));
    assert!(!options.contains(&their_beast));

    lk::aim(&mut engine, &[my_elf, my_spell], &[]);
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, my_elf), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, my_spell), Zone::Hand);
    assert_eq!(lk::zone_of(&engine, my_ring), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, spell_id), Zone::Exile, "Exile Restock");
    assert!(in_graveyard(&engine, P0, restock()).is_none());

    let mut one_card = lk::game(
        SEED,
        &[forest(); 5],
        &[],
        &[restock(), llanowar_elves()],
        &[aurochs()],
    );
    hand_to_graveyard(&mut one_card, P0, llanowar_elves());
    hand_to_graveyard(&mut one_card, P1, aurochs());
    tap_all_mana(&mut one_card, P0);
    assert!(!lk::castable(&one_card, P0, restock()));
}
