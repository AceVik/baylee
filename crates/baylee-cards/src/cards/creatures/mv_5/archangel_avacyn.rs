//! Archangel Avacyn // Avacyn, the Purifier — {3}{W}{W} — Legendary Creature — Angel // Legendary Creature — Angel
//! Oracle: Flash
//! Oracle: Flying, vigilance
//! Oracle: When Archangel Avacyn enters, creatures you control gain indestructible until end of turn.
//! Oracle: When a non-Angel creature you control dies, transform Archangel Avacyn at the beginning of the next upkeep.
//! Oracle: Flying
//! Oracle: When this creature transforms into Avacyn, the Purifier, it deals 3 damage to each other creature and each opponent.
//! Set: INR #11 — Innistrad Remastered | Scryfall ID: fe675f03-cbb5-4177-b7d7-64a30260ee2a | Oracle ID: 432b37a5-d32a-4b78-91ab-860aa026b7cc
//! Face: Archangel Avacyn — {3}{W}{W} — Legendary Creature — Angel
//! Face: Avacyn, the Purifier —  — Legendary Creature — Angel
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARCHANGEL_AVACYN,
    oracle_id = "432b37a5-d32a-4b78-91ab-860aa026b7cc",
    scryfall_id = "fe675f03-cbb5-4177-b7d7-64a30260ee2a",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[
        face!(
            name = "Archangel Avacyn",
            mana_cost = mana!("{3}{W}{W}"),
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::ANGEL],
            power = Some(4),
            toughness = Some(4),
        ),
        face!(
            name = "Avacyn, the Purifier",
            types = TypeSet::CREATURE,
            supertypes = SupertypeSet::LEGENDARY,
            subtypes = &[subtypes::creature::ANGEL],
            power = Some(6),
            toughness = Some(5),
            castable_from_hand = false,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
