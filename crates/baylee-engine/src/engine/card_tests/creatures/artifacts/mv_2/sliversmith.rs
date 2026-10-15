//! `cards/creatures/artifacts/mv_2/sliversmith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sliversmith() -> CardIndex {
    card_index("244443f0-c92b-4640-88b6-308b0312c8be")
}

/// Sliversmith: a 1/1 with "{1}, {T}, Discard a card: Create a 1/1 colorless
/// Sliver artifact creature token named Metallic Sliver." Paying with a
/// Forest and discarding the Island makes the token.
#[test]
fn sliversmith_discards_a_card_to_make_a_metallic_sliver() {
    use baylee_core::generated::subtypes::creature;
    let p0 = PlayerId::new(0);
    let mut engine = table(&[island()], &[sliversmith(), forest()], &[]);
    let smith = on_battlefield(&engine, p0, sliversmith()).expect("seated");
    assert_eq!(pt(&engine, smith), (1, 1));
    assert!(tokens_of(&engine, p0).is_empty());
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, sliversmith(), 0);
    let Pending::ChooseCards {
        player,
        prompt: crate::choice::ChoicePrompt::CostDiscard,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the discard cost question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    let island_card = in_hand(&engine, p0, island()).expect("in hand");
    assert_eq!(options, vec![island_card], "the one card in hand");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![island_card],
            },
        )
        .expect("discard the Island");
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, smith), "{{T}} is part of the price");
    assert!(in_graveyard(&engine, p0, island()).is_some(), "discarded");
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let chars = engine.state().object(tokens[0]).unwrap().characteristics();
    assert_eq!(engine.state().names.get(chars.name), "Metallic Sliver");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.colors.is_empty(), "colorless");
    assert!(chars.subtypes.contains(creature::SLIVER));
}

/// The discard is a cost: with an empty hand the ability is not on offer.
#[test]
fn sliversmith_cannot_activate_with_an_empty_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[sliversmith(), forest()], &[]);
    let smith = on_battlefield(&engine, p0, sliversmith()).expect("seated");
    tap_all_mana(&mut engine, p0);
    assert!(!priority_offer(&engine).abilities.contains(&(smith, 0)));
}
