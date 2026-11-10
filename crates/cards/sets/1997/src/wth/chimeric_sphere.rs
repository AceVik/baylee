//! Chimeric Sphere — {3} — Artifact
//! Oracle: {2}: Until end of turn, this artifact becomes a 2/1 Construct artifact creature with flying.
//! Oracle: {2}: Until end of turn, this artifact becomes a 3/2 Construct artifact creature and loses flying.
//! Set: WTH #148 — Weatherlight | Scryfall ID: cc96857c-b38e-4614-9838-cacd3700e3ee | Oracle ID: b4aae26a-44a4-4ef0-9a1d-14a07a0865a3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CHIMERIC_SPHERE,
    oracle_id = "b4aae26a-44a4-4ef0-9a1d-14a07a0865a3",
    scryfall_id = "cc96857c-b38e-4614-9838-cacd3700e3ee",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chimeric Sphere",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        activated!(
            cost!("{2}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::ReplaceCreatureTypes(subtypes::creature::CONSTRUCT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 1),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::FLYING),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
        activated!(
            cost!("{2}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::ReplaceCreatureTypes(subtypes::creature::CONSTRUCT),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 2),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::RemoveKeyword(KeywordSet::FLYING),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
    ],
);
