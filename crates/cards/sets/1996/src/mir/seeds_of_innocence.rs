//! Seeds of Innocence — {1}{G}{G} — Sorcery
//! Oracle: Destroy all artifacts. They can't be regenerated. The controller of each of those artifacts gains life equal to its mana value.
//! Set: MIR #241 — Mirage | Scryfall ID: f9c868f5-0f90-4f7e-bafb-c45d2372fe06 | Oracle ID: f5ebad5c-9675-4a37-b76c-760416083939
// IMPLEMENTED — captures all artifacts, destroys them without regeneration, and grants each artifact's controller life equal to its mana value.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEEDS_OF_INNOCENCE,
    oracle_id = "f5ebad5c-9675-4a37-b76c-760416083939",
    scryfall_id = "f9c868f5-0f90-4f7e-bafb-c45d2372fe06",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Seeds of Innocence",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::CaptureMatching {
        filter: &Filter::ARTIFACT,
        then: &[
            Effect::DestroyAll {
                filter: &Filter::ARTIFACT,
                no_regen: true,
            },
            Effect::ForEachRemembered {
                filter: &Filter::Any,
                effects: &[Effect::GainLifeFor {
                    who: PlayerRel::EventPlayer,
                    amount: Amount::EventManaValue,
                }],
            },
        ],
    }])],
);
