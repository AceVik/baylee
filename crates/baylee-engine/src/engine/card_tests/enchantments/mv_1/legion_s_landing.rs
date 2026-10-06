//! `cards/enchantments/mv_1/legion_s_landing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Legion's Landing: "When Legion's Landing enters, create a 1/1 white
/// Vampire creature token with lifelink." The card is `Coverage::Partial`
/// for its transform alone (#206); the enters trigger is written.
///
/// The enchantment is cast and the tokens are counted: one 1/1 with
/// lifelink. The legendary supertype is checked alongside, because a
/// `Partial` is not a licence to get the type line wrong.
#[test]
fn legions_landing_makes_a_lifelink_vampire_as_it_enters() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[legion_s_landing()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, legion_s_landing());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let landing = on_battlefield(&engine, p0, legion_s_landing()).expect("it resolved");
    assert!(
        engine
            .state()
            .object(landing)
            .expect("it exists")
            .characteristics()
            .supertypes
            .contains(baylee_core::types::SupertypeSet::LEGENDARY),
        "Legion's Landing is a legendary enchantment"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one Vampire");
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
    assert!(keywords(&engine, tokens[0]).contains(KeywordSet::LIFELINK));
}
