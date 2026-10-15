//! Choose-one activation modes, frozen before targets and payment (CR 700.2a, 700.2c).
use super::{
    AbilityDef, CardLookup, Engine, EngineError, ObjectId, Pending, PlanKind, PlayerId, eval,
};
use crate::choice::{CastModeDesc, CastModeKind};
use baylee_cards_dsl::SpellMode;
use baylee_core::ids::DamageSourceRef;

#[derive(Clone, Copy, Debug, Hash)]
pub(crate) struct ActivationMode {
    pub source: DamageSourceRef,
    pub ability_index: u32,
    pub mode: u8,
}

/// Project one mode onto the ordinary announcement and payment checklist.
pub(super) fn selected_definition(definition: &AbilityDef, mode: &SpellMode) -> AbilityDef {
    let AbilityDef::ModalActivated {
        cost,
        timing,
        zone,
        condition,
        limit,
        x_value,
        cost_reduction,
        ..
    } = *definition
    else {
        return *definition;
    };
    let mana_ability = definition.is_mana_ability();
    if let Some(condition) = condition {
        AbilityDef::ActivatedConditional {
            cost,
            effects: mode.effects,
            targets: mode.targets,
            second_targets: mode.second_targets,
            timing,
            mana_ability,
            zone,
            condition,
            limit,
            x_value,
            cost_reduction,
        }
    } else {
        AbilityDef::Activated {
            cost,
            effects: mode.effects,
            targets: mode.targets,
            second_targets: mode.second_targets,
            timing,
            mana_ability,
            zone,
            limit,
            x_value,
            cost_reduction,
        }
    }
}

impl<L: CardLookup> Engine<L> {
    pub(crate) fn activation_mode_index(&self, source: ObjectId, index: u32) -> Option<u8> {
        self.activation_mode
            .filter(|chosen| {
                chosen.source.object == source
                    && chosen.ability_index == index
                    && self.state.source_identity(source) == Some(chosen.source)
            })
            .map(|chosen| chosen.mode)
    }

    pub(super) fn activation_mode_legal(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: u32,
        mode: &SpellMode,
    ) -> bool {
        mode.cost_override.is_none()
            && mode.additional_cost.is_none()
            && self.ability_has_targets(player, source, index, mode.targets, mode.second_targets)
    }

    /// Offers use the same selected requirements as announcement, without storing a choice.
    pub(super) fn activation_offer_definition(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: u32,
        definition: &AbilityDef,
    ) -> Option<AbilityDef> {
        let AbilityDef::ModalActivated { modes, .. } = definition else {
            return Some(*definition);
        };
        if modes.is_empty()
            || modes.len() > 255
            || modes
                .iter()
                .any(|mode| mode.cost_override.is_some() || mode.additional_cost.is_some())
        {
            return None;
        }
        modes
            .iter()
            .find(|mode| self.activation_mode_legal(player, source, index, mode))
            .map(|mode| selected_definition(definition, mode))
    }

    pub(super) fn announce_activation_mode(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
    ) -> Result<bool, EngineError> {
        let Some(
            definition @ AbilityDef::ModalActivated {
                modes, condition, ..
            },
        ) = self.activation_definition(source, ability_index)
        else {
            return Ok(false);
        };
        if modes.is_empty()
            || modes.len() > 255
            || modes
                .iter()
                .any(|mode| mode.cost_override.is_some() || mode.additional_cost.is_some())
        {
            return Err(EngineError::IllegalAction("unsupported activated modes"));
        }
        if condition.is_some_and(|condition| {
            !eval::condition_holds_with_context(
                &self.state,
                player,
                self.ability_rule_context(source, ability_index),
                condition,
            )
        }) {
            return Err(EngineError::IllegalAction("activation condition not met"));
        }
        if let Some(mode) = self.activation_mode_index(source, ability_index) {
            return if modes.get(usize::from(mode)).is_some() {
                Ok(false)
            } else {
                Err(EngineError::IllegalAction("no such activated mode"))
            };
        }
        if self.activation_mode.is_some() {
            return Err(EngineError::IllegalAction("activation mode source changed"));
        }
        let options = modes
            .iter()
            .enumerate()
            .filter(|(_, mode)| self.activation_mode_legal(player, source, ability_index, mode))
            .enumerate()
            .map(|(position, (index, _))| CastModeDesc {
                index: position as u32,
                kind: CastModeKind::Mode(index),
                cost: match definition {
                    AbilityDef::ModalActivated { cost, .. } => cost.mana,
                    _ => unreachable!(),
                },
            })
            .collect::<Vec<_>>();
        if options.is_empty() {
            return Err(EngineError::IllegalAction("no legal activated modes"));
        }
        self.pending_plan = Some(PlanKind::ModalActivation {
            source,
            ability_index,
        });
        self.pending = Pending::ChooseCastMode {
            player,
            object: source,
            options,
        };
        self.awaiting_answer = true;
        Ok(true)
    }
}
