//! Hostile Desert — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, Exile a land card from your graveyard: This land becomes a 3/4 Elemental creature until end of turn. It's still a land.
//! Set: MKC #266 — Murders at Karlov Manor Commander | Scryfall ID: d71031c2-7379-4d83-b6d6-61f3104593c4 | Oracle ID: 41459587-7509-404e-bd7d-fb8831dee789

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HOSTILE_DESERT,
    oracle_id = "41459587-7509-404e-bd7d-fb8831dee789",
    scryfall_id = "d71031c2-7379-4d83-b6d6-61f3104593c4",
    faces = &[face!(
        name = "Hostile Desert",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // No colour is printed, so none is set: the Elemental is colourless.
        // `AddType` keeps the land a land (CR 205.1b).
        activated!(
            cost!("{2}", ExileFromGraveyard(&Filter::LAND)),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::ELEMENTAL),
                    Duration::UntilEndOfTurn
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(3, 4),
                    Duration::UntilEndOfTurn
                ),
            ]
        ),
    ],
);
