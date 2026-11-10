//! Imprison — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever a player activates an ability of enchanted creature with {T} in its activation cost that isn't a mana ability, you may pay {1}. If you do, counter that ability. If you don't, destroy this Aura.
//! Oracle: Whenever enchanted creature attacks or blocks, you may pay {1}. If you do, tap the creature, remove it from combat, and creatures it was blocking that had become blocked by only that creature this combat become unblocked. If you don't, destroy this Aura.
//! Set: LEG #107 — Legends | Scryfall ID: 12671381-beb7-41b8-9484-97f8aca5c981 | Oracle ID: 632de66b-2314-4299-847c-16a84bf9121f

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;
static COMBAT_EFFECTS: &[Effect] = &[Effect::PlayerMayPayThenElse {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    paid: &[
        Effect::TapEventObject,
        Effect::RemoveEventObjectFromCombat { unblock: true },
    ],
    unpaid: &[Effect::destroy(TargetSpec::ThisObject)],
}];

card!(
    index = index::IMPRISON,
    oracle_id = "632de66b-2314-4299-847c-16a84bf9121f",
    scryfall_id = "12671381-beb7-41b8-9484-97f8aca5c981",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Imprison",
        mana_cost = mana!("{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::AbilityActivated {
                filter: &Filter::AttachedToBySource,
                who: PlayerRel::EachPlayer,
                tap_cost: Some(true),
                mana_ability: Some(false)
            },
            &[Effect::PlayerMayPayThenElse {
                player: PlayerRel::You,
                mana: Amount::Fixed(1),
                paid: &[Effect::CounterEventAbility],
                unpaid: &[Effect::destroy(TargetSpec::ThisObject)]
            }],
        ),
        triggered!(
            Trigger::Attacks(&Filter::AttachedToBySource),
            COMBAT_EFFECTS
        ),
        triggered!(Trigger::Blocks(&Filter::AttachedToBySource), COMBAT_EFFECTS),
    ],
);
