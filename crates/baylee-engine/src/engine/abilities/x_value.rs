//! Announced X: printed constraints and the intersection of payment resources.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::ActivationX;

impl<L: CardLookup> Engine<L> {
    pub(super) fn activation_x_rule(&self, source: ObjectId, index: u32) -> ActivationX {
        self.activation_definition(source, index).map_or(
            ActivationX::Any,
            |ability| match ability {
                AbilityDef::Activated { x_value, .. }
                | AbilityDef::ModalActivated { x_value, .. }
                | AbilityDef::ActivatedConditional { x_value, .. } => x_value,
                _ => ActivationX::Any,
            },
        )
    }

    pub(super) fn activation_x_minimum(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: u32,
    ) -> u32 {
        match self.activation_x_rule(source, index) {
            ActivationX::Any => 0,
            ActivationX::AtLeast(n) => n,
            ActivationX::Defined(amount) => eval::amount_with_context(
                &amount,
                &self.state,
                player,
                self.ability_rule_context(source, index),
                None,
            ),
        }
    }

    /// Fixed payments are reserved first; repeated variable payments share X.
    /// Intersect bounds rather than choosing one resource and ignoring the others.
    pub(super) fn activation_resource_x_max(
        &self,
        player: PlayerId,
        source: ObjectId,
        price: &ActivationPrice,
    ) -> Option<u32> {
        let mut bound = None;
        for part in &price.parts {
            let (held, fixed, variables) = match part {
                CostPart::SacrificeX(filter) | CostPart::TapX(filter) => (
                    u32::try_from(
                        cost_wizard::menu(&self.state, player, source, &price.parts, part).len(),
                    )
                    .unwrap_or(u32::MAX),
                    price
                        .parts
                        .iter()
                        .filter(|other| match (part, other) {
                            (CostPart::SacrificeX(_), CostPart::Sacrifice(other))
                            | (CostPart::TapX(_), CostPart::TapOther(other)) => other == filter,
                            _ => false,
                        })
                        .count()
                        .try_into()
                        .unwrap_or(u32::MAX),
                    price.parts.iter().filter(|p| *p == part).count(),
                ),
                CostPart::PayEnergyX => (
                    u32::from(self.state.players[usize::from(player.get())].energy),
                    energy_payment(&price.parts, 0),
                    price
                        .parts
                        .iter()
                        .filter(|p| matches!(p, CostPart::PayEnergyX))
                        .count(),
                ),
                CostPart::RemoveCounterSelfX { kind } => (
                    self.state
                        .object(source)
                        .map_or(0, |o| u32::from(o.counters.get(*kind))),
                    price
                        .parts
                        .iter()
                        .map(|p| match p {
                            CostPart::RemoveCounterSelf { kind: k, n } if k == kind => {
                                u32::from(*n)
                            }
                            _ => 0,
                        })
                        .sum::<u32>(),
                    price
                        .parts
                        .iter()
                        .filter(
                            |p| matches!(p, CostPart::RemoveCounterSelfX { kind: k } if k == kind),
                        )
                        .count(),
                ),
                _ => continue,
            };
            let max = held.saturating_sub(fixed) / u32::try_from(variables).unwrap_or(u32::MAX);
            bound = Some(bound.map_or(max, |previous: u32| previous.min(max)));
        }
        bound
    }

    /// Returns true when the announcement suspended for a player choice.
    pub(super) fn announce_activation_x(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        index: u32,
        cost: &ActivationPrice,
    ) -> Result<bool, EngineError> {
        let resource_max = self.activation_resource_x_max(player, source, cost);
        let rule = self.activation_x_rule(source, index);
        if !cost.mana.has_variable() && resource_max.is_none() && rule == ActivationX::Any {
            return Ok(false);
        }
        if self.activation_x.is_some() {
            // Defined X was locked before targets and mana payments (CR 107.3a).
            if resource_max.is_some_and(|max| self.activation_x.unwrap_or(0) > max) {
                return Err(EngineError::IllegalAction("cannot pay variable cost"));
            }
            return Ok(false);
        }
        let min = self.activation_x_minimum(player, source, index);
        if matches!(rule, ActivationX::Defined(_)) {
            self.activation_x = Some(min);
            return Ok(false);
        }
        let mut max = resource_max.unwrap_or(u32::MAX);
        if cost.mana.has_variable() && !self.can_make_more_mana(player, source, index) {
            let Some(
                AbilityDef::Activated {
                    cost: printed,
                    cost_reduction: reduction,
                    ..
                }
                | AbilityDef::ModalActivated {
                    cost: printed,
                    cost_reduction: reduction,
                    ..
                }
                | AbilityDef::ActivatedConditional {
                    cost: printed,
                    cost_reduction: reduction,
                    ..
                },
            ) = self.activation_definition(source, index)
            else {
                return Err(EngineError::IllegalAction("not an activated ability"));
            };
            let credit = self.state.object(source).map_or(0, |object| {
                self.state
                    .effects
                    .iter()
                    .filter_map(|effect| match effect.modifier {
                        baylee_cards_dsl::Modifier::AbilitiesCostLess { amount, .. }
                            if crate::effects::applies_to(&self.state, effect, object) =>
                        {
                            Some(amount)
                        }
                        _ => None,
                    })
                    .fold(
                        casting::reduction_amount(&self.state, reduction, player, source),
                        u32::saturating_add,
                    )
            });
            let upper =
                casting::spendable_units(&self.state, player, casting::SpendFor::Ability(source))
                    .saturating_add(self.state.granted_colorless_capacity(player))
                    .saturating_add(credit);
            let additional_life =
                i32::try_from(self.mana_activation_life(source, index)).map_err(|_| {
                    EngineError::IllegalAction("activation life exceeds available life")
                })?;
            max = max.min(casting::greatest_affordable(upper, |x| {
                self.can_plan_activation(
                    player,
                    source,
                    &self.activation_price(
                        player,
                        source,
                        index,
                        &Cost {
                            mana: printed.mana.with_x(x),
                            parts: printed.parts,
                        },
                        reduction,
                    ),
                    additional_life,
                )
            }));
        }
        if min > max {
            return Err(EngineError::IllegalAction("no payable value of X"));
        }
        self.pending_plan = Some(PlanKind::ChooseActivationX {
            source,
            ability_index: index,
        });
        self.pending = Pending::ChooseNumber {
            player,
            min,
            max,
            reason: crate::choice::NumberPrompt::X,
        };
        self.awaiting_answer = true;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::testkit::{Duel, keep_mulligans, on_battlefield, reach_main_phase};
    use baylee_cards_dsl::CounterKind;
    use baylee_core::generated::index;

    #[test]
    fn resource_x_bound_matches_exhaustive_payment_for_combined_fixed_and_repeated_costs() {
        let p = PlayerId::new(0);
        let mut e = Duel::new(411, index::FOREST)
            .battlefield(0, &[index::FOREST])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p);
        let source = on_battlefield(&e, p, index::FOREST).unwrap();
        let cost = Cost {
            mana: baylee_core::mana::ManaCost::ZERO,
            parts: &[
                CostPart::PayEnergy(3),
                CostPart::PayEnergyX,
                CostPart::PayEnergyX,
                CostPart::RemoveCounterSelf {
                    kind: CounterKind::Charge,
                    n: 2,
                },
                CostPart::RemoveCounterSelfX {
                    kind: CounterKind::Charge,
                },
                CostPart::RemoveCounterSelfX {
                    kind: CounterKind::Charge,
                },
                CostPart::RemoveCounterSelfX {
                    kind: CounterKind::Charge,
                },
            ],
        };
        for energy in 3..35 {
            for counters in 2..35 {
                e.state.players[0].energy = energy;
                e.state
                    .object_mut(source)
                    .unwrap()
                    .counters
                    .set(CounterKind::Charge, counters);
                let reference = (0..=35)
                    .filter(|x| 3 + 2 * x <= u32::from(energy) && 2 + 3 * x <= u32::from(counters))
                    .max();
                let price = ActivationPrice::new(&cost, casting::SourceAllowance::default());
                assert_eq!(e.activation_resource_x_max(p, source, &price), reference);
            }
        }
    }
}
