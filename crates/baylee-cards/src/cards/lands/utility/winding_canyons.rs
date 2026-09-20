//! Winding Canyons — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: You may cast creature spells this turn as though they had flash.
//! Set: WTH #167 — Weatherlight | Scryfall ID: f26672a8-f4ff-4c64-bb3e-f5072bbc9e3e | Oracle ID: 622e2561-48b1-4aca-9abb-9a3c284dcceb
// IMPLEMENTED — {T}: Add {C}. The second ability is dropped (see below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINDING_CANYONS,
    oracle_id = "622e2561-48b1-4aca-9abb-9a3c284dcceb",
    scryfall_id = "f26672a8-f4ff-4c64-bb3e-f5072bbc9e3e",
    faces = &[face!(name = "Winding Canyons", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "\"{2}, {T}: You may cast creature spells this turn as though they had flash.\" — \
         the DSL's only cast-as-though-flash effect is Modifier::SorceriesHaveFlash, which \
         names sorceries, and no variant grants flash to creature spells"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{2}, {T}: You may cast creature spells this turn as though they had flash."
//   — the ability is off the card rather than approximated. A flash permission is read while
//   the card is still in hand, so spelling it as Modifier::AddKeyword(FLASH) over a filter on
//   creature spells would add an ability to an object at a moment nobody consults it and
//   grant no permission at all.
