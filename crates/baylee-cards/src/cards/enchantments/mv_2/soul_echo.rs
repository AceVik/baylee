//! Soul Echo — {X}{W}{W} — Enchantment
//! Oracle: This enchantment enters with X echo counters on it.
//! Oracle: You don't lose the game for having 0 or less life.
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment if there are no echo counters on it. Otherwise, target opponent may choose that for each 1 damage that would be dealt to you until your next upkeep, you remove an echo counter from this enchantment instead.
//! Set: MIR #40 — Mirage | Scryfall ID: cb6f9427-ef5c-49fd-81e1-ddf130d69da0 | Oracle ID: 8bc5546e-554f-430d-80b1-99578b5ea188
// PARTIAL: The upkeep opponent choice to replace each 1 damage dealt to you until your next upkeep with removing an echo counter from Soul Echo is not in the engine; enters with X echo counters, prevents you from losing the game for having 0 or less life, and sacrifices itself at your upkeep when it has no echo counters.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SOUL_ECHO,
    oracle_id = "8bc5546e-554f-430d-80b1-99578b5ea188",
    scryfall_id = "cb6f9427-ef5c-49fd-81e1-ddf130d69da0",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "the upkeep opponent choice to replace each 1 damage dealt to you until your next upkeep with removing an echo counter from Soul Echo is not in the engine; enters with X echo counters, prevents you from losing the game for having 0 or less life, and sacrifices itself at your upkeep when it has no echo counters",
    ),
    faces = &[face!(
        name = "Soul Echo",
        mana_cost = mana!("{X}{W}{W}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::ECHO,
            amount: Amount::X,
        }],
    ),],
    abilities = &[
        static_ability!(
            Filter::Any,
            Modifier::NoLossForZeroLife {
                who: PlayerRel::You,
            },
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::IfCondition {
                condition: Condition::CountersOnSelf(counters::ECHO, 1),
                then: &[],
                otherwise: &[Effect::SacrificeSelf],
            }],
        ),
    ],
);
