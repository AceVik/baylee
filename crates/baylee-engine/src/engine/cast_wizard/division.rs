//! Casting-time damage and counter distribution, retained through payment and copies.

use super::{
    AbilityDef, CardLookup, CastModeKind, CastWizard, Engine, EngineError, Pending, WizardStage,
};
use crate::choice::TargetRef;
use crate::engine::division::{DividedKind, effects_divided_spec};

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    pub(super) fn wizard_divided_spec(&self, wizard: &CastWizard) -> Option<(DividedKind, u32)> {
        if wizard.masked || matches!(wizard.option, Some(CastModeKind::Disguise)) {
            return None;
        }
        let obj = self.state.object(wizard.card)?;
        let def = obj.card.and_then(|c| self.lookup.card(c.index))?;
        let face = match wizard.option {
            Some(CastModeKind::Face(i)) => i,
            _ => 0,
        };
        let context = crate::eval::live_context(&self.state, wizard.card);
        let version = Some(obj.version);
        def.abilities_for_face(face).iter().find_map(|ability| {
            let effects = if let AbilityDef::Spell { effects, .. } = ability {
                *effects
            } else {
                let (modes, ..) = ability.modal_spell()?;
                let Some(CastModeKind::Mode(i)) = wizard.option else {
                    return None;
                };
                modes.get(i)?.effects
            };
            effects_divided_spec(
                effects,
                &self.state,
                wizard.player,
                context,
                version,
                wizard.x,
            )
        })
    }

    pub(super) fn wizard_divided_amount(&self, wizard: &CastWizard) -> Option<u32> {
        self.wizard_divided_spec(wizard).map(|(_, total)| total)
    }

    pub(super) fn advance_damage_division(
        &mut self,
        mut wizard: CastWizard,
    ) -> Result<(), EngineError> {
        if let Some((kind, total)) = self.wizard_divided_spec(&wizard) {
            let targets = division_targets(&wizard);
            let asked = wizard.division.len();
            let left = total.saturating_sub(wizard.division.iter().sum());
            if asked + 1 < targets.len() {
                let reserved = u32::try_from(targets.len() - asked - 1).expect("target count");
                self.pending = Pending::ChooseNumber {
                    player: wizard.player,
                    min: 1,
                    max: left - reserved,
                    reason: kind.prompt(targets[asked], asked, targets.len(), left),
                };
                self.cast_wizard = Some(wizard);
                self.awaiting_answer = true;
                return Ok(());
            }
            if asked < targets.len() && total > 0 {
                wizard.division.push(left);
            }
        }
        self.refresh_target_cost_adjustments(&mut wizard);
        wizard.stage = WizardStage::EitherCost;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    pub(super) fn store_damage_division(&mut self, wizard: &CastWizard) {
        if !wizard.division.is_empty() {
            let shares = division_targets(wizard)
                .into_iter()
                .zip(wizard.division.iter().copied())
                .collect();
            self.state.divided.push((wizard.card, shares));
        }
    }
}

fn division_targets(wizard: &CastWizard) -> Vec<TargetRef> {
    wizard
        .target_references
        .first
        .iter()
        .copied()
        .map(TargetRef::Object)
        .chain(wizard.target_players.iter().map(TargetRef::Player))
        .chain(
            wizard
                .chosen_player
                .filter(|p| !wizard.target_players.contains(*p))
                .map(TargetRef::Player),
        )
        .collect()
}
