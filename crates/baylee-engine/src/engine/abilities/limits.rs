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
