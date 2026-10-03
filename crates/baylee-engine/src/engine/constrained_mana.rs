//! Resumable mana instructions share activation legality with ordinary payment.

use crate::choice::LegalActions;
use crate::state::GameState;
use crate::zone::Zone;
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};
use baylee_core::types::TypeSet;

/// Source restrictions of the instruction currently asking for mana.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum ManaActivationScope {
    /// Only mana abilities of lands controlled by the paying player.
    ControlledLands,
    /// One activation from each still-present listed incarnation.
    ListedLands(Vec<DamageSourceRef>),
}

impl ManaActivationScope {
    pub(super) fn admits(&self, state: &GameState, player: PlayerId, source: ObjectId) -> bool {
        let Some(object) = state.object(source) else {
            return false;
        };
        object.zone == Zone::Battlefield
            && object.controller == player
            && object.characteristics().types.contains(TypeSet::LAND)
            && match self {
                Self::ListedLands(lands) => lands.contains(&DamageSourceRef {
                    object: source,
                    version: object.version,
                }),
                Self::ControlledLands => true,
            }
    }

    /// Apply only to an offer already narrowed to mana abilities. Special
    /// actions are not mana abilities of a land, even when they make mana.
    pub(super) fn narrow(&self, state: &GameState, player: PlayerId, legal: &mut LegalActions) {
        legal
            .mana_abilities
            .retain(|&source| self.admits(state, player, source));
        legal
            .abilities
            .retain(|&(source, _)| self.admits(state, player, source));
        legal
            .unpaid_abilities
            .retain(|&(source, _, _)| self.admits(state, player, source));
        legal.granted_actions.clear();
    }
}

/// The exact lands an instruction requires to be activated, and the eventual
/// receiver of all unspent mana. Costs and mana choices use the usual wizard.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct LandManaSequence {
    pub(super) player: PlayerId,
    pub(super) beneficiary: PlayerId,
    remaining: Vec<DamageSourceRef>,
}

impl LandManaSequence {
    pub(super) fn new(state: &GameState, player: PlayerId, beneficiary: PlayerId) -> Self {
        let remaining = state
            .battlefield_seen()
            .filter_map(|id| {
                let object = state.object(id)?;
                (object.controller == player
                    && object.characteristics().types.contains(TypeSet::LAND))
                .then_some(DamageSourceRef {
                    object: id,
                    version: object.version,
                })
            })
            .collect();
        Self {
            player,
            beneficiary,
            remaining,
        }
    }

    pub(super) fn scope(&self) -> ManaActivationScope {
        ManaActivationScope::ListedLands(self.remaining.clone())
    }

    /// Called only when activation costs were successfully paid. A failed
    /// announcement does not use up the land's required activation.
    pub(super) fn activated(&mut self, source: DamageSourceRef) -> bool {
        let Some(index) = self.remaining.iter().position(|&land| land == source) else {
            return false;
        };
        self.remaining.remove(index);
        true
    }

    /// CR 106.13 moves the same mana, not newly produced substitute units.
    /// The caller resumes its interrupted resolution only after this succeeds.
    pub(super) fn transfer(&self, state: &mut GameState) -> bool {
        if self.player == self.beneficiary {
            return true;
        }
        let from = self.player.get() as usize;
        let to = self.beneficiary.get() as usize;
        if from >= state.players.len() || to >= state.players.len() {
            return false;
        }
        if from < to {
            let (first, second) = state.players.split_at_mut(to);
            first[from].mana_pool.transfer_to(&mut second[0].mana_pool)
        } else {
            let (first, second) = state.players.split_at_mut(from);
            second[0].mana_pool.transfer_to(&mut first[to].mana_pool)
        }
    }
}

/// One suspended effect, kept outside the resolution slot used by mana abilities.
#[derive(Clone, Debug)]
pub(super) struct LandManaWork {
    pub(super) resolution: crate::resolve::Resolution,
    sequence: LandManaSequence,
    choice: crate::choice::ManaChoiceId,
}

impl LandManaWork {
    pub(super) fn fingerprint(&self) -> u64 {
        crate::state::structural_fingerprint(&(&self.sequence, self.choice))
            .wrapping_mul(31)
            .wrapping_add(self.resolution.program_fingerprint())
    }

    fn activated(&mut self, source: DamageSourceRef) {
        if self.sequence.activated(source) {
            self.choice.step += 1;
        }
    }
}

impl super::PaymentContinuation {
    pub(super) fn note_mana_activation(&mut self, source: DamageSourceRef) {
        match self {
            Self::LandMana(work) => work.activated(source),
            Self::Activation(payment) => {
                if let Some(previous) = &mut payment.previous {
                    previous.suspended.note_mana_activation(source);
                }
            }
            _ => {}
        }
    }
}

impl<L: crate::state::CardLookup> super::Engine<L> {
    pub(super) fn note_mana_activation(&mut self, source: DamageSourceRef) {
        if let Some(window) = &mut self.mana_window {
            window.suspended.note_mana_activation(source);
        }
    }

    /// Convert the resolution's instruction into a nested mana opportunity.
    pub(super) fn open_land_mana_window(&mut self) -> bool {
        let Some(crate::resolve::AwaitingOp::LandMana {
            player,
            beneficiary,
        }) = self
            .resolution
            .as_ref()
            .and_then(|res| res.awaiting.as_ref())
        else {
            return false;
        };
        let (player, beneficiary) = (*player, *beneficiary);
        let Some(resolution) = self.resolution.take() else {
            return false;
        };
        let source = self
            .state
            .source_identity(resolution.on_stack)
            .expect("resolving stack object");
        self.mana_window = Some(super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::LandMana(Box::new(LandManaWork {
                sequence: LandManaSequence::new(&self.state, player, beneficiary),
                resolution,
                choice: crate::choice::ManaChoiceId { source, step: 0 },
            })),
        });
        self.awaiting_answer = false;
        true
    }

    /// Ordinary legality determines which mandatory activation remains possible.
    pub(super) fn advance_land_mana(&mut self) -> bool {
        let Some(super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::LandMana(work),
        }) = &self.mana_window
        else {
            return false;
        };
        let (player, scope, choice) = (*player, work.sequence.scope(), work.choice);
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        scope.narrow(&self.state, player, &mut legal);
        let options = legal
            .mana_abilities
            .iter()
            .map(|&source| (source, None))
            .chain(
                legal
                    .abilities
                    .iter()
                    .map(|&(source, index)| (source, Some(index))),
            )
            .filter_map(|(source, ability_index)| {
                self.state
                    .source_identity(source)
                    .map(|source| crate::choice::ManaAbilityChoice {
                        source,
                        ability_index,
                    })
            })
            .collect::<Vec<_>>();
        if !options.is_empty() {
            self.pending = crate::choice::Pending::ChooseManaAbility {
                player,
                choice,
                options,
            };
            self.awaiting_answer = true;
            return true;
        }
        let Some(super::PaymentWindow {
            suspended: super::PaymentContinuation::LandMana(mut work),
            ..
        }) = self.mana_window.take()
        else {
            return false;
        };
        if !work.sequence.transfer(&mut self.state) {
            self.state.numeric_failure = Some("mana transfer exceeds u32 per color");
            return true;
        }
        work.resolution.awaiting = None;
        work.resolution.pc += 1;
        match crate::resolve::run(&mut self.state, &mut work.resolution) {
            crate::resolve::Flow::Complete => self.finish_resolution(&work.resolution),
            crate::resolve::Flow::Wait(pending) => {
                self.resolution = Some(work.resolution);
                self.pending = pending;
                self.awaiting_answer = true;
            }
        }
        self.awaiting_answer
    }

    pub(super) fn answer_land_mana(
        &mut self,
        player: PlayerId,
        source: DamageSourceRef,
        ability_index: Option<u32>,
    ) -> Result<(), super::EngineError> {
        // Reuse the normal activation wizard after the exact instruction offer
        // has authenticated this answer. Its cost/color choices remain ordinary.
        self.pending = crate::choice::Pending::Priority {
            player,
            legal: Box::new(self.compute_legal(player)),
        };
        self.apply_inner(
            player,
            ability_index.map_or(
                crate::choice::PlayerAction::ActivateManaAbility {
                    source: source.object,
                },
                |ability_index| crate::choice::PlayerAction::ActivateAbility {
                    source: source.object,
                    ability_index,
                },
            ),
        )
    }
}
