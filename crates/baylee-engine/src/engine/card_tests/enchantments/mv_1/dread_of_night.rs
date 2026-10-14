//! `cards/enchantments/mv_1/dread_of_night.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dread_of_night() -> CardIndex {
    card_index("b431a73b-cf4c-4154-810f-82b7c6099e66")
}

/// "White creatures get -1/-1." Only white ones, on either side.
#[test]
fn dread_of_night_shrinks_white_creatures_only() {
    let mut e = table(
        7121,
        &[dread_of_night()],
        &[index::SERRA_ANGEL, index::GRIZZLY_BEARS],
        &[index::SERRA_ANGEL],
    );
    let mine = on_battlefield(&e, P0, index::SERRA_ANGEL).unwrap();
    let theirs = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&e, mine), (4, 4));
    cast_with_floating(&mut e, P0, dread_of_night());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, mine), (3, 3));
    assert_eq!(pt(&e, theirs), (3, 3), "an opponent's white creature too");
    assert_eq!(pt(&e, bears), (2, 2), "a green creature is untouched");
}
