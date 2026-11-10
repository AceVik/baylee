//! Hipparion — {1}{W} — Creature — Horse
//! Oracle: This creature can't block creatures with power 3 or greater unless you pay {1}.
//! Set: 5ED #34 — Fifth Edition | Scryfall ID: f26fc403-b8af-499a-8cbc-4077ff97b8b0 | Oracle ID: 306fd9d2-5ded-4857-90f4-e62d73c6aed2
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HIPPARION,
    oracle_id = "306fd9d2-5ded-4857-90f4-e62d73c6aed2",
    scryfall_id = "f26fc403-b8af-499a-8cbc-4077ff97b8b0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Hipparion",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORSE],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::CantBlockMatchingUnlessPayMana {
            attacker: &Filter::PowerAtLeast(3),
            mana: 1,
        }
    )],
);
