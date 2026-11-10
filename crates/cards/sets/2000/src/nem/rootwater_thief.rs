//! Rootwater Thief — {1}{U} — Creature — Merfolk Rogue
//! Oracle: {U}: This creature gains flying until end of turn.
//! Oracle: Whenever this creature deals combat damage to a player, you may pay {2}. If you do, search that player's library for a card and exile it, then the player shuffles.
//! Set: NEM #40 — Nemesis | Scryfall ID: 38addef3-1dd7-41a1-9706-3be5c86a58c9 | Oracle ID: 68abaca6-a86a-46e3-b5b8-9860bb6425d2
// NOT SUPPORTED: Whenever this creature deals combat damage to a player, you may pay {2} to search that player's library for a card and exile it is not in the DSL
// PARTIAL — {1}{U} 1/2 Merfolk Rogue with "{U}: This creature gains flying until end of turn" implemented; combat-damage trigger to pay {2} and search that player's library to exile a card is not in the DSL.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ROOTWATER_THIEF,
    oracle_id = "68abaca6-a86a-46e3-b5b8-9860bb6425d2",
    scryfall_id = "38addef3-1dd7-41a1-9706-3be5c86a58c9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Whenever this creature deals combat damage to a player, you may pay {2} to search that player's library for a card and exile it is not in the DSL",
    ),
    faces = &[face!(
        name = "Rootwater Thief",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{U}"),
        &[Effect::PumpFilter {
            filter: &Filter::This,
            controlled_by: None,
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::FLYING,
            duration: Duration::UntilEndOfTurn,
        }],
    ),],
);
