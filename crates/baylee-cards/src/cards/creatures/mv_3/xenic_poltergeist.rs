//! Xenic Poltergeist — {1}{B}{B} — Creature — Spirit
//! Oracle: {T}: Until your next upkeep, target noncreature artifact becomes an artifact creature with power and toughness each equal to its mana value.
//! Set: ME4 #104 — Masters Edition IV | Scryfall ID: 37055986-85e0-4568-a301-b8726da23b75 | Oracle ID: fb8f80cc-6214-4ab9-a9a9-1873ab9feb0c
// IMPLEMENTED — Modifier::AnimateNoncreatureArtifact on the target until your
// next upkeep (Duration::UntilYourNextUpkeep).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONCREATURE_ARTIFACT: Filter =
    Filter::And(&[Filter::ARTIFACT, Filter::Not(&Filter::CREATURE)]);

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
        Cost::TAP,
        &[Effect::continuous(
            &Filter::This,
            Modifier::AnimateNoncreatureArtifact,
            Duration::UntilYourNextUpkeep
        )],
        target = Some(TargetSpec::Object(&NONCREATURE_ARTIFACT))
    )],
);
