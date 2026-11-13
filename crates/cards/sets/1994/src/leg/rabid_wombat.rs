//! Rabid Wombat — {2}{G}{G} — Creature — Wombat
//! Oracle: Vigilance
//! Oracle: This creature gets +2/+2 for each Aura attached to it.
//! Set: ME1 #126 — Masters Edition | Scryfall ID: 604d52c5-ceab-40fd-936e-bab859c4333e | Oracle ID: c886eeb5-f86f-48c8-9adb-af13015972b1
// IMPLEMENTED — vigilance, and +2/+2 for each Aura attached to it (layer 7c
// `Modifier::ModifyPTPerCount` over Auras attached to this creature).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RABID_WOMBAT,
    oracle_id = "c886eeb5-f86f-48c8-9adb-af13015972b1",
    scryfall_id = "604d52c5-ceab-40fd-936e-bab859c4333e",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::VIGILANCE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Rabid Wombat",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::WOMBAT],
        power = Some(0),
        toughness = Some(1),
    ),],
    abilities = &[static_ability!(
        Filter::This,
        Modifier::ModifyPTPerCount {
            filter: &Filter::And(&[
                Filter::HasSubtype(subtypes::enchantment::AURA),
                Filter::AttachedToSource,
            ]),
            p: 2,
            t: 2,
        }
    )],
);
