//! Sakashima of a Thousand Faces — {3}{U} — Legendary Creature — Human Rogue
//! Oracle: You may have Sakashima enter as a copy of another creature you control, except it has Sakashima's other abilities.
//! Oracle: The "legend rule" doesn't apply to permanents you control.
//! Oracle: Partner (You can have two commanders if both have partner.)
//! Set: CMR #89 — Commander Legends | Scryfall ID: 714c3a1f-7b30-4ed8-8f38-6176758741fb | Oracle ID: 8ecdaf4b-4442-42da-9714-4257a83faf50
// IMPLEMENTED — clone of your creatures + legend rule suppression + partner.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::SAKASHIMA_OF_A_THOUSAND_FACES,
    oracle_id = "8ecdaf4b-4442-42da-9714-4257a83faf50",
    scryfall_id = "714c3a1f-7b30-4ed8-8f38-6176758741fb",
    faces = &[face!(
        name = "Sakashima of a Thousand Faces",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[creature::HUMAN, creature::ROGUE],
        power = Some(3),
        toughness = Some(1),
    )],
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    commander = CommanderRule::Legendary,
    partner = PartnerKind::Partner,
    coverage = Coverage::Implemented,
    abilities = &[
        AbilityDef::CopyOnEnter {
            target: TargetSpec::Object(&Filter::ANOTHER_CREATURE_YOU_CONTROL),
            mods: &[],
        },
        static_ability!(Filter::Any, Modifier::LegendRuleOff),
    ],
);
