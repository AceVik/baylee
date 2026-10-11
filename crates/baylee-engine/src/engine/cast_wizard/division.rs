//! Casting-time damage distribution, retained through payment and copies.

use super::{
    AbilityDef, CardLookup, CastModeKind, CastWizard, Engine, EngineError, Pending, WizardStage,
};
use crate::choice::{NumberPrompt, TargetRef};

impl<L: CardLookup> Engine<L> {
    pub(super) fn wizard_divided_amount(&self, wizard: &CastWizard) -> Option<u32> {
        if wizard.masked || matches!(wizard.option, Some(CastModeKind::Disguise)) {
            return None;
        }
        let def = self
            .state
            .object(wizard.card)?
            .card
            .and_then(|c| self.lookup.card(c.index))?;
        let face = match wizard.option {
            Some(CastModeKind::Face(i)) => i,
            _ => 0,
        };
        def.abilities_for_face(face).iter().find_map(|ability| {
            let AbilityDef::Spell { effects, .. } = ability else {
                return None;
            };
            effects.iter().find_map(|effect| match effect {
                baylee_cards_dsl::Effect::DealDamageDivided { amount } => {
                    Some(amount.total(wizard.x))
                }
                _ => None,
            })
        })
    }

    pub(super) fn advance_damage_division(
        &mut self,
        mut wizard: CastWizard,
    ) -> Result<(), EngineError> {
        if let Some(total) = self.wizard_divided_amount(&wizard) {
            let targets = division_targets(&wizard);
            let asked = wizard.division.len();
            let left = total.saturating_sub(wizard.division.iter().sum());
            if asked + 1 < targets.len() {
                let reserved = u32::try_from(targets.len() - asked - 1).expect("target count");
                self.pending = Pending::ChooseNumber {
                    player: wizard.player,
                    min: 1,
                    max: left - reserved,
                    reason: NumberPrompt::DivideDamage {
                        target: targets[asked],
                        index: u8::try_from(asked).expect("division target index"),
                        of: u8::try_from(targets.len()).expect("division target count"),
                        left,
                    },
                };
                self.cast_wizard = Some(wizard);
                self.awaiting_answer = true;
                return Ok(());
            }
            if asked < targets.len() {
                wizard.division.push(left);
            }
        }
        wizard.stage = WizardStage::PitchChoice;
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
