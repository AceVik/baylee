//! Sleeper's Guile — {2}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has fear. (It can't be blocked except by artifact creatures and/or black creatures.)
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: ULG #67 — Urza's Legacy | Scryfall ID: a001ca83-35b5-48e5-8337-92258d5affc2 | Oracle ID: eaaa3e41-dbf6-43ae-b65e-e1502ba4fa0b
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SLEEPER_S_GUILE,
    oracle_id = "eaaa3e41-dbf6-43ae-b65e-e1502ba4fa0b",
    scryfall_id = "a001ca83-35b5-48e5-8337-92258d5affc2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sleeper's Guile",
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
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::FEAR),
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
