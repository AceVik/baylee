//! Lead-Belly Chimera — {4} — Artifact Creature — Chimera
//! Oracle: Trample
//! Oracle: Sacrifice this creature: Put a +2/+2 counter on target Chimera creature. It gains trample. (This effect lasts indefinitely.)
//! Set: VIS #148 — Visions | Scryfall ID: 5d89b377-80d2-42a0-b84e-a455a72ed9fe | Oracle ID: e8fb2323-3eed-45a6-bfc4-44292c8b3697
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHIMERA_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::CHIMERA),
]);

card!(
    index = index::LEAD_BELLY_CHIMERA,
    oracle_id = "e8fb2323-3eed-45a6-bfc4-44292c8b3697",
    scryfall_id = "5d89b377-80d2-42a0-b84e-a455a72ed9fe",
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lead-Belly Chimera",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CHIMERA],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(SacrificeSelf),
        &[
            Effect::AddCounter {
                kind: CounterKind::Plus {
                    power: 2,
                    toughness: 2,
                },
                amount: Amount::Fixed(1),
            },
            Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::TRAMPLE,
                duration: Duration::Indefinitely,
            },
        ],
        target = Some(TargetSpec::Object(&CHIMERA_CREATURE)),
    )],
);
