//! Tangle Kelp — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, tap enchanted creature.
//! Oracle: Enchanted creature doesn't untap during its controller's untap step if it attacked during its controller's last turn.
//! Set: DRK #37 — The Dark | Scryfall ID: 497bc42d-ab81-4d84-bdf7-e3a05a7c984d | Oracle ID: 06ba313f-50b4-4e84-a8e0-b6dda47225f2

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTED_ATTACKED_LAST_TURN: Filter = Filter::And(&[
    Filter::AttachedToBySource,
    Filter::AttackedDuringControllersLastTurn,
]);

card!(
    index = index::TANGLE_KELP,
    oracle_id = "06ba313f-50b4-4e84-a8e0-b6dda47225f2",
    scryfall_id = "497bc42d-ab81-4d84-bdf7-e3a05a7c984d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tangle Kelp",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::ETB,
            &[Effect::TapAll {
                filter: &Filter::AttachedToBySource,
            }]
        ),
        static_ability!(ENCHANTED_ATTACKED_LAST_TURN, Modifier::DoesNotUntap),
    ],
);
