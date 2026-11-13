//! Mana Vortex — {1}{U}{U} — Enchantment
//! Oracle: When you cast this spell, counter it unless you sacrifice a land.
//! Oracle: At the beginning of each player's upkeep, that player sacrifices a land of their choice.
//! Oracle: When there are no lands on the battlefield, sacrifice this enchantment.
//! Set: ME3 #44 — Masters Edition III | Scryfall ID: b9d4d7da-f98a-4622-91b8-3460fe44551a | Oracle ID: 98b88734-09e2-4209-a787-9b5b345391f9
// IMPLEMENTED — cast trigger counters this spell unless you sacrifice a land; upkeep trigger makes each active player sacrifice a land; state trigger sacrifices this when no lands are on the battlefield.

use baylee_cards_dsl::prelude::*;

static NO_LANDS: Condition = Condition::BattlefieldCountAtMost(&Filter::LAND, 0);

card!(
    index = index::MANA_VORTEX,
    oracle_id = "98b88734-09e2-4209-a787-9b5b345391f9",
    scryfall_id = "b9d4d7da-f98a-4622-91b8-3460fe44551a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mana Vortex",
        mana_cost = mana!("{1}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[
        triggered!(
            Trigger::SpellCast(&Filter::This),
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::Sacrifice(&Filter::LAND),
                effect: &Effect::CounterTargetSpell,
            }],
            targets = Some(TargetReq::one(TargetSpec::EventObject)),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::EachPlayer,
            },
            &[Effect::SacrificeFilter {
                who: PlayerRel::ActivePlayer,
                filter: &Filter::LAND,
            }],
        ),
        triggered!(Trigger::State(&NO_LANDS), &[Effect::SacrificeSelf]),
    ],
);
