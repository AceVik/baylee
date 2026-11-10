//! Brass-Talon Chimera — {4} — Artifact Creature — Chimera
//! Oracle: First strike
//! Oracle: Sacrifice this creature: Put a +2/+2 counter on target Chimera creature. It gains first strike. (This effect lasts indefinitely.)
//! Set: VIS #142 — Visions | Scryfall ID: 200c9655-e51c-4b63-96cf-7f3fba3ec75c | Oracle ID: 2e114d15-4356-4cef-8f89-260ab4c8d4b9
// IMPLEMENTED — first strike and sacrifice ability putting a +2/+2 counter on target Chimera creature and granting it first strike indefinitely.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHIMERA_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::CHIMERA),
]);

card!(
    index = index::BRASS_TALON_CHIMERA,
    oracle_id = "2e114d15-4356-4cef-8f89-260ab4c8d4b9",
    scryfall_id = "200c9655-e51c-4b63-96cf-7f3fba3ec75c",
    keywords = KeywordSet::FIRST_STRIKE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brass-Talon Chimera",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CHIMERA],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FIRST_STRIKE,
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
                keywords: KeywordSet::FIRST_STRIKE,
                duration: Duration::Indefinitely,
            },
        ],
        target = Some(TargetSpec::Object(&CHIMERA_CREATURE)),
    )],
);
