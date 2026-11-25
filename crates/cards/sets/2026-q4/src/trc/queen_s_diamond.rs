//! Queen's Diamond — {4} — Legendary Artifact — Spacecraft
//! Oracle: {T}: Choose a color. Add one mana of that color for each charge counter on Queen's Diamond. Spend this mana only to cast your commander.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)
//! Oracle: 8+ | Flying, vigilance, haste
//! Set: TRC #118 — Star Trek Commander | Scryfall ID: 5f936935-4f47-4d33-871b-0159cb6fb4da | Oracle ID: 44837c9b-971a-44b6-8e55-301cebc3a361
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::QUEEN_S_DIAMOND,
    oracle_id = "44837c9b-971a-44b6-8e55-301cebc3a361",
    scryfall_id = "5f936935-4f47-4d33-871b-0159cb6fb4da",
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Queen's Diamond",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
