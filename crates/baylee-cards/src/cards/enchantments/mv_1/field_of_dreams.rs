//! Field of Dreams — {U} — World Enchantment
//! Oracle: Players play with the top card of their libraries revealed.
//! Set: LEG #55 — Legends | Scryfall ID: 6a63e119-3b1b-4964-a4b9-b10170ff542b | Oracle ID: 6dcc62f8-2fd9-473b-9e6a-2dd7e610f855
// PARTIAL — RevealLibraryTop reveals only the source controller's library
// top, not every player's own, so the static is off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIELD_OF_DREAMS,
    oracle_id = "6dcc62f8-2fd9-473b-9e6a-2dd7e610f855",
    scryfall_id = "6a63e119-3b1b-4964-a4b9-b10170ff542b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Modifier::RevealLibraryTop is read as fx.controller == player, so it \
         reveals only the source controller's library top — Courser of \
         Kruphix's sentence, not every player's own"
    ),
    faces = &[face!(
        name = "Field of Dreams",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// NOT SUPPORTED: "Players play with the top card of their libraries
// revealed." — GameState::library_top_revealed asks the effect's controller,
// so one static makes one library public; nothing reveals every player's own
// top card.
