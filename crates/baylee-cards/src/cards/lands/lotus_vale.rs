//! Lotus Vale — (no cost) — Land
//! Oracle: If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add three mana of any one color.
//! Set: WTH #165 — Weatherlight | Scryfall ID: 2e5cd12a-2a07-44a8-8eac-de00d26fe9e3 | Oracle ID: 01fc5bb3-ebd7-4ab4-8aef-2ece1e1d9b7c
// PARTIAL — the mana ability is built: one color pick for three mana
// (`Effect::mana_choice_dynamic`, because "any one color" is one choice for
// the whole amount and not a combination). The entry replacement is not.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LOTUS_VALE,
    oracle_id = "01fc5bb3-ebd7-4ab4-8aef-2ece1e1d9b7c",
    scryfall_id = "2e5cd12a-2a07-44a8-8eac-de00d26fe9e3",
    faces = &[face!(name = "Lotus Vale", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the entry replacement — sacrifice two untapped lands, or else put this into its graveyard — has no EnterModifier and no ReplacementRule variant"
    ),
    abilities = &[
        // NOT SUPPORTED: "If this land would enter, sacrifice two untapped
        // lands instead. If you do, put this land onto the battlefield. If
        // you don't, put it into its owner's graveyard." — `EnterModifier`
        // can make a land enter tapped or ask for a mana payment, and
        // `ReplacementRule` carries only token/counter doubling and trigger
        // multipliers; nothing replaces *this permanent's own arrival* with a
        // cost paid by sacrificing other permanents, so the land would enter
        // unconditionally and for free.
        mana_ability!(&[Effect::mana_choice_dynamic(
            ALL_MANA_COLORS,
            Amount::Fixed(3)
        )]),
    ],
);
