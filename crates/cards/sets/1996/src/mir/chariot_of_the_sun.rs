//! Chariot of the Sun — {3} — Artifact
//! Oracle: {2}, {T}: Until end of turn, target creature you control gains flying and has base toughness 1.
//! Set: MIR #297 — Mirage | Scryfall ID: 2e3bc470-cfcc-4835-b46f-c08d698ee1ab | Oracle ID: b0af249d-9255-42f5-bafc-ac61ee281930
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CHARIOT_OF_THE_SUN,
    oracle_id = "b0af249d-9255-42f5-bafc-ac61ee281930",
    scryfall_id = "2e3bc470-cfcc-4835-b46f-c08d698ee1ab",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chariot of the Sun",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{2}", TapSelf),
        &[
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::FLYING),
                Duration::UntilEndOfTurn,
            ),
            Effect::continuous(
                &Filter::This,
                Modifier::SetToughness(1),
                Duration::UntilEndOfTurn,
            ),
        ],
        target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)),
    )],
);
