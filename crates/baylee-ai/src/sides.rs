//! A spell aimed at the side of the table it helps or hurts
//! ([`AIProfile::TARGET_SIDES`], generation 11).
//!
//! The target question of a spell whose effect reads no side
//! (`tactics::meaning`) falls back to the opponents' permanents first, and
//! an Aura spell's one effect is to attach itself (CR 303.4a), which reads
//! no side. So every Aura the house cast went on an opponent's creature
//! where it had one: a Holy Strength, a Rancor, a Serpent Skin made the
//! other side's creature bigger (Carbon's reading of the house's games
//! against `DeepSeek`, 11.11.2026: 36 Auras and pumps in 192 games, rank 1).
//! And a spell was cast whatever its targets were: a pump with no creature
//! of the house's own went to an opponent's, and a burn spell whose rider
//! the worth reader does not know (Broadside Barrage's loot) to the house's
//! own creature where the opponent had none.
//!
//! From generation 11 an Aura spell of this seat's own goes on the creature
//! its statics do the most for, by what they do to that creature's side
//! ([`HeuristicAgent::enchanting`]): a Holy Strength on its own creature, a
//! Pacifism on an opponent's. One whose statics the reader has no row for
//! (a control Aura) keeps the old answer. And a spell is not cast where
//! every target it could name is on the wrong side: an Aura none of whose
//! creatures gains, a spell that helps what it targets with no target on
//! this seat's side, one that hurts with no target on another's.

use baylee_cards_dsl::{AbilityDef, Effect, TargetReq};
use baylee_core::ids::ObjectId;
use baylee_core::types::TargetSide;
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::DecisionContext;
use baylee_view::{CardIdentity, PlayerView};

use crate::worth::{Aim, Origin};
use crate::{AIProfile, HeuristicAgent};

impl HeuristicAgent {
    /// Whether this level aims by side ([`AIProfile::TARGET_SIDES`]).
    pub(crate) const fn aims_by_side(&self) -> bool {
        self.profile.plays(AIProfile::TARGET_SIDES)
    }

    /// The creature this seat's own Aura spell goes on: the one its statics
    /// do the most for this seat's side; `None` where the question is no
    /// Aura's, or a static of it has no row in the reader.
    pub(crate) fn aura_target(
        &self,
        view: &PlayerView,
        objects: &[ObjectId],
        players: &[baylee_core::ids::PlayerId],
        min: u32,
        context: &DecisionContext<'_>,
    ) -> Option<PlayerAction> {
        if !self.aims_by_side() || min == 0 {
            return None;
        }
        let [Effect::AttachSelf { .. }] = context.effects else {
            return None;
        };
        let printed = context.printed?;
        let abilities = crate::controlled::aura(printed.card(), printed.face())?;
        let mut best: Option<(i64, Aim)> = None;
        for &id in objects {
            let value = self.enchanting(view, abilities, Aim::Object(id))?;
            if best.is_none_or(|(kept, _)| value > kept) {
                best = Some((value, Aim::Object(id)));
            }
        }
        for &player in players {
            let value = self.enchanting(view, abilities, Aim::Player(player))?;
            if best.is_none_or(|(kept, _)| value > kept) {
                best = Some((value, Aim::Player(player)));
            }
        }
        match best?.1 {
            Aim::Object(id) => Some(PlayerAction::ChooseTargets {
                objects: vec![id],
                players: vec![],
            }),
            Aim::Player(player) => Some(PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![player],
            }),
            Aim::Source | Aim::Nothing => None,
        }
    }

    /// Whether casting `card` (`id`, where the seat holds it) would name
    /// only targets on the wrong side: an Aura none of whose creatures gains
    /// this seat anything, a spell that helps what it targets with no target
    /// on this seat's side, or one that hurts with no target on another
    /// side. Spells with no required target, and targets the reader cannot
    /// list, are not judged here.
    pub(crate) fn aims_only_wrong(
        &self,
        view: &PlayerView,
        id: Option<ObjectId>,
        card: CardIdentity,
        spells: &[AbilityDef],
    ) -> bool {
        if !self.aims_by_side() {
            return false;
        }
        let Some(id) = id else {
            return false;
        };
        let origin = Origin::of(view, id);
        for ability in spells {
            let AbilityDef::Spell {
                effects,
                targets: Some(req),
                second_targets: None,
                ..
            } = ability
            else {
                continue;
            };
            if req.min == 0 {
                continue;
            }
            if let [Effect::AttachSelf { .. }] = effects
                && let Some(abilities) = crate::controlled::aura(card.index, card.face)
            {
                if self.no_creature_gains(view, origin, req, abilities) {
                    return true;
                }
                continue;
            }
            let side = self.side_of(effects);
            let Some(aims) = self.aims(view, origin, &req.spec) else {
                continue;
            };
            let ours = |aim: &Aim| match *aim {
                Aim::Object(object) => view
                    .object(object)
                    .map(|o| !self.hostile(o.controller, view.seat)),
                Aim::Player(player) => Some(!self.hostile(player, view.seat)),
                Aim::Source | Aim::Nothing => None,
            };
            let wrong = match side {
                Some(TargetSide::Helpful) => aims.iter().all(|aim| ours(aim) == Some(false)),
                Some(TargetSide::Harmful) => aims.iter().all(|aim| ours(aim) == Some(true)),
                _ => false,
            };
            if wrong && !aims.is_empty() {
                return true;
            }
        }
        false
    }

    /// Whether every creature an Aura printing `abilities` could enchant
    /// gains this seat nothing ([`Self::enchanting`]); `false` where a
    /// static has no row in that reader.
    fn no_creature_gains(
        &self,
        view: &PlayerView,
        origin: Origin<'_>,
        req: &TargetReq,
        abilities: &[AbilityDef],
    ) -> bool {
        let Some(aims) = self.aims(view, origin, &req.spec) else {
            return false;
        };
        let mut values = Vec::with_capacity(aims.len());
        for aim in aims {
            match self.enchanting(view, abilities, aim) {
                Some(value) => values.push(value),
                None => return false,
            }
        }
        values.iter().all(|value| *value <= 0)
    }
}

/// What `effect` does to what it names where the shared reader
/// (`baylee_engine::target_side`) has no sign for it, summed through what
/// the effect runs inside itself: a regeneration shield helps, a creature
/// set to a body of four or more points helps and to a smaller one hurts
/// ("becomes a 4/4" against "becomes a 0/1"), a permanent made a creature
/// is this seat's own to make, and a player made to discard is hurt by each
/// card. Zero for everything else.
fn unread_benefit(effect: &Effect) -> i32 {
    use baylee_cards_dsl::{Modifier, PlayerRel};
    let own = match effect {
        Effect::Regenerate { .. } => 1,
        Effect::CreateContinuousEffect { modifier, .. } => match modifier {
            Modifier::SetPT(power, toughness) => {
                if i32::from(*power) + i32::from(*toughness) >= 4 {
                    1
                } else {
                    -1
                }
            }
            Modifier::AddType(types) if types.contains(baylee_core::types::TypeSet::CREATURE) => 1,
            _ => 0,
        },
        Effect::DiscardForPlayers {
            who: PlayerRel::Chosen,
            count,
        } => -i32::from(*count),
        _ => 0,
    };
    let (then, otherwise) = effect.branches();
    own + then
        .iter()
        .chain(otherwise)
        .map(unread_benefit)
        .sum::<i32>()
}

/// The sign of what `effects` do to their second target: the power and
/// toughness a `PumpTargetSlot` for that slot gives or takes. `None` where
/// no effect names the second slot.
fn second_slot_sign(effects: &[Effect], x: u32) -> Option<i32> {
    use baylee_cards_dsl::TargetSlot;
    let mut sign = None;
    for effect in effects {
        if let Effect::PumpTargetSlot {
            slot: TargetSlot::Second,
            power,
            toughness,
            keywords,
            ..
        } = effect
        {
            let points = crate::tactics::amount(*power, x) + crate::tactics::amount(*toughness, x);
            let own = if points == 0 && keywords.bits() != 0 {
                1
            } else {
                points.signum()
            };
            sign = Some(sign.unwrap_or(0) + own);
        }
    }
    sign
}

impl HeuristicAgent {
    /// What the effects a target question is asked for mean, with the side
    /// readings of [`AIProfile::SIDE_READS`] beside the shared reader's: the
    /// rows of [`unread_benefit`], and for a spell's second target the sign
    /// of what it does to that target alone (Consume Strength's −2/−2 goes
    /// on an opponent's creature, Martial Glory's +0/+3 on this seat's).
    pub(crate) fn sided_meaning(&self, context: &DecisionContext<'_>) -> crate::tactics::Meaning {
        let mut m = crate::tactics::meaning(context.effects, context.x);
        if !self.profile.plays(AIProfile::SIDE_READS) {
            return m;
        }
        if context.second_instance
            && let Some(sign) = second_slot_sign(context.effects, context.x)
        {
            let mut second = crate::tactics::Meaning::default();
            second.benefit = sign;
            return second;
        }
        m.benefit += context.effects.iter().map(unread_benefit).sum::<i32>();
        m
    }

    /// The side `effects` want of what they name: the shared reader's, and
    /// where that has none, the sign of [`unread_benefit`] from
    /// [`AIProfile::SIDE_READS`].
    fn side_of(&self, effects: &[Effect]) -> Option<TargetSide> {
        let shared = baylee_engine::target_side::effects_side(effects, 0);
        if shared.is_some() || !self.profile.plays(AIProfile::SIDE_READS) {
            return shared;
        }
        match effects.iter().map(unread_benefit).sum::<i32>() {
            sign if sign > 0 => Some(TargetSide::Helpful),
            sign if sign < 0 => Some(TargetSide::Harmful),
            _ => None,
        }
    }
}
