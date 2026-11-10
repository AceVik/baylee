//! Fiery Mantle — {1}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: {R}: Enchanted creature gets +1/+0 until end of turn.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: USG #186 — Urza's Saga | Scryfall ID: c41deea2-c12f-40cd-8fc4-47227c762f42 | Oracle ID: 1b553ed4-12eb-4a53-8e1f-d47ec92b1d65
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FIERY_MANTLE,
    oracle_id = "1b553ed4-12eb-4a53-8e1f-d47ec92b1d65",
    scryfall_id = "c41deea2-c12f-40cd-8fc4-47227c762f42",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Fiery Mantle",
        mana_cost = mana!("{1}{R}"),
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
        activated!(
            cost!("{R}"),
            &[Effect::ModifyAttached {
                modifier: Modifier::ModifyPT(1, 0),
                duration: Duration::UntilEndOfTurn,
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
