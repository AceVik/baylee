//! Farmstead — {W}{W}{W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has "At the beginning of your upkeep, you may pay {W}{W}. If you do, you gain 1 life."
//! Set: SUM #19 — Summer Magic / Edgar | Scryfall ID: 8cd5732c-cd54-48d8-8b32-f33782ec69da | Oracle ID: 0d71b157-09a0-4fd4-beb9-103117a784ad

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FARMSTEAD,
    oracle_id = "0d71b157-09a0-4fd4-beb9-103117a784ad",
    scryfall_id = "8cd5732c-cd54-48d8-8b32-f33782ec69da",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Farmstead",
        mana_cost = mana!("{W}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
        ),
        // The land has the ability, so "your" upkeep and "you" are its
        // controller's, and the price is the "may": one question, paid in
        // white (CR 118.12).
        static_ability!(
            Filter::And(&[Filter::LAND, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::StepBegin {
                    step: StepKind::Upkeep,
                    whose: PlayerRel::You,
                },
                effects: &[Effect::PlayerMayPayManaThen {
                    player: PlayerRel::You,
                    cost: mana!("{W}{W}"),
                    effects: &[Effect::gain_life(1)],
                }],
                target: None,
            }
        ),
    ],
);
