//! `cards/lands/manlands/mobilized_district.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mobilized District prints `{{T}}: Add {{C}}` and `{4}: This land becomes a
/// 3/3 Citizen creature with vigilance until end of turn. It's still a land.
/// This ability costs {1} less to activate for each legendary creature and
/// planeswalker you control.`
///
/// Under `Coverage::Partial`, the cost reduction for legendary creatures and
/// planeswalkers is unsupported and dropped. This test pays `{4}` off four
/// basic lands to animate Mobilized District into a 3/3 Citizen creature with
/// vigilance that remains a land.
#[test]
fn mobilized_district_animates_into_a_vigilant_citizen() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mobilized_district(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let district =
        on_battlefield(&engine, p0, mobilized_district()).expect("district on battlefield");
    assert!(
        !engine
            .state()
            .object(district)
            .expect("district exists")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a land is not a creature before activation"
    );

    tap_all_mana_but(&mut engine, p0, Some(mobilized_district()));
    activate(&mut engine, p0, mobilized_district(), 1);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(district)
        .expect("district exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::LAND), "it's still a land");
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::CITIZEN)
    );
    assert!(keywords(&engine, district).contains(KeywordSet::VIGILANCE));
    assert_eq!(pt(&engine, district), (3, 3));
}
