//! In-process explanation of the current choice, separate from the wire view.

use super::{Engine, PlanKind};
use crate::choice::CastModeKind;
use crate::state::CardLookup;
use baylee_cards_dsl::{AbilityDef, CostPart, Effect};
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaCost;

/// Rules context for an agent's current answer. Borrowed, never serialized.
/// It describes the selected mode, including copied and triggered abilities;
/// trying to infer this from the last permanent played loses that information.
#[derive(Clone, Copy, Debug, Default)]
pub struct DecisionContext<'a> {
    /// The object whose spell or ability is asking.
    pub source: Option<ObjectId>,
    /// The selected effects, rather than every mode the card could have used.
    pub effects: &'a [Effect],
    /// Mana cost before substituting X, when the choice belongs to a cast.
    pub cost: Option<ManaCost>,
    /// Whether X also costs life (for example Toxic Deluge).
    pub life_x: bool,
    /// X already announced for targeting and resolution.
    pub x: u32,
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
            return DecisionContext {
                source: Some(wizard.card),
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
            };
        }
        let handle = match self.pending_plan {
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
            ) => Some((source, ability_index, None)),
            Some(PlanKind::Trigger {
                source,
                ability_index,
                mode,
            }) => Some((source, ability_index, mode.map(usize::from))),
            _ => None,
        };
        if let Some((source, index, mode)) = handle {
            let abilities = self
                .activating_abilities
                .filter(|(id, _)| *id == source)
                .map(|(_, list)| list)
                .or_else(|| self.state.object(source).map(|o| o.abilities(&self.lookup)));
            return DecisionContext {
                source: Some(source),
                effects: abilities
                    .and_then(|a| a.get(index as usize))
                    .map_or(&[], |a| effects(a, mode)),
                x: self.activation_x.unwrap_or(0),
                ..DecisionContext::default()
            };
        }
        self.resolution
            .as_ref()
            .map_or_else(DecisionContext::default, |res| DecisionContext {
                source: Some(res.source),
                effects: res.effects.get(res.pc..).unwrap_or_default(),
                x: res.x.unwrap_or(0),
                ..DecisionContext::default()
            })
    }
}
