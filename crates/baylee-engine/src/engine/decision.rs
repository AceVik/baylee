//! In-process explanation of the current choice, separate from the wire view.

use super::{Engine, PlanKind};
use crate::choice::CastModeKind;
use crate::state::CardLookup;
use baylee_cards_dsl::{AbilityDef, CopyMod, CostPart, Effect};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;

/// Rules context for an agent's current answer. Borrowed, never serialized.
/// It describes the selected mode, including copied and triggered abilities;
/// trying to infer this from the last permanent played loses that information.
#[derive(Clone, Copy, Debug, Default)]
pub struct DecisionContext<'a> {
    /// The object whose spell or ability is asking.
    pub source: Option<ObjectId>,
    /// Captured printed rules, including copies and a chosen back face.
    pub printed: Option<crate::object::PrintedFace>,
    /// Exact clause provenance within a composed list, including token text
    /// and abilities supplied by a copy exception.
    pub provenance: Option<crate::copiable_abilities::AbilityProvenance>,
    /// Printed ability and mode responsible for this decision.
    pub ability_index: Option<u32>,
    /// Selected modal branch, if any.
    pub mode: Option<usize>,
    /// A non-modal spell is explained by its whole face, not one ability sentence.
    pub whole_spell: bool,
    /// The selected effects, rather than every mode the card could have used.
    pub effects: &'a [Effect],
    /// Mana cost before substituting X, when the choice belongs to a cast.
    pub cost: Option<ManaCost>,
    /// Selected casting route, including miracle's later mana opportunity.
    pub cast_mode: Option<CastModeKind>,
    /// Whether X also costs life (for example Toxic Deluge).
    pub life_x: bool,
    /// X already announced for targeting and resolution.
    pub x: u32,
    /// Available distinct targets when the spell requires exactly X targets.
    pub x_targets: Option<u32>,
    /// Whether the target question being asked is for the **second**
    /// instance of the word "target" — what an effect names as
    /// [`baylee_cards_dsl::TargetSlot::Second`].
    ///
    /// The same effects explain both questions, and they mean opposite
    /// things to them: Bridgeworks Battle pumps its first target and fights
    /// its second, so an agent reading "this spell is beneficial" for both
    /// would decline to name any creature it is meant to fight.
    pub second_instance: bool,
    /// What the first instance already chose, while the second is asked —
    /// the fighter an agent weighs each candidate against.
    pub first_targets: &'a [ObjectId],
    /// What the copy changes, while the question is which object the
    /// source enters as a copy of (CR 707.2, asked before it enters by CR
    /// 614.12a); `None` for every other question.
    ///
    /// The clone's choice arrives as a target question with nothing behind
    /// it: copying is an `AbilityDef`, not an `Effect`, so `effects` is empty
    /// and the source was not named either. An agent reading only those fell
    /// back to taking an opponent's permanent — Phyrexian Metamorph copied a
    /// Llanowar Elves over its own controller's Serra Angel (#227).
    pub copying: Option<&'static [CopyMod]>,
}

/// Effects of the chosen ability or mode. An unknown ability stays unknown.
fn effects(ability: &AbilityDef, mode: Option<usize>) -> &'static [Effect] {
    match ability {
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::Triggered { effects, .. }
        | AbilityDef::SagaChapter { effects, .. }
        | AbilityDef::Loyalty { effects, .. } => effects,
        AbilityDef::ModalSpell { modes, .. } | AbilityDef::ModalTriggered { modes, .. } => {
            mode.and_then(|i| modes.get(i)).map_or(&[], |m| m.effects)
        }
        _ => &[],
    }
}

/// Which of several chosen modes a cast is asking about: the one whose
/// targets are asked — the first of them that says "target", or at the
/// second instance of the word the second (CR 700.2c) — and otherwise the
/// first chosen.
fn asked_mode(def: &baylee_cards_dsl::CardDef, set: u8, second: bool) -> Option<usize> {
    def.abilities_for_face(0).iter().find_map(|a| match a {
        AbilityDef::ModalSpell { modes, .. } => {
            let mut targeting = crate::casting::chosen_modes(modes, set)
                .filter(|(_, mode)| mode.targets.is_some())
                .map(|(i, _)| i);
            let asked = if second {
                targeting.nth(1)
            } else {
                targeting.next()
            };
            asked.or_else(|| {
                crate::casting::chosen_modes(modes, set)
                    .next()
                    .map(|(i, _)| i)
            })
        }
        _ => None,
    })
}

impl<L: CardLookup> Engine<L> {
    /// Explains a pending choice to an in-process controller. This is not a
    /// player request endpoint and contains no library or opposing hand.
    #[must_use]
    #[allow(clippy::too_many_lines)] // one arm per plan that names an ability
    pub fn decision_context(&self) -> DecisionContext<'_> {
        if let Some(wizard) = &self.cast_wizard {
            return self.wizard_context(wizard);
        }
        if let Some(PlanKind::CopyOnEnter { object, .. }) = &self.pending_plan {
            return DecisionContext {
                source: Some(*object),
                copying: self.copy_on_enter(*object).map(|(_, mods)| mods),
                ..DecisionContext::default()
            };
        }
        let mut first: &[ObjectId] = &[];
        let handle = match &self.pending_plan {
            Some(PlanKind::ActivateAbilitySecondTargets {
                source,
                ability_index,
                targets,
                ..
            }) => {
                first = targets;
                Some((*source, *ability_index, None))
            }
            Some(
                PlanKind::ActivateAbility {
                    source,
                    ability_index,
                }
                | PlanKind::LoyaltyPlayer {
                    source,
                    ability_index,
                }
                | PlanKind::ChooseActivationX {
                    source,
                    ability_index,
                }
                | PlanKind::ChooseActivationGraveyard {
                    source,
                    ability_index,
                }
                | PlanKind::ChoosePhyrexianLife {
                    source,
                    ability_index,
                },
            ) => Some((*source, *ability_index, None)),
            Some(PlanKind::Trigger {
                source,
                ability_index,
                mode,
                ..
            }) => Some((*source, *ability_index, mode.map(usize::from))),
            // The trigger is on the stack already, its first instance chosen:
            // the ability it asks for is the one its stack object names, and
            // the first targets are shown beside the second question as an
            // activation's are. A division is asked the same way, of the
            // targets it divides among.
            Some(
                PlanKind::TriggerSecondTarget { on_stack }
                | PlanKind::DivideDamage { on_stack, .. },
            ) => self.state.object(*on_stack).and_then(|o| {
                first = &o.targets;
                o.ability.map(|loc| (loc.source, loc.index, None))
            }),
            _ => None,
        };
        if let Some((source, index, mode)) = handle {
            let captured = match self.pending_plan {
                Some(PlanKind::Trigger { .. }) => {
                    self.trigger_queue.front().and_then(|t| t.abilities.clone())
                }
                Some(
                    PlanKind::TriggerSecondTarget { on_stack }
                    | PlanKind::DivideDamage { on_stack, .. },
                ) => self
                    .state
                    .object(on_stack)
                    .map(|object| object.printed_ability_list(&self.lookup)),
                _ => self
                    .activating_abilities
                    .clone()
                    .filter(|(id, _)| *id == source)
                    .map(|(_, list)| list),
            };
            let list = captured.or_else(|| {
                self.state
                    .object(source)
                    .map(|o| o.printed_ability_list(&self.lookup))
            });
            let provenance = list.as_ref().map(|list| list.origin(index as usize));
            return DecisionContext {
                source: Some(source),
                printed: provenance
                    .and_then(|origin| origin.origin)
                    .and_then(crate::object::AbilityOrigin::printed),
                provenance,
                ability_index: Some(provenance.map_or(index, |origin| origin.index)),
                mode,
                effects: list
                    .and_then(|list| list.abilities.get(index as usize))
                    .map_or(&[], |a| effects(a, mode)),
                x: self.activation_x.unwrap_or(0),
                second_instance: matches!(
                    self.pending_plan,
                    Some(PlanKind::ActivateAbilitySecondTargets { .. })
                ),
                first_targets: first,
                ..DecisionContext::default()
            };
        }
        self.resolution
            .as_ref()
            .map_or_else(DecisionContext::default, |res| {
                let stack = self.state.object(res.on_stack);
                let mode = stack.and_then(|o| o.mode_index).map(usize::from);
                let provenance = stack.and_then(|object| {
                    let index = object.ability?.index;
                    Some(
                        object
                            .printed_ability_list(&self.lookup)
                            .origin(index as usize),
                    )
                });
                DecisionContext {
                    source: Some(res.source),
                    printed: provenance
                        .and_then(|origin| origin.origin)
                        .and_then(crate::object::AbilityOrigin::printed)
                        .or_else(|| {
                            stack
                                .filter(|object| object.ability.is_none())
                                .and_then(crate::object::GameObject::printed_face)
                        }),
                    provenance,
                    ability_index: provenance.map(|origin| origin.index),
                    mode,
                    whole_spell: stack.is_some_and(|o| o.ability.is_none()) && mode.is_none(),
                    effects: res.effects.get(res.pc..).unwrap_or_default(),
                    x: res.x.unwrap_or(0),
                    ..DecisionContext::default()
                }
            })
    }

    /// [`Self::decision_context`] while a cast is being announced.
    fn wizard_context<'a>(
        &'a self,
        wizard: &'a super::cast_wizard::CastWizard,
    ) -> DecisionContext<'a> {
        let Some(def) = self
            .state
            .object(wizard.card)
            .and_then(|o| o.card)
            .and_then(|c| self.lookup.card(c.index))
        else {
            return DecisionContext::default();
        };
        let face = match wizard.option {
            Some(CastModeKind::Face(i)) => i,
            _ => 0,
        };
        let second_stage = wizard.stage == super::cast_wizard::WizardStage::SecondTargets;
        let several = matches!(wizard.option, Some(CastModeKind::Modes(_)));
        let mode = match wizard.option {
            Some(CastModeKind::Mode(i)) => Some(i),
            Some(CastModeKind::Modes(set)) => asked_mode(def, set, second_stage),
            _ => None,
        };
        DecisionContext {
            source: Some(wizard.card),
            provenance: None,
            printed: u8::try_from(face)
                .ok()
                .and_then(|face| crate::object::PrintedFace::new(def.index, face)),
            ability_index: def
                .abilities_for_face(face)
                .iter()
                .position(|a| matches!(a, AbilityDef::Spell { .. } | AbilityDef::ModalSpell { .. }))
                .and_then(|i| u32::try_from(i).ok()),
            mode,
            whole_spell: mode.is_none(),
            effects: def
                .abilities_for_face(face)
                .iter()
                .find(|a| matches!(a, AbilityDef::Spell { .. } | AbilityDef::ModalSpell { .. }))
                .map_or(&[], |a| effects(a, mode)),
            cast_mode: wizard.option,
            cost: wizard
                .options
                .iter()
                .find(|o| Some(o.kind) == wizard.option)
                .map(|o| o.cost),
            life_x: def
                .faces
                .get(face)
                .is_some_and(|f| f.mandatory_additional_costs.contains(&CostPart::PayLifeX)),
            x: wizard.x,
            x_targets: self
                .wizard_target_req(wizard)
                .filter(|req| req.count_is_x)
                .map(|req| {
                    let objects = crate::eval::target_options(
                        &req.spec,
                        &self.state,
                        wizard.player,
                        wizard.card,
                    )
                    .len();
                    let players =
                        crate::eval::target_player_options(&self.state, &req.spec, wizard.player)
                            .len();
                    u32::try_from(objects + players).unwrap_or(u32::MAX)
                }),
            // The second instance of a spell cast with several modes belongs
            // to a mode of its own, with its own effects: it is that mode's
            // first, and what the other mode chose is nothing to weigh.
            second_instance: second_stage && !several,
            first_targets: if several { &[] } else { &wizard.targets },
            // A clone chooses as it resolves, never while it is being cast.
            copying: None,
        }
    }
}

#[cfg(test)]
mod provenance_tests {
    use super::*;
    use crate::copiable_abilities::compose;
    use crate::engine::testkit::{Duel, SEED};
    use crate::object::{AbilityList, PrintedFace};
    use baylee_core::generated::index;

    #[test]
    fn decision_uses_captured_clause_provenance_after_the_source_changes_again() {
        let mut engine = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[index::LLANOWAR_ELVES])
            .start();
        let source = *engine
            .state
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .first()
            .unwrap();
        let effigy = baylee_cards::by_index(index::MACHINE_GOD_S_EFFIGY).unwrap();
        let own = AbilityList::from_static(
            effigy.abilities_for_face(0),
            PrintedFace::new(effigy.index, 0),
            None,
        );
        let AbilityDef::CopyOnEnter { mods, .. } = own.abilities[0] else {
            panic!("copy clause");
        };
        let original = engine.state.printed_ability_list(source).unwrap();
        let copied = compose(original, &own, 0, mods, None);
        let captured_effects = effects(copied.abilities.get(1).unwrap(), None);
        engine.activating_abilities = Some((source, copied));
        engine.pending_plan = Some(PlanKind::ActivateAbility {
            source,
            ability_index: 1,
        });
        // The original body now answers with another list; the pending
        // decision still belongs to the ability captured before its cost.
        engine
            .state
            .object_mut(source)
            .unwrap()
            .take_abilities(AbilityList::NONE);
        let context = engine.decision_context();
        assert_eq!(context.printed, PrintedFace::new(effigy.index, 0));
        assert_eq!(context.ability_index, Some(0));
        assert_eq!(context.provenance.unwrap().copy_modifier, Some(2));
        assert_eq!(context.effects, captured_effects);
    }
}
