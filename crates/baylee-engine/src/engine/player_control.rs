//! A decision's actor is distinct from the player whose resources it uses.

use super::{CardLookup, Engine, ObjectId, PlayerId};
use baylee_core::ids::DamageSourceRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Control {
    source: DamageSourceRef,
    controller: PlayerId,
    player: PlayerId,
}

/// Exact spells awaiting their later control segment, and currently resolving segments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct PlayerControl {
    waiting: Vec<Control>,
    active: Vec<Control>,
}

impl<L: CardLookup> Engine<L> {
    /// A necessary-condition rejection, never a complete mana-plan solver.
    /// With only cost-free mana producers remaining, generated mana has no
    /// consumer except the selected spell. Extra hypothetical resources make
    /// this an optimistic check: failure proves the activation cannot finish.
    pub(super) fn commanded_payment_dead_end(&self) -> bool {
        use baylee_core::mana::{ManaColor, ManaPool};
        let Some(super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::Miracle { wizard, cost, .. },
        }) = &self.mana_window
        else {
            return false;
        };
        let Some(obligation) = self
            .state
            .constrained_payment(*player)
            .filter(|payment| payment.card.object == wizard.card && !payment.required.is_empty())
        else {
            return false;
        };
        let Some(required) = crate::constrained_payment::amounts(&obligation.required) else {
            return true;
        };
        if self.land_mana_might_consume(*player) {
            return false;
        }
        let mut future = ManaPool::new();
        for color in ManaColor::ALL {
            future.add_snow(color, u32::MAX);
        }
        crate::mana_pay::payment_consuming(
            &future,
            cost,
            crate::casting::mana_spending(&self.state, *player),
            [0; 6],
            None,
            required,
        )
        .is_none()
    }

    /// Any potential mana cost or stateful mana effect defeats the simple
    /// no-consumer proof. In particular repeatable X=0 storage abilities are
    /// legal sinks, and filter lands may consume one color to produce another.
    fn land_mana_might_consume(&self, player: PlayerId) -> bool {
        use baylee_cards_dsl::{AbilityDef, Effect};
        let only_adds = |effects: &[Effect]| {
            effects
                .iter()
                .all(|effect| matches!(effect, Effect::AddMana { .. }))
        };
        for id in self.state.battlefield_seen() {
            let Some(object) = self.state.object(id) else {
                continue;
            };
            if object.controller != player
                || !object
                    .characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::LAND)
            {
                continue;
            }
            if crate::casting::activation_increase(&self.state, id) != 0 {
                return true;
            }
            for ability in object.abilities(&self.lookup) {
                match ability {
                    AbilityDef::Activated {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        cost,
                        effects,
                        mana_ability: true,
                        ..
                    } if cost.mana != baylee_core::mana::ManaCost::ZERO || !only_adds(effects) => {
                        return true;
                    }
                    _ => {}
                }
            }
            if crate::effects::granted_activated(&self.state, id).any(|ability| {
                ability.mana_ability
                    && (ability.cost.mana != baylee_core::mana::ManaCost::ZERO
                        || !only_adds(ability.effects))
            }) {
                return true;
            }
        }
        false
    }

    pub(super) fn controlled_actor(&self, player: PlayerId) -> PlayerId {
        let mut actor = player;
        let mut visited = baylee_core::ids::SeatSet::new();
        while !visited.contains(actor) {
            visited.insert(actor);
            let control = self
                .effect_plays
                .iter()
                .rev()
                .filter_map(super::effect_play::EffectPlay::control)
                .find(|&(_, subject)| subject == actor)
                .or_else(|| {
                    self.player_control
                        .active
                        .iter()
                        .rev()
                        .find(|entry| entry.player == actor)
                        .map(|entry| (entry.controller, entry.player))
                });
            let Some((controller, _)) = control else {
                break;
            };
            if self
                .state
                .players
                .get(controller.get() as usize)
                .is_none_or(crate::state::Player::has_lost)
            {
                break;
            }
            actor = controller;
        }
        actor
    }

    /// Other players whose private information this controller may inspect now.
    #[must_use]
    pub fn controlled_players(&self, viewer: PlayerId) -> baylee_core::ids::SeatSet {
        self.state
            .players
            .iter()
            .filter(|p| p.id != viewer && !p.has_lost() && self.may_inspect_private(viewer, p.id))
            .map(|p| p.id)
            .collect()
    }

    /// The special mana restrictions apply only while playing the selected card.
    pub(super) fn commanded_player(&self) -> Option<PlayerId> {
        self.effect_plays
            .last()
            .and_then(super::effect_play::EffectPlay::control)
            .map(|(_, player)| player)
    }

    pub(super) fn commanded_card(&self, card: ObjectId) -> bool {
        self.effect_plays
            .last()
            .is_some_and(|frame| frame.commanded_card() == Some(card))
    }

    pub(super) fn remember_controlled_spell(
        &mut self,
        card: ObjectId,
        controller: PlayerId,
        player: PlayerId,
    ) {
        if let Some(source) = self.state.source_identity(card) {
            self.player_control.waiting.push(Control {
                source,
                controller,
                player,
            });
        }
    }

    pub(super) fn begin_controlled_resolution(&mut self, card: ObjectId) {
        let Some(source) = self.state.source_identity(card) else {
            return;
        };
        if let Some(index) = self
            .player_control
            .waiting
            .iter()
            .position(|entry| entry.source == source)
        {
            self.player_control
                .active
                .push(self.player_control.waiting.remove(index));
        }
    }

    pub(super) fn end_controlled_resolution(&mut self, card: ObjectId) {
        self.player_control
            .active
            .retain(|entry| entry.source.object != card);
    }

    pub(super) fn prune_player_control(&mut self) {
        self.player_control.waiting.retain(|entry| {
            self.state
                .object(entry.source.object)
                .is_some_and(|object| {
                    object.version == entry.source.version
                        && object.zone == crate::zone::Zone::Stack
                })
        });
        // Entry replacement choices still belong to the resolving permanent spell.
        if self.entry_questions.is_empty() {
            self.player_control.active.retain(|entry| {
                self.state
                    .object(entry.source.object)
                    .is_some_and(|object| {
                        object.version == entry.source.version
                            && object.zone == crate::zone::Zone::Stack
                    })
            });
        }
    }
}
