//! Fable of the Mirror-Breaker // Reflection of Kiki-Jiki — {2}{R} — Enchantment — Saga // Enchantment Creature — Goblin Shaman
//! Oracle: (As this Saga enters and after your draw step, add a lore counter.)
//! Oracle: I — Create a 2/2 red Goblin Shaman creature token with "Whenever this token attacks, create a Treasure token."
//! Oracle: II — You may discard up to two cards. If you do, draw that many cards.
//! Oracle: III — Exile this Saga, then return it to the battlefield transformed under your control.
//! Oracle: {1}, {T}: Create a token that's a copy of another target nonlegendary creature you control, except it has haste. Sacrifice it at the beginning of the next end step.
//! Set: NEO #141 — Kamigawa: Neon Dynasty | Scryfall ID: 24c0d87b-0049-4beb-b9cb-6f813b7aa7dc | Oracle ID: c0957e5e-c71b-439c-931c-9f55d2f76ace
//! Face: Fable of the Mirror-Breaker — {2}{R} — Enchantment — Saga
//! Face: Reflection of Kiki-Jiki —  — Enchantment Creature — Goblin Shaman
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FABLE_OF_THE_MIRROR_BREAKER,
    oracle_id = "c0957e5e-c71b-439c-931c-9f55d2f76ace",
    scryfall_id = "24c0d87b-0049-4beb-b9cb-6f813b7aa7dc",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Fable of the Mirror-Breaker",
            mana_cost = mana!("{2}{R}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::SAGA],
        ),
        face!(
            name = "Reflection of Kiki-Jiki",
            types = TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
            subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::SHAMAN],
            power = Some(2),
            toughness = Some(2),
            castable_from_hand = false,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
