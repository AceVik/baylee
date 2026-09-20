//! Bonders' Enclave — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Draw a card. Activate only if you control a creature with power 4 or greater.
//! Set: OTC #274 — Outlaws of Thunder Junction Commander | Scryfall ID: d174f42c-dcf2-4da9-812b-7e1d0991d466 | Oracle ID: f33ce38a-34ec-4b65-a0fc-160484a02007
// PARTIAL — {T}: Add {C} is built; the {3}, {T} draw is dropped rather
// than offered ungated, because its gate is not sayable.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BONDERS_ENCLAVE,
    oracle_id = "f33ce38a-34ec-4b65-a0fc-160484a02007",
    scryfall_id = "d174f42c-dcf2-4da9-812b-7e1d0991d466",
    faces = &[face!(name = "Bonders' Enclave", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "{3}, {T}: Draw a card is gated on \"a creature with power 4 or greater\", \
         and no Filter compares power (ToughnessAtMost is the only P/T predicate)"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: {3}, {T}: Draw a card. Activate only if you control a creature with power 4 or greater.
        // The gate is the card, not the machinery: Condition::ControlCount
        // exists and asks a Filter once, so the sentence would be one line —
        // if the filter could say "power 4 or greater". It cannot: the
        // vocabulary carries CmcAtMost/CmcAtLeast and ToughnessAtMost and no
        // power comparison at all, so an activation written here would be
        // offered on a board with no such creature and hand out a free card
        // (CR 602.5 forbids beginning a prohibited activation). Written
        // ungated it is a rules bug, so the ability comes off the card.
    ],
);
