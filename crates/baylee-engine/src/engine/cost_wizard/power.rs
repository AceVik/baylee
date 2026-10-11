//! Effective power used only to pay crew/saddle, with one reusable effect index.
use crate::{effects, layers::LayerPlan, state::GameState};
use baylee_cards_dsl::{CostPart, Layer, Modifier};
use baylee_core::ids::ObjectId;

pub(crate) struct PowerPayment<'a> {
    state: &'a GameState,
    plan: Option<LayerPlan>,
}

impl<'a> PowerPayment<'a> {
    pub(crate) fn new(state: &'a GameState, source: ObjectId, part: &CostPart) -> Self {
        use baylee_core::generated::subtypes::{artifact, creature};
        let subtype = if matches!(part, CostPart::Crew(_)) {
            artifact::VEHICLE
        } else {
            creature::MOUNT
        };
        let relevant = state
            .object(source)
            .is_some_and(|o| o.characteristics().subtypes.contains(subtype))
            && state
                .effects
                .iter()
                .any(|effect| matches!(effect.modifier, Modifier::CrewSaddlePowerBonus(_)));
        Self {
            state,
            plan: relevant.then(|| LayerPlan::build(&state.effects)),
        }
    }

    pub(crate) fn weight(&self, id: ObjectId) -> i32 {
        let Some(object) = self.state.object(id) else {
            return 0;
        };
        let base = i32::from(object.characteristics().power.unwrap_or(0));
        let Some(plan) = &self.plan else {
            return base;
        };
        let total = plan.effects_for(&self.state.effects, Layer::Text, id).fold(
            i64::from(base),
            |power, effect| {
                if let Modifier::CrewSaddlePowerBonus(bonus) = effect.modifier
                    && effects::applies_to(self.state, effect, object)
                {
                    power + i64::from(bonus)
                } else {
                    power
                }
            },
        );
        i32::try_from(total.clamp(i64::from(i32::MIN), i64::from(i32::MAX)))
            .expect("clamped power weight")
    }

    /// With no upper count limit, positive contributions are the maximum.
    /// Stop once the threshold is met; negative powers never help feasibility.
    pub(crate) fn can_reach(&self, options: &[ObjectId], target: u8) -> bool {
        let mut remaining = i32::from(target);
        for &id in options {
            if remaining <= 0 {
                return true;
            }
            remaining = remaining.saturating_sub(self.weight(id).max(0));
        }
        remaining <= 0
    }
}
