//! Muldrotha, the Gravetide — {3}{B}{G}{U} — Legendary Creature — Elemental Avatar
//! Oracle: During each of your turns, you may play a land and cast a permanent spell of each permanent type from your graveyard. (If a card has multiple permanent types, choose one as you play it.)
//! Set: ECC #128 — Lorwyn Eclipsed Commander | Scryfall ID: 705b4d97-2f50-47f7-9053-d748f4337553 | Oracle ID: e4625704-1d52-44e4-804f-2f45644d76ac
// IMPLEMENTED — the graveyard land play (`Modifier::PlayLandsFromGraveyard`,
// which CR 305.2 already limits to your own turn); the cast half is NOT
// SUPPORTED below, so the card is `Coverage::Partial`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MULDROTHA_THE_GRAVETIDE,
    oracle_id = "e4625704-1d52-44e4-804f-2f45644d76ac",
    scryfall_id = "705b4d97-2f50-47f7-9053-d748f4337553",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "the \"cast a permanent spell of each permanent type from your graveyard\" half: \
         no Modifier grants casting out of a graveyard — Modifier::GrantsFlashback is \
         per-card, a spell permission and exiles the card afterwards — and nothing \
         carries the once-per-type-per-turn tally"
    ),
    faces = &[face!(
        name = "Muldrotha, the Gravetide",
        mana_cost = mana!("{3}{B}{G}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::AVATAR],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[
        // NOT SUPPORTED: "cast a permanent spell of each permanent type from
        // your graveyard" — the DSL has no Modifier for a casting permission
        // out of a graveyard, and none for the one-per-permanent-type tally
        // the clause needs. Only the land half is expressible, and it is a
        // player-scoped rule, so it takes `Filter::Any` like Crucible of
        // Worlds' (CR 305.2 keeps it to your own turn on its own).
        static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard),
    ],
);
