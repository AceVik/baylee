//! Crimson Hellkite — {6}{R}{R}{R} — Creature — Dragon
//! Oracle: Flying
//! Oracle: {X}, {T}: This creature deals X damage to target creature. Spend only red mana on X.
//! Set: 7ED #178 — Seventh Edition | Scryfall ID: 5451e44f-8dd2-48c6-ba67-5c62a04819ef | Oracle ID: 37a1f340-10d5-407e-bf2b-cbbbb6cab985
// NOT SUPPORTED: Restricting the mana spent on `{X}` in an activated ability's cost to red mana only (`Spend only red mana on X`) is not yet supported by `Cost`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRIMSON_HELLKITE,
    oracle_id = "37a1f340-10d5-407e-bf2b-cbbbb6cab985",
    scryfall_id = "5451e44f-8dd2-48c6-ba67-5c62a04819ef",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "Restricting the mana spent on `{X}` in an activated ability's cost to red mana only (`Spend only red mana on X`) is not yet supported by `Cost`; in play, `{X}, {T}` deals X damage to target creature using any mana to pay `{X}`.",
    ),
    faces = &[face!(
        name = "Crimson Hellkite",
        mana_cost = mana!("{6}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
        keywords = KeywordSet::FLYING,
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::DealDamage {
            target: TargetSpec::Object(&Filter::CREATURE),
            amount: Amount::X,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
    )],
);
