//! Sindbad — {1}{U} — Creature — Human
//! Oracle: {T}: Draw a card and reveal it. If it isn't a land card, discard it.
//! Set: TSB #31 — Time Spiral Timeshifted | Scryfall ID: 6a6372ec-1ae2-4806-bb31-78f9ea6259df | Oracle ID: dc81069b-b2cf-44b3-98fc-45bb24b815cb
// PARTIAL — the draw-and-reveal ability is off the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SINDBAD,
    oracle_id = "dc81069b-b2cf-44b3-98fc-45bb24b815cb",
    scryfall_id = "6a6372ec-1ae2-4806-bb31-78f9ea6259df",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sindbad",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "no effect draws a card and reveals the drawn card, so the conditional discard has nothing to read"
    ),
    // NOT SUPPORTED: "{T}: Draw a card and reveal it. If it isn't a land card,
    // discard it." — the draw and the reveal are each sayable (`Effect::draw(1)`
    // and `Effect::RevealTopAndSort` over `Filter::LAND`), but the sort acts on
    // the *top card of the library* rather than on the drawn card, so nothing
    // draws it and draw replacement effects and "whenever you draw" triggers
    // never see it; and its `SearchDest` has no graveyard, so the nonland half
    // cannot even be the printed discard. No Effect draws and reveals one card,
    // and `Effect::DiscardForPlayers` chooses among the whole hand without
    // knowing which card was drawn, so the ability comes off the card.
);
