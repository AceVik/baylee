//! Akroma, Vision of Ixidor — {5}{W}{W} — Legendary Creature — Angel
//! Oracle: Flying, first strike, vigilance, trample
//! Oracle: At the beginning of each combat, until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner.
//! Oracle: Partner
//! Set: CMR #2 — Commander Legends | Scryfall ID: c0bdd07d-9fb9-4ee4-80db-494ee0cba58d | Oracle ID: b9be5ce9-08bb-4361-940c-9031e429ba86
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const AKROMA_COUNTED_KEYWORDS: KeywordSet = KeywordSet::FLYING
    .union(KeywordSet::FIRST_STRIKE)
    .union(KeywordSet::DOUBLE_STRIKE)
    .union(KeywordSet::DEATHTOUCH)
    .union(KeywordSet::HASTE)
    .union(KeywordSet::HEXPROOF)
    .union(KeywordSet::INDESTRUCTIBLE)
    .union(KeywordSet::LIFELINK)
    .union(KeywordSet::MENACE)
    .union(KeywordSet::REACH)
    .union(KeywordSet::TRAMPLE)
    .union(KeywordSet::VIGILANCE)
    .union(KeywordSet::PARTNER);

card!(
    index = index::AKROMA_VISION_OF_IXIDOR,
    oracle_id = "b9be5ce9-08bb-4361-940c-9031e429ba86",
    scryfall_id = "c0bdd07d-9fb9-4ee4-80db-494ee0cba58d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    partner = PartnerKind::Partner,
    keywords = KeywordSet::FLYING
        .union(KeywordSet::FIRST_STRIKE)
        .union(KeywordSet::VIGILANCE)
        .union(KeywordSet::TRAMPLE),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Akroma, Vision of Ixidor",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(6),
        toughness = Some(6),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::EachPlayer,
        },
        &[Effect::PumpFilterPerKeyword {
            filter: &Filter::ANOTHER_CREATURE_YOU_CONTROL,
            keywords: AKROMA_COUNTED_KEYWORDS,
            count_protection: true,
            duration: Duration::UntilEndOfTurn,
        }]
    )],
);
