//! Brad Boimler, Eager Ensign — {1}{W} — Legendary Creature — Human Officer
//! Oracle: Lifelink
//! Oracle: Whenever Brad Boimler becomes tapped, until end of turn, if one or more counters would be put on a permanent you control, that many plus one of each of those kinds of counters are put on that permanent instead.
//! Set: TRK #5 — Star Trek | Scryfall ID: a6bf1525-2212-46d0-ad4d-1dbaa2e3b3cd | Oracle ID: 10af9cd9-1700-48f9-97e1-61e239536fef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRAD_BOIMLER_EAGER_ENSIGN,
    oracle_id = "10af9cd9-1700-48f9-97e1-61e239536fef",
    scryfall_id = "a6bf1525-2212-46d0-ad4d-1dbaa2e3b3cd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Brad Boimler, Eager Ensign",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::OFFICER],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
