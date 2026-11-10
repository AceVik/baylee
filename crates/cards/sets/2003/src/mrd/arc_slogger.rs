//! Arc-Slogger — {3}{R}{R} — Creature — Beast
//! Oracle: {R}, Exile the top ten cards of your library: This creature deals 2 damage to any target.
//! Set: MRD #85 — Mirrodin | Scryfall ID: d3dd67e0-72b4-4c55-b49b-c69950feccb1 | Oracle ID: a28fc506-a98c-4b4d-a0f4-7d971489ccbb
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARC_SLOGGER,
    oracle_id = "a28fc506-a98c-4b4d-a0f4-7d971489ccbb",
    scryfall_id = "d3dd67e0-72b4-4c55-b49b-c69950feccb1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Arc-Slogger",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(5),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        Cost {
            mana: mana!("{R}"),
            parts: &[CostPart::ExileTopOfLibrary(10)],
        },
        &[Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::AnyTarget,
        }],
        target = Some(TargetSpec::AnyTarget),
    )],
);
