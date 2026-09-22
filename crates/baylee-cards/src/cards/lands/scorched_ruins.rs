//! Scorched Ruins — (no cost) — Land
//! Oracle: If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.
//! Oracle: {T}: Add {C}{C}{C}{C}.
//! Set: WTH #166 — Weatherlight | Scryfall ID: 75a4e843-937c-47fb-8768-0f42c5cb4e4f | Oracle ID: 6ee68855-c8c5-422b-88da-163c09a96416
// PARTIAL — {T}: Add {C}{C}{C}{C} is built; the entering drawback is not.

use baylee_cards_dsl::prelude::*;

// NOT SUPPORTED: "If this land would enter, sacrifice two untapped lands
// instead. If you do, put this land onto the battlefield. If you don't, put
// it into its owner's graveyard." — `EnterModifier` has no variant that
// charges a price in permanents as the land enters, and none of its
// price-shaped variants (`TappedOrPayLife`) puts the card in a graveyard
// when the price is not paid. An ETB trigger would be a different card: the
// land would be on the battlefield, and tapping it, before anyone paid.

card!(
    index = index::SCORCHED_RUINS,
    oracle_id = "6ee68855-c8c5-422b-88da-163c09a96416",
    scryfall_id = "75a4e843-937c-47fb-8768-0f42c5cb4e4f",
    faces = &[face!(name = "Scorched Ruins", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the enter replacement — sacrifice two untapped lands, or this land is put into its owner's graveyard — has no EnterModifier variant"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 4)])],
);
