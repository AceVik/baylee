//! Gorilla Shaman — {R} — Creature — Ape Shaman
//! Oracle: {X}{X}{1}: Destroy target noncreature artifact with mana value X.
//! Set: MH2 #280 — Modern Horizons 2 | Scryfall ID: c8f8ee19-3a88-40fa-85d8-386ffe06efd7 | Oracle ID: 9f53b691-da24-4264-a2a0-d0b95a385963
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NONCREATURE_ARTIFACT_CMC_X: Filter = Filter::And(&[
    Filter::ARTIFACT,
    Filter::LacksType(TypeSet::CREATURE),
    Filter::CmcExactlyX,
]);

card!(
    index = index::GORILLA_SHAMAN,
    oracle_id = "9f53b691-da24-4264-a2a0-d0b95a385963",
    scryfall_id = "c8f8ee19-3a88-40fa-85d8-386ffe06efd7",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Gorilla Shaman",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::APE, subtypes::creature::SHAMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{X}{X}{1}"),
        &[Effect::destroy(TargetSpec::Object(
            &NONCREATURE_ARTIFACT_CMC_X
        ))],
        target = Some(TargetSpec::Object(&NONCREATURE_ARTIFACT_CMC_X)),
    )],
);
