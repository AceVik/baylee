//! Brazen Borrower // Petty Theft — {1}{U}{U} — Creature — Faerie Rogue // Instant — Adventure
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: This creature can block only creatures with flying.
//! Oracle: Return target nonland permanent an opponent controls to its owner's hand.
//! Set: SOC #190 — Secrets of Strixhaven Commander | Scryfall ID: 25d309d6-9e56-441e-bd29-5c903d5221bf | Oracle ID: c7b044c3-3cfa-407e-bf20-2875e8e04b7b
//! Face: Brazen Borrower — {1}{U}{U} — Creature — Faerie Rogue
//! Face: Petty Theft — {1}{U} — Instant — Adventure

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static THEFT_TARGET: Filter = Filter::And(&[Filter::NONLAND, Filter::ControlledByOpponent]);

static PETTY_THEFT: &[AbilityDef] = &[spell!(
    &[Effect::bounce(TargetSpec::Object(&THEFT_TARGET))],
    targets = Some(TargetReq::one(TargetSpec::Object(&THEFT_TARGET)))
)];

card!(
    index = index::BRAZEN_BORROWER,
    oracle_id = "c7b044c3-3cfa-407e-bf20-2875e8e04b7b",
    scryfall_id = "25d309d6-9e56-441e-bd29-5c903d5221bf",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Brazen Borrower",
            mana_cost = mana!("{1}{U}{U}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::FAERIE, subtypes::creature::ROGUE],
            power = Some(3),
            toughness = Some(1),
        ),
        face!(
            name = "Petty Theft",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
            subtypes = &[subtypes::spell::ADVENTURE],
            abilities = PETTY_THEFT,
            adventure = true,
        ),
    ],
    keywords = KeywordSet::FLASH.union(KeywordSet::FLYING),
    coverage = Coverage::Implemented,
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantBlock(&Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)),)
    )],
);
