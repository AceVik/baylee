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
        AbilityDef::ModalSpell { modes } | AbilityDef::ModalTriggered { modes, .. } => {
            mode.and_then(|i| modes.get(i)).map_or(&[], |m| m.effects)
        }
        _ => &[],
    }
}

impl<L: CardLookup> Engine<L> {
    /// Explains a pending choice to an in-process controller. This is not a
    /// player request endpoint and contains no library or opposing hand.
    #[must_use]
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
            let captured = if matches!(self.pending_plan, Some(PlanKind::Trigger { .. })) {
                self.trigger_queue.front().and_then(|t| t.abilities)
            } else {
                self.activating_abilities
                    .filter(|(id, _)| *id == source)
                    .map(|(_, list)| list)
            };
            let abilities = captured.map(|list| list.abilities).or_else(|| {
                self.state
                    .object(source)
                    .map(|o| o.printed_abilities(&self.lookup))
            });
            return DecisionContext {
                source: Some(source),
                printed: captured.and_then(|list| list.printed).or_else(|| {
                    self.state
                        .object(source)
                        .and_then(crate::object::GameObject::printed_face)
                }),
                ability_index: Some(index),
                mode,
                effects: abilities
                    .and_then(|a| a.get(index as usize))
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
                DecisionContext {
                    source: Some(res.source),
                    printed: stack.and_then(crate::object::GameObject::printed_face),
                    ability_index: stack.and_then(|o| o.ability).map(|a| a.index),
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
        let mode = match wizard.option {
            Some(CastModeKind::Mode(i)) => Some(i),
            _ => None,
        };
        DecisionContext {
            source: Some(wizard.card),
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
            second_instance: wizard.stage == super::cast_wizard::WizardStage::SecondTargets,
            first_targets: &wizard.targets,
            // A clone chooses as it resolves, never while it is being cast.
            copying: None,
        }
    }
}
