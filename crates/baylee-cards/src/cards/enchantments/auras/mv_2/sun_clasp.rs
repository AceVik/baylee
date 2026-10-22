//! Sun Clasp — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+3.
//! Oracle: {W}: Return enchanted creature to its owner's hand.
//! Set: DMR #30 — Dominaria Remastered | Scryfall ID: 49e33908-cf4f-45b4-8d97-a117308187f7 | Oracle ID: cb8f5ee4-285a-4c37-adcb-514b3e4bdad3
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]);

card!(
    index = index::SUN_CLASP,
    oracle_id = "cb8f5ee4-285a-4c37-adcb-514b3e4bdad3",
    scryfall_id = "49e33908-cf4f-45b4-8d97-a117308187f7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sun Clasp",
        mana_cost = mana!("{1}{W}"),
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
        static_ability!(ENCHANTED_CREATURE, Modifier::ModifyPT(1, 3)),
        activated!(
            cost!("{W}"),
            &[Effect::ReturnAllToHand {
                filter: &ENCHANTED_CREATURE,
                opponents_only: false,
            }],
        ),
    ],
);
