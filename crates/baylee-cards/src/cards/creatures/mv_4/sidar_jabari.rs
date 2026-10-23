//! Sidar Jabari — {3}{W} — Legendary Creature — Human Knight
//! Oracle: Flanking (Whenever a creature without flanking blocks this creature, the blocking creature gets -1/-1 until end of turn.)
//! Oracle: Whenever Sidar Jabari attacks, tap target creature defending player controls.
//! Set: VMA #45 — Vintage Masters | Scryfall ID: c6d7c16b-27f2-4608-bb79-c2edb86abdef | Oracle ID: cb637dfd-4022-43a4-8632-77e614436f38

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DEFENDING_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::SIDAR_JABARI,
    oracle_id = "cb637dfd-4022-43a4-8632-77e614436f38",
    scryfall_id = "c6d7c16b-27f2-4608-bb79-c2edb86abdef",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLANKING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sidar Jabari",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::TapTarget],
        targets = Some(TargetReq::one(TargetSpec::Object(&DEFENDING_CREATURE))),
    ),],
);
