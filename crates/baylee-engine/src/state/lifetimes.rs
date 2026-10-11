//! Durations that end at a state change, including inside one resolution.
use super::GameState;
use crate::effects::EffectFilter;
use crate::object::Status;
use crate::zone::Zone;
use baylee_cards_dsl::Duration;
use baylee_core::ids::{ObjectId, PlayerId};

impl GameState {
    /// CR 611.2b: a duration that cannot start creates no continuous effect.
    pub(crate) fn source_duration_holds(
        &self,
        duration: Duration,
        source: ObjectId,
        controller: PlayerId,
        filter: EffectFilter,
    ) -> bool {
        let (control, tapped, power_limited) = match duration {
            Duration::WhileYouControlSource => (true, false, false),
            Duration::WhileSourceTapped {
                control,
                power_limited,
            } => (control, true, power_limited),
            _ => return true,
        };
        let Some(source) = self.object(source).filter(|object| {
            object.zone == Zone::Battlefield && !object.status.contains(Status::PHASED_OUT)
        }) else {
            return false;
        };
        if (control && source.controller != controller)
            || (tapped && !source.status.contains(Status::TAPPED))
        {
            return false;
        }
        if !power_limited {
            return true;
        }
        let EffectFilter::ObjectIs(target, version) = filter else {
            return false;
        };
        self.object(target).is_some_and(|target| {
            target.version == version
                && target.zone == Zone::Battlefield
                && !target.status.contains(Status::PHASED_OUT)
                && target.characteristics().power.unwrap_or(0)
                    <= source.characteristics().power.unwrap_or(0)
        })
    }

    /// Every pass removes at least one effect. The caller reprojects until
    /// no duration expires; expired effects are never reinstated.
    pub(crate) fn expire_source_durations(&mut self) -> bool {
        if !self.effects.may_have_source_duration() {
            return false;
        }
        let expired: Vec<_> = self
            .effects
            .iter()
            .filter(|effect| {
                effect.source.is_some_and(|source| {
                    !self.source_duration_holds(
                        effect.duration,
                        source,
                        effect.controller,
                        effect.filter,
                    )
                })
            })
            .map(|effect| effect.id)
            .collect();
        if expired.is_empty() {
            return false;
        }
        // Iteration is in monotone EffectId order, so membership is logarithmic.
        self.effects
            .remove_where(|effect| expired.binary_search(&effect.id).is_ok());
        true
    }
}
