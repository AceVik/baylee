//! Necratog — {1}{B}{B} — Creature — Atog
//! Oracle: Exile the top creature card of your graveyard: this creature gets +2/+2 until end of turn.
//! Set: WTH #76 — Weatherlight | Scryfall ID: fb19c519-c09a-44a0-8d4b-ab6c15dabdef | Oracle ID: 9f820f1e-3cd1-4a3e-8d00-194c8a80b296
// PARTIAL — the body only. NOT SUPPORTED: "Exile the top creature card of your graveyard: this creature gets +2/+2 until end of turn" — `CostPart::ExileFromGraveyard` lets the player pick any creature card of the graveyard, not the top one, so the ability is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NECRATOG,
    oracle_id = "9f820f1e-3cd1-4a3e-8d00-194c8a80b296",
    scryfall_id = "fb19c519-c09a-44a0-8d4b-ab6c15dabdef",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the 1/2 body only; the ability 'exile the top creature card of your graveyard: +2/+2 until end of turn' is left off because the exile cost cannot name the top creature card, and picking any creature card would be a different card than printed",
    ),
    faces = &[face!(
        name = "Necratog",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ATOG],
        power = Some(1),
        toughness = Some(2),
    ),],
);
