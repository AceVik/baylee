//! `cards/creatures/artifacts/mv_3/etherium_spinner.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn etherium_spinner() -> CardIndex {
    card_index("b501128b-2797-401b-8aec-923050081cae")
}

/// Etherium Spinner — "Whenever you cast a spell with mana value 4 or
/// greater, create a 1/1 colorless Thopter artifact creature token with
/// flying." A three-mana spell makes nothing; a four-mana one makes the
/// Thopter.
#[test]
fn etherium_spinner_makes_a_thopter_for_a_big_spell_only() {
    let p0 = PlayerId::new(0);
    let mut engine = main_with(
        &[onulet(), cobalt_golem()],
        &[
            etherium_spinner(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
            forest(),
        ],
    );
    let spinner = on_battlefield(&engine, p0, etherium_spinner()).expect("seated");
    assert_eq!(pt(&engine, spinner), (2, 1));
    tap_all_mana(&mut engine, p0);

    cast_with_floating(&mut engine, p0, onulet());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "mana value 3 is below the line"
    );

    cast_with_floating(&mut engine, p0, cobalt_golem());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "mana value 4 makes one Thopter");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::FLYING));
    let t = types(&engine, tokens[0]);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(on_battlefield(&engine, p0, cobalt_golem()).is_some());
}
