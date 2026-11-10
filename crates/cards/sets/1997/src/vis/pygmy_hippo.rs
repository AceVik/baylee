//! Pygmy Hippo — {G}{U} — Creature — Hippo
//! Oracle: Whenever this creature attacks and isn't blocked, you may have defending player activate a mana ability of each land they control and lose all unspent mana. If you do, this creature assigns no combat damage this turn and at the beginning of your next main phase this turn, you add an amount of {C} equal to the amount of mana that player lost this way.
//! Set: VIS #133 — Visions | Scryfall ID: 2e3f6220-6ead-46b4-8663-57609ef5a12e | Oracle ID: 0d39513a-1f05-4d9a-a772-745f4f1a3471
// PARTIAL: 2/2 Hippo; forcing defending player to tap their lands for mana and adding that much {C} at the beginning of your next main phase is not yet supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Whenever this creature attacks and isn't blocked, you may have defending player activate a mana ability of each land they control and lose all unspent mana. If you do, this creature assigns no combat damage this turn and at the beginning of your next main phase this turn, you add an amount of {C} equal to the amount of mana that player lost this way.
card!(
    index = index::PYGMY_HIPPO,
    oracle_id = "0d39513a-1f05-4d9a-a772-745f4f1a3471",
    scryfall_id = "2e3f6220-6ead-46b4-8663-57609ef5a12e",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    coverage = Coverage::Partial(
        "2/2 Hippo; forcing defending player to tap their lands for mana and adding that much {C} at the beginning of your next main phase is not yet supported"
    ),
    faces = &[face!(
        name = "Pygmy Hippo",
        mana_cost = mana!("{G}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HIPPO],
        power = Some(2),
        toughness = Some(2),
    ),],
);
