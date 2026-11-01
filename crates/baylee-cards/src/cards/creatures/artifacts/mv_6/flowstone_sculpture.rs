//! Flowstone Sculpture — {6} — Artifact Creature — Shapeshifter
//! Oracle: {2}, Discard a card: Put a +1/+1 counter on this creature or this creature gains flying, first strike, or trample. (This effect lasts indefinitely.)
//! Set: VMA #268 — Vintage Masters | Scryfall ID: 448d1d1d-00bc-4026-8a63-4812d1f28502 | Oracle ID: 42b01697-8663-4b53-86d5-699063bd0778
// IMPLEMENTED — choose a counter or a permanent keyword on resolution after paying {2} and discarding.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLOWSTONE_SCULPTURE,
    oracle_id = "42b01697-8663-4b53-86d5-699063bd0778",
    scryfall_id = "448d1d1d-00bc-4026-8a63-4812d1f28502",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Flowstone Sculpture",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::SHAPESHIFTER],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!("{2}", Discard(&Filter::Any)),
        &[Effect::ChooseOne {
            choices: &[
                Effect::AddCounterSource {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1)
                },
                Effect::ModifySource {
                    modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                    duration: Duration::Indefinitely
                },
                Effect::ModifySource {
                    modifier: Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                    duration: Duration::Indefinitely
                },
                Effect::ModifySource {
                    modifier: Modifier::AddKeyword(KeywordSet::TRAMPLE),
                    duration: Duration::Indefinitely
                },
            ]
        }],
    )],
);
