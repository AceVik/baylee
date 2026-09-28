//! Kiki-Jiki, Mirror Breaker — {2}{R}{R}{R} — Legendary Creature — Goblin Shaman
//! Oracle: Haste
//! Oracle: {T}: Create a token that's a copy of target nonlegendary creature you control, except it has haste. Sacrifice it at the beginning of the next end step.
//! Set: IMA #136 — Iconic Masters | Scryfall ID: a2ff0ee3-9600-4c7d-acec-6ec90595384e | Oracle ID: a34b7416-cfe3-4a1e-a8c1-a3056b747519
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KIKI_JIKI_MIRROR_BREAKER,
    oracle_id = "a34b7416-cfe3-4a1e-a8c1-a3056b747519",
    scryfall_id = "a2ff0ee3-9600-4c7d-acec-6ec90595384e",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Kiki-Jiki, Mirror Breaker",
        mana_cost = mana!("{2}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GOBLIN, subtypes::creature::SHAMAN],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
