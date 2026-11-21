//! `cards/sorceries/mv_2/diabolic_intent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn diabolic_intent() -> CardIndex {
    card_index("038519b9-bca8-4b27-b5ac-2409595469d0")
}

/// Diabolic Intent (038519b9-bca8-4b27-b5ac-2409595469d0) — {1}{B} Sorcery:
/// "As an additional cost to cast this spell, sacrifice a creature. Search your
/// library for a card, put that card into your hand, then shuffle."
#[test]
fn diabolic_intent_sacrifices_a_creature_and_tutors_any_card_into_hand() {
    let mut engine = lk::game(
        SEED,
        &[swamp(), swamp(), llanowar_elves(), sol_ring()],
        &[aurochs()],
        &[diabolic_intent(), counterspell()],
        &[],
    );
    let my_elf = lk::obj(&engine, P0, llanowar_elves());
    let my_ring = lk::obj(&engine, P0, sol_ring());
    let spell = hand_to_library_top(&mut engine, P0, counterspell());
    let lib_before = library_size(&engine, P0);

    tap_mana_where(&mut engine, P0, |id| id != my_elf && id != my_ring);
    cast_with_floating(&mut engine, P0, diabolic_intent());
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

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::SearchLibrary,
                ..
            }
        )
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![spell],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, spell), Zone::Hand);
    assert_eq!(library_size(&engine, P0), lib_before - 1);

    let mut no_creature = lk::game(
        SEED,
        &[swamp(), swamp()],
        &[aurochs()],
        &[diabolic_intent()],
        &[],
    );
    let _ = P1;
    tap_all_mana(&mut no_creature, P0);
    assert!(!lk::castable(&no_creature, P0, diabolic_intent()));
}
