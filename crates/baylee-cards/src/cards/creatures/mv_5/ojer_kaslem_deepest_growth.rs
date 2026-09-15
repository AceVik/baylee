//! Ojer Kaslem, Deepest Growth // Temple of Cultivation — {3}{G}{G} — Legendary Creature — God // Land
//! Oracle: Trample
//! Oracle: Whenever Ojer Kaslem deals combat damage to a player, reveal that many cards from the top of your library. You may put a creature card and/or a land card from among them onto the battlefield. Put the rest on the bottom in a random order.
//! Oracle: When Ojer Kaslem dies, return it to the battlefield tapped and transformed under its owner's control.
//! Oracle: (Transforms from Ojer Kaslem, Deepest Growth.)
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}, {T}: Transform this land. Activate only if you control ten or more permanents and only as a sorcery.
//! Set: LCI #204 — The Lost Caverns of Ixalan | Scryfall ID: 0cbc43a3-8cba-4988-9de1-c89aedd79ada | Oracle ID: eda11077-b2ce-408b-b982-def2da8fe599
//! Face: Ojer Kaslem, Deepest Growth — {3}{G}{G} — Legendary Creature — God
//! Face: Temple of Cultivation —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = 27011,
    oracle_id = "eda11077-b2ce-408b-b982-def2da8fe599",
    scryfall_id = "0cbc43a3-8cba-4988-9de1-c89aedd79ada",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Ojer Kaslem, Deepest Growth",
            mana_cost = mana!("{3}{G}{G}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::GOD],
            power = Some(6),
            toughness = Some(5),
        ),
        face!(name = "Temple of Cultivation", types = TypeSet::LAND,),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
