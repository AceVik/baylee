//! Telim'Tor — {4}{R} — Legendary Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: Whenever Telim'Tor attacks, all attacking creatures with flanking get +1/+1 until end of turn.
//! Set: MIR #197 — Mirage | Scryfall ID: 33cc89c8-a3ea-469e-b499-f48acec4f538 | Oracle ID: b13a723d-80a9-4116-9857-baf3b373f87a
// IMPLEMENTED — flanking and attack trigger pumping all attacking creatures with flanking +1/+1 until end of turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACKING_WITH_FLANKING: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::HasKeyword(KeywordSet::FLANKING),
]);

card!(
    index = index::TELIM_TOR,
    oracle_id = "b13a723d-80a9-4116-9857-baf3b373f87a",
    scryfall_id = "33cc89c8-a3ea-469e-b499-f48acec4f538",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::FLANKING,
    commander = CommanderRule::Legendary,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Telim'Tor",
        mana_cost = mana!("{4}{R}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
        keywords = KeywordSet::FLANKING,
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::PumpFilter {
            filter: &ATTACKING_WITH_FLANKING,
            controlled_by: None,
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(1),
            keywords: KeywordSet::EMPTY,
            duration: Duration::UntilEndOfTurn,
        }],
    ),],
);
