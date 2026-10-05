//! Jandor's Ring — {6} — Artifact
//! Oracle: {2}, {T}, Discard the last card you drew this turn: Draw a card.
//! Set: SUM #256 — Summer Magic / Edgar | Scryfall ID: af9988fd-72a0-42dc-9f10-85edd3133977 | Oracle ID: 737db899-dcdb-48f9-8d30-cbbce3ae3434
// PARTIAL — the activated ability is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JANDOR_S_RING,
    oracle_id = "737db899-dcdb-48f9-8d30-cbbce3ae3434",
    scryfall_id = "af9988fd-72a0-42dc-9f10-85edd3133977",
    faces = &[face!(
        name = "Jandor's Ring",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Partial(
        "no Filter names the last card its controller drew this turn, so the \
         discard cost cannot be stated"
    ),
    // NOT SUPPORTED: "{2}, {T}, Discard the last card you drew this turn:
    // Draw a card." — the other two costs and the draw are sayable
    // (`cost!("{2}", TapSelf, Discard(&…))` and `Effect::draw(1)`), but the
    // discarded card cannot be named: `CostPart::Discard` reads a `Filter`,
    // and no `Filter` variant asks about the turn's draw record — the engine
    // keeps that record for `Effect::PayLifeOrPutBackDrawn` (Sylvan
    // Library) but exposes none of it to a filter. `Discard(&Filter::Any)`
    // would let the player discard any card, so the ability stays off the
    // card rather than offering a cheaper cost.
);
