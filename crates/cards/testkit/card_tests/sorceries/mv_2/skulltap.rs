//! `cards/sorceries/mv_2/skulltap.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);

fn skulltap() -> CardIndex {
    card_index("17c7878f-891f-40a6-80a7-47fcc7b8e29b")
}

/// Skulltap (17c7878f-891f-40a6-80a7-47fcc7b8e29b) — {1}{B} Sorcery:
/// "As an additional cost to cast this spell, sacrifice a creature. Draw two
/// cards."
#[test]
fn skulltap_sacrifices_controlled_creature_and_draws_two_cards() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), llanowar_elves(), sol_ring()],
        &[aurochs()],
        &[skulltap()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let my_ring = lk::obj(&engine, P0, sol_ring());
    let lib_before = library_size(&engine, P0);

    tap_mana_where(&mut engine, P0, |id| id != my_elf && id != my_ring);
    cast_with_floating(&mut engine, P0, skulltap());
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("expected sacrifice prompt, got {:?}", engine.pending());
    };
    assert_eq!(player, P0);
    assert_eq!(options, vec![my_elf]);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![my_elf],
            },
        )
        .unwrap();
    assert_eq!(lk::zone_of(&engine, my_elf), Zone::Graveyard);

    lk::settle(&mut engine);
    assert_eq!(library_size(&engine, P0), lib_before - 2);
    assert_eq!(engine.state().zones.list(ZoneLocation::Hand(P0)).len(), 2);

    let mut no_creature = lk::game(SEED, &[swamp(), swamp()], &[aurochs()], &[skulltap()], &[]);
    tap_all_mana(&mut no_creature, P0);
    assert!(!lk::castable(&no_creature, P0, skulltap()));
}
