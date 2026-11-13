//! Xenic Poltergeist — {1}{B}{B} — Creature — Spirit
//! Oracle: {T}: Until your next upkeep, target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value.
//! Set: ME4 #104 — Masters Edition IV | Scryfall ID: 37055986-85e0-4568-a301-b8726da23b75 | Oracle ID: fb8f80cc-6214-4ab9-a9a9-1873ab9feb0c

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::XENIC_POLTERGEIST,
    oracle_id = "fb8f80cc-6214-4ab9-a9a9-1873ab9feb0c",
    scryfall_id = "37055986-85e0-4568-a301-b8726da23b75",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Xenic Poltergeist",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AddType(TypeSet::CREATURE),
                Duration::UntilYourNextUpkeep
            ),
            Effect::SetPTFilter {
                filter: &Filter::This,
                power: Amount::TargetCmc,
                toughness: Amount::TargetCmc,
                duration: Duration::UntilYourNextUpkeep
            },
        ],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::ARTIFACT,
            Filter::NONCREATURE
        ])))
    )],
);
