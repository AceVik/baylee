//! Hostile Desert — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, Exile a land card from your graveyard: This land becomes a 3/4 Elemental creature until end of turn. It's still a land.
//! Set: MKC #266 — Murders at Karlov Manor Commander | Scryfall ID: d71031c2-7379-4d83-b6d6-61f3104593c4 | Oracle ID: 41459587-7509-404e-bd7d-fb8831dee789
// PARTIAL — {T}: Add {C}; the animate half is dropped, its cost cannot be said.

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
    coverage = Coverage::Partial(
        "no CostPart exiles a land card from a graveyard, so the land's second ability is not writable"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {2}, Exile a land card from your graveyard: This land becomes a 3/4 Elemental creature until end of turn. It's still a land. — the cost names a card in a graveyard, and no `CostPart` reaches that zone (`ExileFromHand` is the hand, and an activation never pays it).
    ],
);
