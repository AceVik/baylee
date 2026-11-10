//! Brad Boimler, Eager Ensign — {1}{W} — Legendary Creature — Human Officer
//! Oracle: Lifelink
//! Oracle: Whenever Brad Boimler becomes tapped, until end of turn, if one or more counters would be put on a permanent you control, that many plus one of each of those kinds of counters are put on that permanent instead.
//! Set: TRK #5 — Star Trek | Scryfall ID: a6bf1525-2212-46d0-ad4d-1dbaa2e3b3cd | Oracle ID: 10af9cd9-1700-48f9-97e1-61e239536fef
// IMPLEMENTED — lifelink and a separate temporary additive counter-placement
// replacement for each tap trigger, ordered with other counter replacements.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRAD_BOIMLER_EAGER_ENSIGN,
    oracle_id = "10af9cd9-1700-48f9-97e1-61e239536fef",
    scryfall_id = "a6bf1525-2212-46d0-ad4d-1dbaa2e3b3cd",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::LIFELINK,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brad Boimler, Eager Ensign",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::OFFICER],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesTapped(&Filter::This),
        &[Effect::continuous(
            &Filter::And(&[
                Filter::InZone(ZoneRef::Battlefield),
                Filter::ControlledByYou
            ]),
            Modifier::AdditionalCounterPlacement { n: 1 },
            Duration::UntilEndOfTurn,
        )],
    )],
);
