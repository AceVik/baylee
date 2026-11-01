//! Iron-Heart Chimera — {4} — Artifact Creature — Chimera
//! Oracle: Vigilance
//! Oracle: Sacrifice this creature: Put a +2/+2 counter on target Chimera creature. It gains vigilance. (This effect lasts indefinitely.)
//! Set: VIS #146 — Visions | Scryfall ID: 5899a575-a97d-4850-b55c-22ad6900ba20 | Oracle ID: fe7c70dd-76ff-4958-9973-6452c7f071ed
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHIMERA_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::CHIMERA),
]);

card!(
    index = index::IRON_HEART_CHIMERA,
    oracle_id = "fe7c70dd-76ff-4958-9973-6452c7f071ed",
    scryfall_id = "5899a575-a97d-4850-b55c-22ad6900ba20",
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Iron-Heart Chimera",
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
                keywords: KeywordSet::VIGILANCE,
                duration: Duration::Indefinitely,
            },
        ],
        target = Some(TargetSpec::Object(&CHIMERA_CREATURE)),
    )],
);
