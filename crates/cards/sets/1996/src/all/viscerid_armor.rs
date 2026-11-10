//! Viscerid Armor — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1.
//! Oracle: {1}{U}: Return this Aura to its owner's hand.
//! Set: ME2 #72 — Masters Edition II | Scryfall ID: a9906f76-3b14-4cf4-90c1-b9bbb594256b | Oracle ID: 87b4cbec-3521-4bdb-bf01-64ddb526b655
// IMPLEMENTED — Enchant creature; enchanted creature gets +1/+1; {1}{U}: return this Aura to its owner's hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VISCERID_ARMOR,
    oracle_id = "87b4cbec-3521-4bdb-bf01-64ddb526b655",
    scryfall_id = "a9906f76-3b14-4cf4-90c1-b9bbb594256b",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Viscerid Armor",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(1, 1),
        ),
        activated!(
            cost!("{1}{U}"),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
