//! Tin-Wing Chimera — {4} — Artifact Creature — Chimera
//! Oracle: Flying
//! Oracle: Sacrifice this creature: Put a +2/+2 counter on target Chimera creature. It gains flying. (This effect lasts indefinitely.)
//! Set: VIS #157 — Visions | Scryfall ID: 3375dcc6-9399-48eb-9aa4-7b40c3686cc5 | Oracle ID: 0d5fe27f-db51-4323-aa74-b6fdb4309425
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CHIMERA_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::creature::CHIMERA),
]);

card!(
    index = index::TIN_WING_CHIMERA,
    oracle_id = "0d5fe27f-db51-4323-aa74-b6fdb4309425",
    scryfall_id = "3375dcc6-9399-48eb-9aa4-7b40c3686cc5",
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tin-Wing Chimera",
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
                keywords: KeywordSet::FLYING,
                duration: Duration::Indefinitely,
            },
        ],
        target = Some(TargetSpec::Object(&CHIMERA_CREATURE)),
    )],
);
