//! `cards/enchantments/mv_5/sigil_of_the_empty_throne.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sigil_of_the_empty_throne() -> CardIndex {
    card_index("3b9b5b22-5a7d-4e37-a870-ca0f0efa4f36")
}

fn six_forests() -> [CardIndex; 7] {
    [
        sigil_of_the_empty_throne(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
        forest(),
    ]
}

/// "Whenever you cast an enchantment spell, create a 4/4 white Angel
/// creature token with flying." Casting Centaur Glade makes the Angel;
/// casting a creature spell does not.
#[test]
fn sigil_of_the_empty_throne_makes_an_angel_for_an_enchantment_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[centaur_glade(), llanowar_elves()])
        .battlefield(0, &six_forests())
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "a creature spell: no Angel"
    );

    cast_with_floating(&mut engine, p0, centaur_glade());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Angel");
    assert_eq!(pt(&engine, tokens[0]), (4, 4));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
}
