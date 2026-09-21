//! The Lonely Mountain — (no cost) — Land — Mountain
//! Oracle: ({T}: Add {R}.)
//! Oracle: This land enters tapped unless you control an Equipment.
//! Oracle: {4}{R}, {T}: Create a 2/2 red Dwarf creature token. This ability costs {1} less to activate for each Equipment you control. Activate only as a sorcery.
//! Set: HOB #187 — The Hobbit | Scryfall ID: b39ebc4d-a01a-4401-ab3a-bf6142c93b47 | Oracle ID: 3678c06f-8a33-4a6d-bf20-5b92d5c05a95
// PARTIAL — {T}: Add {R}, and it enters tapped unless you control an
// Equipment; the {4}{R} token ability is not printed (see the note below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THE_LONELY_MOUNTAIN,
    oracle_id = "3678c06f-8a33-4a6d-bf20-5b92d5c05a95",
    scryfall_id = "b39ebc4d-a01a-4401-ab3a-bf6142c93b47",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "The Lonely Mountain",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::MOUNTAIN],
        enter_modifiers = &[EnterModifier::TappedUnless(&f!(
            your Filter::HasSubtype(subtypes::artifact::EQUIPMENT)
        ))],
    ),],
    coverage = Coverage::Partial(
        "the token ability's \"This ability costs {1} less to activate for each Equipment you \
         control\": no variant reduces an activation cost (CostReduction carries only \
         NotStartingPlayer), and the 2/2 red Dwarf it makes is not one of the tokens \
         `crate::tokens` declares, which is the only registry a card file may name"
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Red, 1)])],
);

// NOT SUPPORTED: "{4}{R}, {T}: Create a 2/2 red Dwarf creature token. This
// ability costs {1} less to activate for each Equipment you control. Activate
// only as a sorcery." — two clauses of one ability.
//
// The discount is the first: docs/card-dsl.md lists cost reducers under
// "Explicitly not supported yet (M3+)", and an ability printed at the
// un-reduced {4}{R} would charge a price the card does not print, so the
// ability comes off the card rather than onto it wrong.
//
// The token is the second and would block the ability on its own: a
// `TokenDef` literal in a card file has no id in the ledger
// (`no_card_file_defines_its_own_token`), and `crate::tokens` carries no
// Dwarf for it to name.
