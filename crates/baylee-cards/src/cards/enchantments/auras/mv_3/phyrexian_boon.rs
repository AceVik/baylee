//! Phyrexian Boon — {2}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+1 as long as it's black. Otherwise, it gets -1/-2.
//! Set: ME1 #81 — Masters Edition | Scryfall ID: 0dbf0ef0-3e16-4c32-9d51-33fe23bbdc94 | Oracle ID: a18e6468-b8a2-4681-b8fa-223ab5f3061a
// IMPLEMENTED — Aura enchanting a creature, giving +2/+1 while black and -1/-2 otherwise.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_BLACK: Filter = Filter::And(&[
    Filter::AttachedToBySource,
    Filter::HasColor(ColorSet::from_slice(&[Color::Black])),
]);
static ENCHANTED_NONBLACK: Filter = Filter::And(&[
    Filter::AttachedToBySource,
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
]);

card!(
    index = index::PHYREXIAN_BOON,
    oracle_id = "a18e6468-b8a2-4681-b8fa-223ab5f3061a",
    scryfall_id = "0dbf0ef0-3e16-4c32-9d51-33fe23bbdc94",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Phyrexian Boon",
        mana_cost = mana!("{2}{B}"),
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
        static_ability!(ENCHANTED_BLACK, Modifier::ModifyPT(2, 1)),
        static_ability!(ENCHANTED_NONBLACK, Modifier::ModifyPT(-1, -2)),
    ],
);
