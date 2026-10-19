//! Erosion — {U}{U}{U} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: At the beginning of the upkeep of enchanted land's controller, destroy that land unless that player pays {1} or 1 life.
//! Set: 4ED #70 — Fourth Edition | Scryfall ID: 8dc539ff-03f5-4bec-877a-061fb5d40de9 | Oracle ID: 155ee24f-a458-4df4-86d5-5eb7396ebf45
// IMPLEMENTED — enchant land; at the beginning of enchanted land's controller's upkeep, destroy that land unless that player pays {1} or 1 life.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DESTROY_ATTACHED: Effect = Effect::DestroyAll {
    filter: &Filter::AttachedToBySource,
    no_regen: false,
};

static PAY_LIFE_OR_DESTROY: Effect = Effect::PlayerMayPayLifeOr {
    player: PlayerRel::ControllerOfAttached,
    life: Amount::Fixed(1),
    effect: &DESTROY_ATTACHED,
};

card!(
    index = index::EROSION,
    oracle_id = "155ee24f-a458-4df4-86d5-5eb7396ebf45",
    scryfall_id = "8dc539ff-03f5-4bec-877a-061fb5d40de9",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Erosion",
        mana_cost = mana!("{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfAttached,
                mana: Amount::Fixed(1),
                effect: &PAY_LIFE_OR_DESTROY,
            }],
        ),
    ],
);
