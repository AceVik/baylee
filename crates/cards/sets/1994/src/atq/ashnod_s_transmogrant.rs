//! Ashnod's Transmogrant — {1} — Artifact
//! Oracle: {T}, Sacrifice this artifact: Put a +1/+1 counter on target nonartifact creature. That creature becomes an artifact in addition to its other types.
//! Set: ME1 #152 — Masters Edition | Scryfall ID: 31c830a1-5b93-493f-9976-af950f6ec70a | Oracle ID: 1a8072c9-e2b8-4173-af88-ed0dd64d10fe
// IMPLEMENTED — {T}, sacrifice this artifact: a +1/+1 counter on target
// nonartifact creature, which becomes an artifact indefinitely.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASHNOD_S_TRANSMOGRANT,
    oracle_id = "1a8072c9-e2b8-4173-af88-ed0dd64d10fe",
    scryfall_id = "31c830a1-5b93-493f-9976-af950f6ec70a",
    faces = &[face!(
        name = "Ashnod's Transmogrant",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::continuous(
                &Filter::This,
                Modifier::AddType(TypeSet::ARTIFACT),
                Duration::Indefinitely,
            ),
        ],
        target = Some(TargetSpec::Object(&Filter::And(&[
            Filter::CREATURE,
            Filter::LacksType(TypeSet::ARTIFACT),
        ])))
    )],
);
