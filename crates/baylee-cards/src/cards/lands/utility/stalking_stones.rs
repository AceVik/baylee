//! Stalking Stones — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {6}: This land becomes a 3/3 Elemental artifact creature that's still a land. (This effect lasts indefinitely.)
//! Set: TPR #245 — Tempest Remastered | Scryfall ID: 27856bc5-4f39-4461-baa0-700442942aff | Oracle ID: f3658894-3d3d-4cd4-b0ac-c53e1d08747c
// IMPLEMENTED — {T} for {C}, and {6} to animate the land into a 3/3 Elemental
// artifact creature that is still a land (three layer effects, indefinitely).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::STALKING_STONES,
    oracle_id = "f3658894-3d3d-4cd4-b0ac-c53e1d08747c",
    scryfall_id = "27856bc5-4f39-4461-baa0-700442942aff",
    faces = &[face!(name = "Stalking Stones", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{6}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::ELEMENTAL),
                    Duration::Indefinitely,
                ),
                Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::Indefinitely,),
            ],
        ),
    ],
);
