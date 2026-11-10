//! Sephara, Sky's Blade — {4}{W}{W}{W} — Legendary Creature — Angel
//! Oracle: You may pay {W} and tap four untapped creatures you control with flying rather than pay this spell's mana cost.
//! Oracle: Flying, lifelink
//! Oracle: Other creatures you control with flying have indestructible. (Damage and effects that say "destroy" don't destroy them.)
//! Set: CMM #54 — Commander Masters | Scryfall ID: 729ee848-6c27-4202-b2bc-605f45209468 | Oracle ID: ac8952c9-bd30-48cd-b964-7e6271f9cbc8
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const UNTAPPED_FLYING_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FLYING),
    Filter::Untapped,
    Filter::ControlledByYou,
]);

const OTHER_FLYING_CREATURE_YOU_CONTROL: Filter = Filter::And(&[
    Filter::Another,
    Filter::CREATURE,
    Filter::HasKeyword(KeywordSet::FLYING),
    Filter::ControlledByYou,
]);

card!(
    index = index::SEPHARA_SKY_S_BLADE,
    oracle_id = "ac8952c9-bd30-48cd-b964-7e6271f9cbc8",
    scryfall_id = "729ee848-6c27-4202-b2bc-605f45209468",
    color_identity = ColorSet::from_slice(&[Color::White]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::FLYING.union(KeywordSet::LIFELINK),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sephara, Sky's Blade",
        mana_cost = mana!("{4}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(7),
        toughness = Some(7),
        alternative_costs = &[AlternativeCost {
            cost: cost!(
                "{W}",
                TapN {
                    filter: &UNTAPPED_FLYING_CREATURE_YOU_CONTROL,
                    count: 4,
                }
            ),
            condition: AltCondition::Always,
        }],
    ),],
    abilities = &[static_ability!(
        OTHER_FLYING_CREATURE_YOU_CONTROL,
        Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE)
    )],
);
