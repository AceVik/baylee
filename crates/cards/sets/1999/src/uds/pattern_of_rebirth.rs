//! Pattern of Rebirth — {3}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature dies, that creature's controller may search their library for a creature card, put that card onto the battlefield, then shuffle.
//! Set: UMA #176 — Ultimate Masters | Scryfall ID: c0e34219-9fd9-4f11-a244-670fb75ef938 | Oracle ID: 4b8856f8-3cd1-450c-a71b-eb0026e8e179
// IMPLEMENTED — enchants a creature; when enchanted creature dies, that creature's controller may search their library for a creature card, put it onto the battlefield under their control, then shuffle.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PATTERN_OF_REBIRTH,
    oracle_id = "4b8856f8-3cd1-450c-a71b-eb0026e8e179",
    scryfall_id = "c0e34219-9fd9-4f11-a244-670fb75ef938",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pattern of Rebirth",
        mana_cost = mana!("{3}{G}"),
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
        triggered!(
            Trigger::Dies(&Filter::AttachedToBySource),
            &[Effect::MayDoFor {
                player: PlayerRel::ControllerOfEvent,
                effects: &[Effect::SearchLibraryOf {
                    library: PlayerRel::ControllerOfEvent,
                    owner_searches: true,
                    filter: &Filter::CREATURE,
                    mana_value: None,
                    finds: &[Find::BATTLEFIELD],
                    optional: true,
                }],
            }],
        ),
    ],
);
