//! Resolved, temporary permissions to take special actions (CR 116.2c).
use crate::choice::{GrantedActionKind, GrantedActionOffer};
use crate::event::{Cause, GameEvent};
use crate::prevention::{Shield, ShieldKind, ShieldOrigin, Shielded};
use crate::state::GameState;
use crate::zone::Zone;
use baylee_cards_dsl::{SpecialActionCost, SpecialActionTiming};
use baylee_core::ids::{GrantedActionId, PlayerId, TargetRef};
use baylee_core::mana::ManaColor;

/// One player's independently reusable permission.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GrantedAction {
    /// Player who received this permission; it is not controlled by its source.
    pub player: PlayerId,
    /// Immutable action description, retained even after source or recipient moves.
    pub offer: GrantedActionOffer,
}

impl GameState {
    /// Install one permission, giving it a never-reused identity.
    pub(crate) fn grant_action(&mut self, player: PlayerId, mut offer: GrantedActionOffer) {
        offer.id = GrantedActionId::new(self.next_granted_action);
        self.next_granted_action = self
            .next_granted_action
            .checked_add(1)
            .expect("granted action ID space");
        self.granted_actions.push(GrantedAction { player, offer });
    }

    /// Whether the payment and result fit the represented state, atomically.
    pub(crate) fn granted_action_payable(&self, action: &GrantedAction) -> bool {
        if self.has_left(action.player) {
            return false;
        }
        if let GrantedActionKind::AddMana { color, amount } = action.offer.effect
            && self.players[usize::from(action.player.get())]
                .mana_pool
                .available(color)
                .checked_add(u32::from(amount))
                .is_none()
        {
            return false;
        }
        match action.offer.cost {
            SpecialActionCost::Life(n) => {
                i32::try_from(n).is_ok_and(|n| self.can_pay_life(action.player, n))
            }
            SpecialActionCost::Mana(cost) => crate::casting::affordable(
                self,
                action.player,
                &self.players[usize::from(action.player.get())].mana_pool,
                &cost,
            ),
        }
    }

    /// Pay once and perform the immediate result; failure changes nothing.
    pub(crate) fn take_granted_action(&mut self, player: PlayerId, id: GrantedActionId) -> bool {
        let Some(action) = self
            .granted_actions
            .iter()
            .find(|g| g.player == player && g.offer.id == id)
            .cloned()
        else {
            return false;
        };
        if !self.granted_action_payable(&action) {
            return false;
        }
        match action.offer.cost {
            SpecialActionCost::Life(n) => {
                self.change_life(
                    player,
                    -i32::try_from(n).expect("checked life cost"),
                    Cause::Cost,
                );
            }
            SpecialActionCost::Mana(cost) => {
                if !crate::casting::pay_mana(self, player, &cost) {
                    return false;
                }
            }
        }
        match action.offer.effect {
            GrantedActionKind::AddMana { color, amount } => {
                self.players[usize::from(player.get())]
                    .mana_pool
                    .add(color, u32::from(amount));
                self.journal.record(GameEvent::ManaProduced {
                    player,
                    color,
                    amount,
                    source: Some(action.offer.source.object),
                });
            }
            GrantedActionKind::PreventNextDamage { target, amount } => {
                let protects = match target {
                    TargetRef::Player(p) if !self.has_left(p) => Shielded::Player(p),
                    TargetRef::Object(r)
                        if self.object(r.object).is_some_and(|o| {
                            o.version == r.version && o.zone == Zone::Battlefield
                        }) =>
                    {
                        Shielded::Object(r.object, r.version)
                    }
                    _ => return true,
                };
                if amount > 0 {
                    self.shields.push_from(
                        Shield {
                            protects,
                            kind: ShieldKind::Next(amount),
                            controller: player,
                        },
                        Some(ShieldOrigin {
                            source: action.offer.source.object,
                            ability: action.offer.ability,
                        }),
                    );
                }
            }
        }
        true
    }

    /// A conservative shared-resource bound for life-to-colorless permissions.
    /// Alternative grants do not each get to spend the same life again.
    pub(crate) fn granted_colorless_capacity(&self, player: PlayerId) -> u32 {
        self.granted_colorless_after_life(player, 0)
    }

    pub(crate) fn granted_colorless_after_life(&self, player: PlayerId, reserved: u32) -> u32 {
        if self.has_left(player) {
            return 0;
        }
        let life = u32::try_from(self.life_payable(player))
            .unwrap_or(0)
            .saturating_sub(reserved);
        let room = u32::MAX
            - self.players[usize::from(player.get())]
                .mana_pool
                .available(ManaColor::Colorless);
        self.granted_actions
            .iter()
            .filter(|g| g.player == player && g.offer.timing == SpecialActionTiming::ManaAbility)
            .filter_map(|g| match (g.offer.cost, g.offer.effect) {
                (
                    SpecialActionCost::Life(cost),
                    GrantedActionKind::AddMana {
                        color: ManaColor::Colorless,
                        amount,
                    },
                ) if cost > 0 => Some((life / cost).saturating_mul(u32::from(amount))),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            .min(room)
    }
}
