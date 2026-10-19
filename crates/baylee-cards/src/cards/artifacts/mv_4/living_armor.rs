//! Living Armor — {4} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Put X +0/+1 counters on target creature, where X is that creature's mana value.
//! Set: CHR #103 — Chronicles | Scryfall ID: b9da41f7-c1d2-4d5c-b6a5-09f6cc05cd4a | Oracle ID: 7a32b0d1-d9dc-4d24-ae60-3f3a0e18ec03

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIVING_ARMOR,
    oracle_id = "7a32b0d1-d9dc-4d24-ae60-3f3a0e18ec03",
    scryfall_id = "b9da41f7-c1d2-4d5c-b6a5-09f6cc05cd4a",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Living Armor",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        abilities = &[activated!(
            cost!("", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::Plus {
                    power: 0,
                    toughness: 1,
                },
                amount: Amount::TargetCmc,
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        )],
    ),],
);
