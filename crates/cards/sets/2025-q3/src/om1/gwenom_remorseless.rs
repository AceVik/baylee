//! Gwenom, Remorseless — {3}{B}{B} — Legendary Creature — Symbiote Spider Hero
//! Oracle: Deathtouch, lifelink
//! Oracle: Whenever Gwenom attacks, until end of turn, you may look at the top card of your library any time and you may play cards from the top of your library. If you cast a spell this way, pay life equal to its mana value rather than pay its mana cost.
//! Set: SPM #56 — Marvel's Spider-Man | Scryfall ID: 46b6cc5d-7a37-4e8b-a1a5-9a573056610c | Oracle ID: c48cf135-b280-4e9f-a9d9-225104870741
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GWENOM_REMORSELESS,
    oracle_id = "c48cf135-b280-4e9f-a9d9-225104870741",
    scryfall_id = "46b6cc5d-7a37-4e8b-a1a5-9a573056610c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Gwenom, Remorseless",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[
            subtypes::creature::SYMBIOTE,
            subtypes::creature::SPIDER,
            subtypes::creature::HERO
        ],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
