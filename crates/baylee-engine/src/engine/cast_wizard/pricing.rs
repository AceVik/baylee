//! Prices left after the selected option consumes its fixed generic cost.
#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct FrozenReduction {
    pub total: u32,
    pub remainder: u32,
    pub allowance: casting::SourceAllowance,
}

impl<L: CardLookup> Engine<L> {
    pub(in crate::engine) fn wizard_source_allowance(
        &self,
        wizard: &CastWizard,
    ) -> casting::SourceAllowance {
        let frozen = wizard
            .reduction
            .unwrap_or_else(|| self.frozen_spell_reduction(wizard));
        let selected = u32::try_from(wizard.delve_exiles.len() + wizard.convoke_taps.len())
            .unwrap_or(u32::MAX);
        frozen.allowance.reduced(selected)
    }

    pub(super) fn frozen_spell_reduction(&self, wizard: &CastWizard) -> FrozenReduction {
        let def = self
            .state
            .object(wizard.card)
            .and_then(|object| object.card)
            .and_then(|card| self.lookup.card(card.index))
            .expect("wizard card known");
        let index = wizard.face_index();
        let face = self.wizard_face(wizard);
        let form = match spend_for(wizard, face) {
            casting::SpendFor::SpellAs(_, form) => Some(form),
            casting::SpendFor::KickedSpell(_, _, form) => form,
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
            allowance: casting::base_allowance(&self.state, spend_for(wizard, face)).adjusted(
                adjustment.eligible_increase,
                adjustment
                    .increase
                    .saturating_sub(adjustment.eligible_increase),
                total,
            ),
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
    let raw = match wizard.option {
        Some(CastModeKind::Normal | CastModeKind::Face(_)) | None => normal,
        Some(CastModeKind::Kicked) => casting::kicked_mana_cost(face),
        Some(CastModeKind::Alternative(index)) => face.alternative_costs[index].cost.mana,
        Some(CastModeKind::Prototype) => face.prototype.expect("prototype").cost,
        Some(CastModeKind::Disguise) => const { ManaCost::parse("{3}") },
        Some(CastModeKind::Miracle) => face.miracle.expect("miracle"),
        Some(CastModeKind::Flashback) => face.flashback.expect("flashback"),
        Some(CastModeKind::BeamMeUp) => face.beam_me_up.expect("beam me up"),
        Some(CastModeKind::Dash) => face.dash.expect("dash"),
        Some(CastModeKind::Mutate) => face.mutate.expect("mutate"),
        Some(CastModeKind::Escape) => face.escape.expect("escape").cost,
        Some(CastModeKind::PlayLandFace(_)) => ManaCost::ZERO,
        Some(
            CastModeKind::Dungeon(_)
            | CastModeKind::DungeonRoom { .. }
            | CastModeKind::MutatePosition(_)
            | CastModeKind::Colors(_)
            | CastModeKind::Counter(_)
            | CastModeKind::Permission(_)
            | CastModeKind::EscapeGranted { .. },
        ) => {
            unreachable!("resolution choices are not spell costs")
        }
        Some(CastModeKind::Mode(index)) => def
            .abilities_for_face(wizard.face_index())
            .iter()
            .find_map(|ability| {
                let AbilityDef::ModalSpell { modes, .. } = ability else {
                    return None;
                };
                Some(modes[index].cost_override.unwrap_or(normal))
            })
            .expect("modal spell"),
        Some(CastModeKind::Modes(set)) => def
            .abilities_for_face(wizard.face_index())
            .iter()
            .find_map(|ability| {
                let AbilityDef::ModalSpell { modes, choose } = ability else {
                    return None;
                };
                let base = casting::mode_set_cost(normal, modes, set);
                Some(
                    if choose.is_one()
                        && let Some(entwine) = face.entwine
                    {
                        base.combine(&entwine.mana)
                    } else {
                        base
                    },
                )
            })
            .expect("modal spell"),
    };
    if wizard.option != Some(CastModeKind::Kicked)
        && matches!(
            wizard.announced,
            Some(CastModeKind::EscapeGranted { kicked: true, .. })
        )
    {
        face.additional_costs
            .iter()
            .fold(raw, |cost, additional| cost.combine(&additional.mana))
    } else {
        raw
    }
}
