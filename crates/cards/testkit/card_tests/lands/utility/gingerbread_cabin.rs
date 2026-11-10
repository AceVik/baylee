//! `cards/lands/utility/gingerbread_cabin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gingerbread Cabin is the same rule over Forests, and it is here because
/// the three cards read the *subtype* out of their own filter: a rule that
/// wrote the wrong one would still pass every assertion above.
#[test]
fn a_gingerbread_cabin_counts_forests_and_not_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(216, forest())
        .battlefield(0, &[forest(), forest(), island()])
        .hand(0, &[gingerbread_cabin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cabin = play_land(&mut engine, p0, gingerbread_cabin());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        entered_tapped(&engine, cabin),
        "two Forests and an Island are not three other Forests"
    );
    assert!(tokens_of(&engine, p0).is_empty(), "no Food either");
}

/// Gingerbread Cabin: "When this land enters untapped, create a Food
/// token." Three other Forests: it enters untapped and makes the Food.
#[test]
fn gingerbread_cabin_makes_a_food_when_it_enters_untapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4203, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[gingerbread_cabin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let cabin = play_land(&mut engine, p0, gingerbread_cabin());
    pass_until(&mut engine, stack_is_empty);
    assert!(!entered_tapped(&engine, cabin));
    assert_eq!(tokens_of(&engine, p0).len(), 1, "one Food token");
}
