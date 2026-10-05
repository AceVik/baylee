//! Land Equilibrium — {2}{U}{U} — Enchantment
//! Oracle: If an opponent who controls at least as many lands as you do would put a land onto the battlefield, that player instead puts that land onto the battlefield then sacrifices a land of their choice.
//! Set: ME3 #40 — Masters Edition III | Scryfall ID: 51c5df75-746c-4e4b-84f1-76b689d317d7 | Oracle ID: a3711453-b17d-4b1c-b726-9b41f36d07ab
// PARTIAL — the replacement is off the card; no `ReplacementRule` replaces a
// zone change or tacks a sacrifice onto it (see the NOT SUPPORTED line).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LAND_EQUILIBRIUM,
    oracle_id = "a3711453-b17d-4b1c-b726-9b41f36d07ab",
    scryfall_id = "51c5df75-746c-4e4b-84f1-76b689d317d7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "no `ReplacementRule` replaces putting a land onto the battlefield or \
         appends an action to that entry, and no filter compares an opponent's \
         land count with yours"
    ),
    faces = &[face!(
        name = "Land Equilibrium",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    // NOT SUPPORTED: "If an opponent who controls at least as many lands as
    // you do would put a land onto the battlefield, that player instead puts
    // that land onto the battlefield then sacrifices a land of their choice."
    // — `ReplacementRule` has no entry-replacement variant (its variants
    // cover graveyard redirection, token/counter doubling, trigger
    // multiplication and turn skips only), so the "would put … instead …
    // then sacrifices" shape cannot be registered; there is also no filter
    // that compares one player's land count with another's.
    abilities = &[],
);
