//! Rancor — {G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+0 and has trample.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: 2X2 #156 — Double Masters 2022 | Scryfall ID: 86d6b411-4a31-4bfc-8dd6-e19f553bb29b | Oracle ID: 9d2d6479-531c-4ce1-b52b-00e36fa63b64
// IMPLEMENTED — enchant creature (attach clause), +2/+0 and trample to the
// enchanted creature, and the dies trigger returning the Aura to its hand.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RANCOR,
    oracle_id = "9d2d6479-531c-4ce1-b52b-00e36fa63b64",
    scryfall_id = "86d6b411-4a31-4bfc-8dd6-e19f553bb29b",
    coverage = Coverage::Implemented,
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rancor",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        // "Enchant creature" is one spell!, and has to be: the engine reads
        // the Aura's continuing legality out of this effect's target spec.
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 0)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::TRAMPLE)
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }]
        ),
    ],
);
