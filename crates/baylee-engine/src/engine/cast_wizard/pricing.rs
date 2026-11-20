//! Prices left after the selected option consumes its fixed generic cost.
#[allow(clippy::wildcard_imports)]
use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct FrozenReduction {
    pub total: u32,
    pub remainder: u32,
    pub colored_only_remainder: ManaCost,
    pub mana_remainder: ManaCost,
    pub allowance: casting::SourceAllowance,
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    pub(in crate::engine) fn wizard_source_allowance(
        &self,
        wizard: &CastWizard,
    ) -> casting::SourceAllowance {
        let frozen = wizard
            .reduction
            .unwrap_or_else(|| self.frozen_spell_reduction(wizard));
        // What the taps and exiles pay of the generic part: a convoke tap
        // that pays a coloured symbol (CR 702.51a) takes nothing off it.
        let colored = wizard_total_cost(self.wizard_face(wizard), wizard)
            .convoked_colored(&self.convoke_tap_colors(wizard))
            .1;
        let selected = u32::try_from(wizard.delve_exiles.len() + wizard.convoke_taps.len())
            .unwrap_or(u32::MAX)
            .saturating_sub(colored);
        frozen.allowance.reduced(selected)
    }

    pub(in crate::engine) fn frozen_spell_reduction(&self, wizard: &CastWizard) -> FrozenReduction {
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
        let emerge_mv = if wizard.option == Some(CastModeKind::Emerge) {
            wizard
                .emerge_sacrifice
                .and_then(|id| self.state.object(id))
                .map_or(0, |o| o.characteristics().mana_value())
        } else {
            0
        };
        let total = adjustment
            .reduction
            .saturating_add(printed)
            .saturating_add(emerge_mv);
        let raw = raw_option_cost(def, face, wizard);
        let raw_with_inc = raw
            .combine(&adjustment.mana_increase)
            .with_more_generic(adjustment.increase);
        let (after_co, colored_only_remainder) =
            raw_with_inc.with_less_mana_remainder(&adjustment.colored_only_reduction, true);
        let (after_mana, mana_remainder) =
            after_co.with_less_mana_remainder(&adjustment.mana_reduction, false);
        let fixed = after_mana.generic_total();
        FrozenReduction {
            total,
            remainder: total.saturating_sub(fixed),
            colored_only_remainder,
            mana_remainder,
            allowance: casting::base_allowance(&self.state, spend_for(wizard, face)).adjusted(
                adjustment.eligible_increase,
                adjustment
                    .increase
                    .saturating_sub(adjustment.eligible_increase),
                total,
            ),
        }
    }

    /// Computes the priced `CastModeKind::Emerge` mana cost after subtracting
    /// `sacrificed_mv` generic mana (`CR 702.119a`, `CR 601.2f`).
    pub(in crate::engine) fn emerge_option_cost(
        &self,
        wizard: &CastWizard,
        sacrificed_mv: u32,
    ) -> ManaCost {
        let def = self
            .state
            .object(wizard.card)
            .and_then(|object| object.card)
            .and_then(|card| self.lookup.card(card.index))
            .expect("wizard card known");
        let face = self.wizard_face(wizard);
        let emerge = face.emerge.expect("emerge option");
        let adjustment = casting::face_adjustment(
            &self.state,
            wizard.player,
            wizard.card,
            def,
            wizard.face_index(),
            None,
        );
        let printed = casting::printed_reduction(&self.state, face, wizard.player, wizard.card);
        adjustment.price(emerge.cost, printed.saturating_add(sacrificed_mv))
    }

    /// Recomputes target-dependent cost modifiers (CR 601.2c -> 601.2f, such as
    /// `Filter::SpellTargetingAffected` on Kaervek's Torch or
    /// `Filter::TargetsPermanentYouControl`) once targets have been chosen.
    pub(super) fn refresh_target_cost_adjustments(&self, wizard: &mut CastWizard) {
        if !self.state.effects.may_change_spell_costs() {
            return;
        }
        let Some(object) = self.state.object(wizard.card) else {
            return;
        };
        let Some(def) = object.card.and_then(|c| self.lookup.card(c.index)) else {
            return;
        };
        let face_idx = wizard.face_index();
        let face = self.wizard_face(wizard);
        let form = match spend_for(wizard, face) {
            casting::SpendFor::SpellAs(_, form) => Some(form),
            casting::SpendFor::KickedSpell(_, _, form) => form,
            _ => None,
        };
        let without =
            casting::face_adjustment(&self.state, wizard.player, wizard.card, def, face_idx, form);
        let mut targeted = object.clone();
        targeted.base = std::sync::Arc::new(crate::object::Characteristics::from_face(
            def,
            face_idx,
            object.base.name,
        ));
        targeted.cache.clear();
        targeted.face_index = u8::try_from(face_idx).unwrap_or(0);
        if let Some(form) = form {
            targeted = form.project(&targeted);
        }
        targeted.targets.clone_from(&wizard.targets);
        targeted.target_players = wizard.target_players;
        targeted.set_second(wizard.second_targets.clone(), None);
        targeted.set_second_players(wizard.second_target_players);
        let with_targets =
            casting::spell_adjustment_with_def(&self.state, wizard.player, &targeted, Some(def));
        wizard.target_increase = with_targets
            .mana_increase
            .with_less_mana(&without.mana_increase, true)
            .with_more_generic(with_targets.increase.saturating_sub(without.increase));
        let extra_red = with_targets.reduction.saturating_sub(without.reduction);
        if extra_red > 0
            && let Some(frozen) = wizard.reduction.as_mut()
        {
            frozen.total = frozen.total.saturating_add(extra_red);
            frozen.remainder = frozen.remainder.saturating_add(extra_red);
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
    let library_top = matches!(wizard.announced, Some(CastModeKind::LibraryTopLife { .. }));
    let normal = if wizard.free || wizard.masked || library_top {
        ManaCost::ZERO
    } else if wizard.alternative_cost == Some(CastModeKind::Madness) {
        face.madness.expect("madness").mana
    } else {
        face.mana_cost
    };
    let raw = match wizard.option {
        Some(CastModeKind::Normal | CastModeKind::Face(_)) | None => normal,
        // Only kicker's additional mana survives the alternative zero base.
        Some(CastModeKind::Kicked) if library_top => face
            .additional_costs
            .iter()
            .fold(ManaCost::ZERO, |cost, additional| {
                cost.combine(&additional.mana)
            }),
        Some(CastModeKind::Kicked) => casting::kicked_mana_cost(face),
        Some(CastModeKind::Alternative(index)) => face.alternative_costs[index].cost.mana,
        Some(CastModeKind::Prototype) => face.prototype.expect("prototype").cost,
        Some(CastModeKind::Disguise) => const { ManaCost::parse("{3}") },
        Some(CastModeKind::Miracle) => face.miracle.expect("miracle"),
        Some(CastModeKind::Madness) => face.madness.expect("madness").mana,
        Some(CastModeKind::Foretell) => face.foretell.expect("foretell"),
        Some(CastModeKind::Flashback) => face.flashback.expect("flashback"),
        Some(CastModeKind::BeamMeUp) => face.beam_me_up.expect("beam me up"),
        Some(CastModeKind::Dash) => face.dash.expect("dash"),
        Some(CastModeKind::Blitz) => face.blitz.expect("blitz").mana,
        Some(CastModeKind::Emerge) => face.emerge.expect("emerge").cost,
        Some(CastModeKind::Mutate) => face.mutate.expect("mutate"),
        Some(CastModeKind::Bestow) => face.bestow.expect("bestow").mana,
        Some(CastModeKind::JumpStart) => face.mana_cost,
        Some(CastModeKind::Escape) => face.escape.expect("escape").cost,
        Some(CastModeKind::PlayLandFace(_)) => ManaCost::ZERO,
        Some(
            CastModeKind::Dungeon(_)
            | CastModeKind::DungeonRoom { .. }
            | CastModeKind::MutatePosition(_)
            | CastModeKind::Colors(_)
            | CastModeKind::Counter(_)
            | CastModeKind::Permission(_)
            | CastModeKind::EscapeGranted { .. }
            | CastModeKind::LibraryTopLife { .. }
            | CastModeKind::SpellEnding(_),
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
                let mode = modes.get(index)?;
                // A cost-overriding mode is another alternative cost, which
                // the library-top reader does not combine with this route.
                Some(if library_top {
                    normal
                } else {
                    mode.cost_override.unwrap_or(normal)
                })
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
            Some(
                CastModeKind::EscapeGranted { kicked: true, .. }
                    | CastModeKind::LibraryTopLife { kicked: true, .. }
            )
        )
    {
        face.additional_costs
            .iter()
            .fold(raw, |cost, additional| cost.combine(&additional.mana))
    } else {
        raw
    }
}
