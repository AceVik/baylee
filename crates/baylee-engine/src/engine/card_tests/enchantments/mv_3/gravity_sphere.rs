//! `cards/enchantments/mv_3/gravity_sphere.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gravity Sphere — {2}{R} World enchantment: "All creatures lose flying."
///
/// The reading is the two-sided one, because "all" is exactly what a filter
/// narrowed to the controller would keep: the Sphere is cast by p0 and has to
/// ground a Sphinx of the Final Word across the table. That Sphinx is also the
/// control for the other half of the sentence — it prints hexproof beside
/// flying, so the same creature says both that one keyword went and that the
/// rest stayed, and its body is compared against the one read before the
/// Sphere resolved, so a card that had stripped every keyword cannot pass on
/// the strength of a missing `FLYING` alone. The Elf under the Sphere's own
/// controller is the third reading: a creature with nothing to lose comes out
/// of the resolution exactly as it went in.
#[test]
fn gravity_sphere_grounds_every_creature_and_takes_only_flying() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[gravity_sphere()])
        .battlefield(1, &[sphinx_of_the_final_word()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sphinx =
        on_battlefield(&engine, p1, sphinx_of_the_final_word()).expect("the Sphinx is out");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "the Sphinx prints flying before anything is cast"
    );
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::HEXPROOF),
        "and hexproof beside it, which the Sphere has no business touching"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the Elf never had flying, so it is the negative half of the claim"
    );
    let body = pt(&engine, sphinx);

    // {2}{R} off the three Mountains; `tap_all_mana` also takes the Elf's own
    // `{T}: Add {G}`, which is a printed mana ability and no reason for the
    // Elf to move.
    cast_from_hand(&mut engine, p0, gravity_sphere());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, gravity_sphere()).is_some()
    });

    assert!(
        on_battlefield(&engine, p0, gravity_sphere()).is_some(),
        "the enchantment resolved onto the battlefield"
    );
    assert!(
        !keywords(&engine, sphinx).contains(KeywordSet::FLYING),
        "\"All creatures lose flying\" reaches the creature across the table"
    );
    assert!(
        keywords(&engine, sphinx).contains(KeywordSet::HEXPROOF),
        "the Sphere removes one keyword and not the set it lives in"
    );
    assert_eq!(
        pt(&engine, sphinx),
        body,
        "and the Sphinx is still the body it was, so nothing was removed but \
         the flying"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the creature that had no flying is untouched"
    );
}
