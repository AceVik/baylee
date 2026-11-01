//! Treefolk Mystic — {3}{G} — Creature — Treefolk
//! Oracle: Whenever this creature blocks or becomes blocked by a creature, destroy all Auras attached to that creature.
//! Set: ULG #114 — Urza's Legacy | Scryfall ID: 9697acf5-9bc5-411d-8574-fe6185f18672 | Oracle ID: 68a6fd00-9cb0-4518-97aa-b7f475da58c1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static AURAS_ON_PARTNER: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::Reference),
]);

card!(
    index = index::TREEFOLK_MYSTIC,
    oracle_id = "68a6fd00-9cb0-4518-97aa-b7f475da58c1",
    scryfall_id = "9697acf5-9bc5-411d-8574-fe6185f18672",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Treefolk Mystic",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::TREEFOLK],
        power = Some(2),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE),
        &[Effect::DestroyAll {
            filter: &AURAS_ON_PARTNER,
            no_regen: false,
        }],
    )],
);
