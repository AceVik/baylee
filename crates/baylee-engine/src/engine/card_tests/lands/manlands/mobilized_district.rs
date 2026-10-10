//! `cards/lands/manlands/mobilized_district.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Without legendary creatures or planeswalkers, animation costs the full four
/// generic mana and creates a vigilant 3/3 Citizen that remains a land.
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

#[test]
fn mobilized_district_counts_legendary_creatures_and_planeswalkers_once_each() {
    use crate::engine::card_tests::legends_kit as kit;
    let emry = card_index("da3e7d3d-2ca0-40c3-9602-fca37c92f507");
    let mut engine = kit::game(
        934,
        &[
            mobilized_district(),
            emry,
            karn_the_great_creator(),
            llanowar_elves(),
        ],
        &[emry],
        &[],
        &[],
    );
    kit::float(&mut engine, kit::P0, &[(ManaColor::Blue, 1)]);
    assert!(!kit::offered(&engine, mobilized_district(), kit::P0, 1));
    kit::float(&mut engine, kit::P0, &[(ManaColor::Blue, 1)]);
    kit::use_ability(&mut engine, kit::P0, mobilized_district(), 1, &[], &[]);
    assert_eq!(kit::pool_total(&engine, kit::P0), 0);
    let district = kit::obj(&engine, kit::P0, mobilized_district());
    assert_eq!(pt(&engine, district), (3, 3));
}
