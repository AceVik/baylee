//! Phyrexian Purge — {2}{B}{R} — Sorcery
//! Oracle: This spell costs 3 life more to cast for each target.
//! Oracle: Destroy any number of target creatures.
//! Set: MIR #273 — Mirage | Scryfall ID: 312bbc1b-4c2a-44c1-8e62-c0f94fd2ba8e | Oracle ID: df032801-fccf-4bff-aa88-d01650b6c81e
// PARTIAL: The engine has no per-target additional life cost scaling ("costs 3 life more to cast for each target"); casting this spell pays 3 life as an additional cost and destroys one target creature

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PHYREXIAN_PURGE,
    oracle_id = "df032801-fccf-4bff-aa88-d01650b6c81e",
    scryfall_id = "312bbc1b-4c2a-44c1-8e62-c0f94fd2ba8e",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    coverage = Coverage::Partial(
        "The engine has no per-target additional life cost scaling (\"costs 3 life more to cast for each target\"); casting this spell pays 3 life as an additional cost and destroys one target creature"
    ),
    faces = &[face!(
        name = "Phyrexian Purge",
        mana_cost = mana!("{2}{B}{R}"),
        types = TypeSet::SORCERY,
        mandatory_additional_costs = &[CostPart::PayLife(3)],
    ),],
    abilities = &[spell!(
        &[Effect::Destroy {
            target: TargetSpec::Object(&Filter::CREATURE),
            no_regen: false,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
