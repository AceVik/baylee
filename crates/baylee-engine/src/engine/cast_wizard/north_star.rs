//! North Star changes payment of the mana cost, after reductions, exactly once.
#[allow(clippy::wildcard_imports)]
use super::*;

impl<L: CardLookup> Engine<L> {
    pub(super) fn north_star_component(&self, wizard: &CastWizard) -> Option<ManaCost> {
        if wizard.free
            || wizard.masked
            || wizard.escapes()
            || wizard.permission == Some(casting::GraveyardPermission::Flashback)
        {
            return None;
        }
        let face = self.wizard_face(wizard);
        let option = wizard.option?;
        let def = self
            .state
            .object(wizard.card)
            .and_then(|object| object.card)
            .and_then(|card| self.lookup.card(card.index))?;
        if !casting::mana_cost_mode(def, wizard.face_index(), option) {
            return None;
        }
        if option == CastModeKind::Prototype {
            face.prototype.map(|prototype| prototype.cost)
        } else {
            Some(face.mana_cost)
        }
    }

    pub(super) fn ask_north_star(&mut self, mut wizard: CastWizard) -> Result<(), EngineError> {
        if self.north_star_component(&wizard).is_some()
            && casting::north_star_effect(&self.state, wizard.player).is_some()
        {
            self.pending = Pending::YesNo {
                player: wizard.player,
                prompt: crate::choice::YesNoPrompt::ManaAnyType,
                source: None,
            };
            self.cast_wizard = Some(wizard);
            self.awaiting_answer = true;
            return Ok(());
        }
        wizard.stage = WizardStage::XValue;
        self.cast_wizard = Some(wizard);
        self.advance_cast_wizard()
    }

    pub(in crate::engine) fn north_star_payment(
        &self,
        wizard: &CastWizard,
        cost: ManaCost,
    ) -> ManaCost {
        if wizard.by_effect == Some(EffectCast::AnyMana) {
            return cost.with_any_type_component(&cost);
        }
        let Some(component) = wizard
            .north_star
            .and_then(|_| self.north_star_component(wizard))
        else {
            return cost;
        };
        let paid_component = chosen_option_cost(wizard)
            .component_after_phyrexian(&component, phyrexian_life_mask(wizard));
        cost.with_any_type_component(&paid_component)
    }
}
