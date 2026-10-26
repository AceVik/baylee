//! Per-incarnation activation limits.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// Whether a printed "activate only once each turn" has already been
    /// spent on this ability this turn.
    ///
    /// The tally is [`GameState::ability_fires`], shared with once-per-turn
    /// triggers: both are counts of *this ability of this object* within a
    /// turn, both are keyed the same way, and both are cleared as a turn
    /// begins. The key carries the object, so a permanent that leaves and
    /// comes back is a new object with a fresh count — which is CR 400.7
    /// rather than a convenience.
    ///
    /// Asked on the offering side, so an exhausted ability is simply not in
    /// `legal.abilities`; `apply`'s offer guard is then the whole of the
    /// refusal, the way it is for every other activation restriction.
    pub(super) fn activation_limit_spent(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: u32,
        limit: ActivationLimit,
    ) -> bool {
        let maximum = match limit {
            ActivationLimit::Unlimited => return false,
            ActivationLimit::Exhaust => {
                return self.state.object(source).is_none_or(|o| {
                    o.riders
                        .contains(&crate::object::Rider::ExhaustedAbility(index))
                });
            }
            ActivationLimit::PerTurn(n) => u32::from(n),
            ActivationLimit::PerTurnAmount(amount) => {
                crate::eval::amount(&amount, &self.state, player, source, None)
            }
            ActivationLimit::Boast => {
                let Some(object) = self.state.object(source) else {
                    return true;
                };
                if !self
                    .state
                    .per_turn
                    .attacked
                    .iter()
                    .any(|&(id, version, _)| id == source && version == object.version)
                {
                    return true;
                }
                let doubled = self.state.turn.active == player
                    && self.state.effects.iter().any(|effect| {
                        effect.controller == player
                            && effect.modifier == baylee_cards_dsl::Modifier::BoastTwice
                            && crate::effects::applies_to(&self.state, effect, object)
                    });
                if doubled { 2 } else { 1 }
            }
        };
        self.state
            .ability_fires
            .get(&(
                self.state.source_identity(source).expect("offered source"),
                index,
            ))
            .copied()
            .unwrap_or(0)
            >= maximum
    }
}

#[cfg(test)]
mod tests {
    use crate::engine::synthetic::{SyntheticLookup, keep_mulligans, land, permanents, preset};
    use crate::engine::*;
    use baylee_cards_dsl::{ActivationLimit, Amount, CounterKind, Effect, mana_ability};

    #[test]
    fn amount_activation_limit_rechecks_current_count_and_records_successful_activations() {
        const INDEX: u32 = 19031;
        static ABILITIES: &[AbilityDef] = &[mana_ability!(
            Cost::FREE,
            &[Effect::mana(baylee_core::mana::ManaColor::Colorless, 1)],
            limit = ActivationLimit::PerTurnAmount(Amount::CountersOnSource(CounterKind::Charge))
        )];
        let mut engine = Engine::new(
            &preset(51, &[INDEX]),
            SyntheticLookup::new(vec![land(INDEX, "Bounded mana", ABILITIES)]),
        )
        .unwrap();
        keep_mulligans(&mut engine);
        let source = permanents(&engine, INDEX)[0];
        let player = engine.state.object(source).unwrap().controller;
        let limit = ActivationLimit::PerTurnAmount(Amount::CountersOnSource(CounterKind::Charge));
        assert!(engine.activation_limit_spent(player, source, 0, limit));
        crate::replacement::put_counters(&mut engine.state, source, CounterKind::Charge, 1);
        assert!(!engine.activation_limit_spent(player, source, 0, limit));
        // Refresh the current question after changing the synthetic fixture.
        engine.pending = Pending::Priority {
            player,
            legal: Box::new(engine.compute_legal(player)),
        };
        engine
            .apply(
                player,
                PlayerAction::ActivateAbility {
                    source,
                    ability_index: 0,
                },
            )
            .unwrap();
        assert!(engine.activation_limit_spent(player, source, 0, limit));
        crate::replacement::put_counters(&mut engine.state, source, CounterKind::Charge, 1);
        assert!(!engine.activation_limit_spent(player, source, 0, limit));
        crate::replacement::remove_counters(&mut engine.state, source, CounterKind::Charge, 1);
        assert!(engine.activation_limit_spent(player, source, 0, limit));
        engine.begin_turn(false);
        assert!(
            !engine.activation_limit_spent(player, source, 0, limit),
            "new turns reset actual usage"
        );
    }
}
