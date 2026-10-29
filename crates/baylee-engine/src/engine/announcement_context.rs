//! Read-only announcement facts for explicitly opted-in authorized views.
use super::{
    Engine, PaymentContinuation,
    cast_wizard::{CastWizard, WizardStage},
};
use crate::state::CardLookup;
use baylee_core::ids::{ObjectId, PlayerId};

fn settled_x(wizard: &CastWizard) -> Option<u32> {
    match wizard.stage {
        WizardStage::ChooseMode
        | WizardStage::Permission
        | WizardStage::ManaPermission
        | WizardStage::XValue => None,
        WizardStage::Kicker
        | WizardStage::Replicate
        | WizardStage::Targets
        | WizardStage::SecondTargets
        | WizardStage::ChoosePlayer
        | WizardStage::DivideDamage
        | WizardStage::PitchChoice
        | WizardStage::ReturnCreature
        | WizardStage::Behold
        | WizardStage::Delve
        | WizardStage::Convoke
        | WizardStage::Sacrifice
        | WizardStage::Escape
        | WizardStage::Done => Some(wizard.x),
    }
}

impl<L: CardLookup> Engine<L> {
    /// In-progress source and settled X, visible only to its resource owner or
    /// that owner's currently authorized controller (CR 720.4).
    ///
    /// `None` X means the announcement has not passed its X choice. `Some(0)`
    /// includes an explicitly chosen zero and a cost whose X is fixed at zero.
    /// This grants no cancellation permission and changes no engine state.
    #[must_use]
    pub fn announced_spell_context(&self, viewer: PlayerId) -> Option<(ObjectId, Option<u32>)> {
        if let Some(wizard) = &self.cast_wizard
            && self.may_inspect_private(viewer, wizard.player)
        {
            return Some((wizard.card, settled_x(wizard)));
        }
        let mut window = self.mana_window.as_ref();
        while let Some(payment) = window {
            if !self.may_inspect_private(viewer, payment.player) {
                return None;
            }
            match &payment.suspended {
                PaymentContinuation::Miracle { wizard, .. } => {
                    return Some((wizard.card, settled_x(wizard)));
                }
                PaymentContinuation::Cast { card, .. } => {
                    // An effect opens this mana window BEFORE start_paid_cast
                    // begins the wizard and asks X. A stored object's old X
                    // cannot certify a choice for the new announcement.
                    return Some((*card, None));
                }
                PaymentContinuation::Activation(activation) => {
                    window = activation.previous.as_deref();
                }
                _ => return None,
            }
        }
        None
    }
}
