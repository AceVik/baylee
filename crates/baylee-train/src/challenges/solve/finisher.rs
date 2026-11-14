//! A visible-information alternative strategy: reserve a burn spell and use it for lethal.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

#[derive(Default)]
pub(super) struct Finisher {
    target: Option<PlayerId>,
    anchor: Option<ObjectId>,
}

impl Finisher {
    pub(super) fn preserve_combat_finish(view: &PlayerView, pending: &mut Pending) {
        if view.owed.is_some() {
            return;
        }
        let Some(enemy) = view.seats.iter().find(|seat| seat.player != view.seat) else {
            return;
        };
        let Pending::Priority { legal, .. } = pending else {
            return;
        };
        // The ordinary policy otherwise takes a visible burn lethal before combat.
        // Withhold only potentially lethal spells, retaining earlier interaction.
        let lethal = |id: &ObjectId| {
            view.hand
                .iter()
                .find(|card| card.id == *id)
                .and_then(|card| damage(&card.name))
                .is_some_and(|amount| amount >= enemy.life)
        };
        legal.castable.retain(|id| !lethal(id));
        legal.payable.retain(|id| !lethal(id));
    }
    pub(super) fn reserve(&mut self, view: &PlayerView, pending: &mut Pending) {
        if view.owed.is_some() {
            return;
        }
        let Pending::Priority { legal, .. } = pending else {
            return;
        };
        if !self
            .anchor
            .is_some_and(|id| view.hand.iter().any(|card| card.id == id))
        {
            self.anchor = view
                .hand
                .iter()
                .find(|card| damage(&card.name).is_some())
                .map(|card| card.id);
        }
        // Keep one finisher, not every copy of a removal spell. The rest remain
        // available to the ordinary policy for survival and board development.
        legal.castable.retain(|id| Some(*id) != self.anchor);
        legal.payable.retain(|id| Some(*id) != self.anchor);
    }
    pub(super) fn preserve_burn_finish(
        view: &PlayerView,
        pending: &Pending,
        action: &mut PlayerAction,
    ) {
        let Pending::ChooseAttackers {
            required,
            minimum_required,
            ..
        } = pending
        else {
            return;
        };
        if !required.is_empty()
            || minimum_required.is_some_and(|n| n > 0)
            || !view.hand.iter().any(|card| damage(&card.name).is_some())
        {
            return;
        }
        let Some(enemy) = view.seats.iter().find(|seat| seat.player != view.seat) else {
            return;
        };
        let PlayerAction::DeclareAttackers { attackers } = action else {
            return;
        };
        // Removing optional attackers preserves all pair/group restrictions. Use only
        // public power as an upper bound; blockers can reduce the damage further.
        let mut budget = enemy.life.saturating_sub(1).max(0);
        attackers.retain(|(id, defender)| {
            if *defender != baylee_core::ids::Defender::Player(enemy.player) {
                return true;
            }
            let power = view
                .object(*id)
                .and_then(|card| card.power)
                .map_or(0, i32::from)
                .max(0);
            if power > budget {
                false
            } else {
                budget -= power;
                true
            }
        });
    }

    pub(super) fn answer(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        pressure: bool,
    ) -> Option<PlayerAction> {
        match pending {
            Pending::Priority { legal, .. } if view.owed.is_none() && view.stack.is_empty() => {
                let enemy = view
                    .seats
                    .iter()
                    .find(|seat| seat.player != view.seat && seat.life > 0)?;
                let card = view.hand.iter().find(|card| {
                    damage(&card.name).is_some_and(|damage| pressure || damage >= enemy.life)
                        && affordable(view, legal, card)
                })?;
                self.target = Some(enemy.player);
                Some(PlayerAction::CastSpell { card: card.id })
            }
            Pending::ChooseTargets { player_options, .. } => {
                let target = self
                    .target
                    .filter(|target| player_options.contains(target))?;
                self.target = None;
                Some(PlayerAction::ChooseTargets {
                    objects: Vec::new(),
                    players: vec![target],
                })
            }
            _ => None,
        }
    }
}

fn affordable(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
    card: &baylee_view::HandObject,
) -> bool {
    if legal.castable.contains(&card.id) {
        return true;
    }
    if !legal.payable.contains(&card.id) {
        return false;
    }
    let Some(face) = baylee_cards::by_index(card.card.index)
        .and_then(|def| def.faces.get(usize::from(card.card.face)))
    else {
        return false;
    };
    let Some(seat) = view.seat(view.seat) else {
        return false;
    };
    let mode = baylee_engine::choice::CastModeKind::Normal;
    let cost = baylee_ai::spell_price(view, card.id, Some(card.card), legal, mode, face.mana_cost);
    baylee_ai::plan_for_spell(&cost, &seat.mana_pool, view, legal, card.id, mode).is_some()
}

pub(super) fn damage(name: &str) -> Option<i32> {
    match name {
        "Lightning Bolt" | "Incinerate" | "Volcanic Hammer" | "Chain Lightning" => Some(3),
        "Shock" => Some(2),
        _ => None,
    }
}
