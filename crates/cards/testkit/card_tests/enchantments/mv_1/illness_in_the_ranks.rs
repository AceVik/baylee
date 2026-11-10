//! `cards/enchantments/mv_1/illness_in_the_ranks.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn illness_in_the_ranks() -> CardIndex {
    card_index("51abc95c-bd92-46f7-a0a1-6e9f8e0dda87")
}

fn sarcomancy() -> CardIndex {
    card_index("bd1e93da-7524-4bdd-9738-accd76bda7f7")
}

/// "Creature tokens get -1/-1." A 2/2 Zombie token becomes 1/1, a nontoken
/// creature is untouched.
#[test]
fn illness_in_the_ranks_shrinks_tokens_only() {
    let mut e = table(
        7151,
        &[sarcomancy()],
        &[illness_in_the_ranks(), index::GRIZZLY_BEARS],
        &[],
    );
    cast_with_floating(&mut e, P0, sarcomancy());
    pass_until(&mut e, stack_is_empty);
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let tokens = tokens_of(&e, P0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&e, tokens[0]), (1, 1));
    assert_eq!(pt(&e, bears), (2, 2));
}
