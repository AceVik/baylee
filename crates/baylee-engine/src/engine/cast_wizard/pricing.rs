//! Prices left after the selected option consumes its fixed generic cost.
#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct FrozenReduction {
    pub total: u32,
    pub remainder: u32,
}

impl<L: CardLookup> Engine<L> {
    pub(super) fn frozen_spell_reduction(&self, wizard: &CastWizard) -> FrozenReduction {
        let def = self
            .state
            .object(wizard.card)
            .and_then(|object| object.card)
            .and_then(|card| self.lookup.card(card.index))
            .expect("wizard card known");
        let index = match wizard.option {
            Some(CastModeKind::Face(index)) => index,
            _ => 0,
        };
        let face = self.wizard_face(wizard);
        let form = match spend_for(wizard, face) {
            casting::SpendFor::SpellAs(_, form) => Some(form),
            _ => None,
        };
        let adjustment =
            casting::face_adjustment(&self.state, wizard.player, wizard.card, def, index, form);
        let printed = if wizard.masked || wizard.option == Some(CastModeKind::Disguise) {
            0
        } else {
            casting::printed_reduction(&self.state, face, wizard.player, wizard.card)
        };
        let total = adjustment.reduction.saturating_add(printed);
        let raw = raw_option_cost(def, face, wizard);
        let fixed = raw.generic_total().saturating_add(adjustment.increase);
        FrozenReduction {
            total,
            remainder: total.saturating_sub(fixed),
        }
    }
}

/// Reconstruct only the selected alternative's unreduced fixed cost. This
/// preserves credit when an option's displayed price already clamped to zero.
fn raw_option_cost(
    def: &baylee_cards_dsl::CardDef,
    face: &baylee_cards_dsl::FaceDef,
    wizard: &CastWizard,
) -> ManaCost {
    let normal = if wizard.free || wizard.masked {
        ManaCost::ZERO
    } else {
        face.mana_cost
    };
    match wizard.option {
        Some(CastModeKind::Normal | CastModeKind::Face(_)) | None => normal,
        Some(CastModeKind::Kicked) => casting::kicked_mana_cost(face),
        Some(CastModeKind::Alternative(index)) => face.alternative_costs[index].cost.mana,
        Some(CastModeKind::Prototype) => face.prototype.expect("prototype").cost,
        Some(CastModeKind::Disguise) => const { ManaCost::parse("{3}") },
        Some(CastModeKind::Miracle) => face.miracle.expect("miracle"),
        Some(CastModeKind::Flashback) => face.flashback.expect("flashback"),
        Some(CastModeKind::Dash) => face.dash.expect("dash"),
        Some(CastModeKind::Escape) => face.escape.expect("escape").cost,
        Some(CastModeKind::PlayLandFace(_)) => ManaCost::ZERO,
        Some(CastModeKind::Colors(_) | CastModeKind::Counter(_)) => {
            unreachable!("resolution choices are not spell costs")
        }
        Some(CastModeKind::Mode(index)) => def
            .abilities_for_face(0)
            .iter()
            .find_map(|ability| {
                let AbilityDef::ModalSpell { modes, .. } = ability else {
                    return None;
                };
                Some(modes[index].cost_override.unwrap_or(normal))
            })
            .expect("modal spell"),
        Some(CastModeKind::Modes(set)) => def
            .abilities_for_face(0)
            .iter()
            .find_map(|ability| {
                let AbilityDef::ModalSpell { modes, .. } = ability else {
                    return None;
                };
                Some(casting::mode_set_cost(normal, modes, set))
            })
            .expect("modal spell"),
    }
}
