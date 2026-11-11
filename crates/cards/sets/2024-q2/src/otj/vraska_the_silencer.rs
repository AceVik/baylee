//! Vraska, the Silencer — {1}{B}{G} — Legendary Creature — Gorgon Assassin
//! Oracle: Deathtouch
//! Oracle: Whenever a nontoken creature an opponent controls dies, you may pay {1}. If you do, return that card to the battlefield tapped under your control. It's a Treasure artifact with "{T}, Sacrifice this artifact: Add one mana of any color," and it loses all other card types.
//! Set: OTJ #237 — Outlaws of Thunder Junction | Scryfall ID: b042abf2-c40b-4235-a4fa-2e4901c375c3 | Oracle ID: e7c5a568-3855-4a20-9edf-01a8a6a4ba97
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VRASKA_THE_SILENCER,
    oracle_id = "e7c5a568-3855-4a20-9edf-01a8a6a4ba97",
    scryfall_id = "b042abf2-c40b-4235-a4fa-2e4901c375c3",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Vraska, the Silencer",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::GORGON, subtypes::creature::ASSASSIN],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
