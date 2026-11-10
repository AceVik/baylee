//! Spreading Algae — {G} — Enchantment — Aura
//! Oracle: Enchant Swamp
//! Oracle: When enchanted land becomes tapped, destroy it.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: USG #274 — Urza's Saga | Scryfall ID: 20cfc00e-180c-4277-8911-43afa691b9d7 | Oracle ID: a08189f2-1643-496a-8a91-74864528b4db
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static SWAMP: Filter = Filter::And(&[Filter::LAND, Filter::HasSubtype(subtypes::land::SWAMP)]);

card!(
    index = index::SPREADING_ALGAE,
    oracle_id = "a08189f2-1643-496a-8a91-74864528b4db",
    scryfall_id = "20cfc00e-180c-4277-8911-43afa691b9d7",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spreading Algae",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&SWAMP),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&SWAMP))),
        ),
        triggered!(
            Trigger::BecomesTapped(&Filter::AttachedToBySource),
            &[Effect::destroy(TargetSpec::EventObject)],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::EventObject,
            }],
        ),
    ],
);
