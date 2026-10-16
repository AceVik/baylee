//! `cards/sorceries/mv_6/rise_of_eagles.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "8fc154ff-f2a2-4089-a504-f483f545606e"
fn rise_of_eagles() -> CardIndex {
    card_index("8fc154ff-f2a2-4089-a504-f483f545606e")
}

/// Rise of Eagles: "Create two 2/2 blue Bird enchantment creature tokens with
/// flying. Scry 1."
#[test]
fn rise_of_eagles_makes_two_flying_birds_and_scries_one() {
    let mut engine = table(971, &[rise_of_eagles()], &[island(); 6], &[], &[]);
    cast_from_hand(&mut engine, me(), rise_of_eagles());
    let cards = look_and_send(&mut engine, 0);
    assert_eq!(cards.len(), 1, "scry 1 looks at one card");
    settle(&mut engine);

    let birds = my_tokens(&engine);
    assert_eq!(birds.len(), 2);
    for bird in birds {
        assert_eq!(pt(&engine, bird), (2, 2));
        let kinds = types(&engine, bird);
        assert!(kinds.contains(TypeSet::ENCHANTMENT) && kinds.contains(TypeSet::CREATURE));
        assert!(keywords_of(&engine, bird).contains(KeywordSet::FLYING), "flying");
    }
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(me())).last(),
        Some(&cards[0]),
        "kept on top"
    );
}
