//! Sandstorm Verge — (no cost) — Land — Desert
//! Oracle: {T}: Add {C}.
//! Oracle: {3}, {T}: Target creature can't block this turn. Activate only as a sorcery.
//! Set: OTJ #263 — Outlaws of Thunder Junction | Scryfall ID: 3ab4e0a4-2faf-456b-99e3-ee06c008538c | Oracle ID: de417a82-8f03-4d7e-aee7-48f7d7eba61a
// PARTIAL — {T}: Add {C} is built; the second ability is dropped because
// nothing in the DSL can say a creature may not block.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SANDSTORM_VERGE,
    oracle_id = "de417a82-8f03-4d7e-aee7-48f7d7eba61a",
    scryfall_id = "3ab4e0a4-2faf-456b-99e3-ee06c008538c",
    faces = &[face!(
        name = "Sandstorm Verge",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
    ),],
    coverage = Coverage::Partial(
        "Target creature can't block this turn — no Modifier says a creature \
         may not block, and the keyword bits that exist (menace, unblockable) \
         are about being blocked, which is the other direction"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{3}, {T}: Target creature can't block this turn.
        // Activate only as a sorcery." — the ability comes off the card: the
        // DSL has no way to express it.
    ],
);
