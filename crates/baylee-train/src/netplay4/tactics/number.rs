//! The numbers the tactical layer gives for the seat where the number is a
//! price (`--choice-guard tactical-scalar`): X, and floating mana spent to
//! prevent damage.
//!
//! More is not better where X is paid in life, or where what it buys stops
//! growing. So the layer reads what X buys where the spell's structure says
//! (a sweep: what each X ends on each side), and pays the least X that buys
//! the best of it; where it cannot read that and X costs life, it takes the
//! net's own number. Either way it never pays life into the other side's
//! visible swing. Elsewhere X is mana the seat has, and the whole of it stands.

use std::cmp::Reverse;

use baylee_cards_dsl::{AbilityDef, CostPart, Effect, PlayerRel, TargetSpec};
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::PlayerView;

use super::lookup_card_identity;
use super::rules::read::{actor, blockers_of, damage_through, hitters_of, life, opponents};
use super::sweep::{amount_of, sweep_toll, sweeps_by_x, swept, taken, x_steps};

/// What X is paid with, for a spell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::netplay4) struct XPrice {
    /// Mana: the mana cost has an X.
    pub(in crate::netplay4) mana: bool,
    /// Life: X life is an additional cost (CR 601.2f, 118.3).
    pub(in crate::netplay4) life: bool,
}

/// What the spell `id` pays X with, and the effects of its single body (none
/// for a modal spell: which mode is chosen the view does not say).
pub(in crate::netplay4) fn x_price(
    view: &PlayerView,
    id: ObjectId,
) -> Option<(XPrice, &'static [Effect])> {
    let identity = lookup_card_identity(view, id)?;
    let card = baylee_cards::by_index(identity.index)?;
    let face = card.faces.get(usize::from(identity.face))?;
    let price = XPrice {
        mana: face.mana_cost.has_variable(),
        life: face
            .mandatory_additional_costs
            .contains(&CostPart::PayLifeX),
    };
    let mut bodies = card
        .abilities_for_face(usize::from(identity.face))
        .iter()
        .filter_map(|ability| match ability {
            AbilityDef::Spell { effects, .. } => Some(Some(*effects)),
            AbilityDef::ModalSpell { .. } | AbilityDef::Modal { .. } => Some(None),
            _ => None,
        });
    let effects = match (bodies.next(), bodies.next()) {
        (Some(Some(effects)), None) => effects,
        _ => &[],
    };
    Some((price, effects))
}

/// The layer's X for the spell the seat is casting, from `min` to `max`, where
/// it has one; `None` where X is mana and what it buys is not read, and the
/// whole X stands. `net` is the net's own number.
pub(in crate::netplay4) fn x_value(view: &PlayerView, min: u32, max: u32, net: u32) -> Option<u32> {
    let me = actor(view);
    let (price, effects) = x_price(view, view.casting.or(view.announced_cast)?)?;
    if effects.iter().any(sweeps_by_x) {
        return Some(sweep_x(view, me, effects, price, min, max));
    }
    price
        .life
        .then(|| net.clamp(min, max).min(safe_life_x(view, me, min, max)))
}

/// The most life the seat can pay, from `min` to `max`, and keep more than
/// the swing the other side has on the table: `min` where none is that safe.
fn safe_life_x(view: &PlayerView, me: PlayerId, min: u32, max: u32) -> u32 {
    let blockers = blockers_of(view, me, &[]);
    let swing: i32 = opponents(view, me)
        .map(|player| damage_through(view, me, hitters_of(view, player), &blockers))
        .sum();
    let spare = u32::try_from(life(view, me) - swing.max(0) - 1).unwrap_or(0);
    spare.clamp(min, max.max(min))
}

/// What a sweep at one X leaves, as the layer weighs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Outcome {
    /// Every opponent's life gone.
    wins: bool,
    /// The seat keeps more life than the other side's swing at what is left.
    safe: bool,
    /// Their worth taken, less the seat's own and the life it pays.
    gain: i32,
}

/// The best X for the sweeps among `effects`: the safest, then the most gain,
/// then the least X. Never an X that leaves the seat without life.
fn sweep_x(
    view: &PlayerView,
    me: PlayerId,
    effects: &[Effect],
    price: XPrice,
    min: u32,
    max: u32,
) -> u32 {
    // No life total or toughness on a table comes near this; past it more X
    // buys nothing, and the sums below stay far from overflowing.
    const CEILING: u32 = 10_000;
    let low = i32::try_from(min.min(CEILING)).unwrap_or(0);
    let high = i32::try_from(max.min(CEILING)).unwrap_or(0);
    x_steps(view, me, effects, low, high)
        .into_iter()
        .filter_map(|x| Some((outcome(view, me, effects, price, x)?, Reverse(x))))
        .max()
        .and_then(|(_, Reverse(x))| u32::try_from(x).ok())
        .unwrap_or(min)
}

/// What the sweeps among `effects` do at X = `x`; `None` where the seat
/// would be left without life.
fn outcome(
    view: &PlayerView,
    me: PlayerId,
    effects: &[Effect],
    price: XPrice,
    x: i32,
) -> Option<Outcome> {
    let (mut mine, mut theirs) = (0, 0);
    for (filter, take) in effects.iter().filter_map(|effect| swept(effect, x)) {
        let (lost, gained) = sweep_toll(view, me, filter, take);
        mine += lost;
        theirs += gained;
    }
    let (to_me, to_them) = player_damage(effects, x);
    let paid = if price.life { x } else { 0 };
    let left = life(view, me) - paid - to_me;
    if left < 1 {
        return None;
    }
    let wins = to_them > 0 && opponents(view, me).all(|player| life(view, player) <= to_them);
    let blockers: Vec<_> = blockers_of(view, me, &[])
        .into_iter()
        .filter(|object| !taken(me, effects, x, object))
        .collect();
    let swing: i32 = opponents(view, me)
        .map(|player| {
            let hitters = hitters_of(view, player)
                .into_iter()
                .filter(|hitter| !taken(me, effects, x, hitter.object))
                .collect();
            damage_through(view, me, hitters, &blockers)
        })
        .sum();
    let foes = i32::try_from(opponents(view, me).count()).unwrap_or(1);
    Some(Outcome {
        wins,
        safe: left > swing,
        gain: theirs - mine - paid - to_me + to_them * foes,
    })
}

/// The damage `effects` deal to the seat and to each opponent at X = `x`.
fn player_damage(effects: &[Effect], x: i32) -> (i32, i32) {
    let (mut to_me, mut to_them) = (0, 0);
    for effect in effects {
        if let Effect::DealDamage {
            amount,
            target: TargetSpec::Player(who),
        } = effect
            && let Some(damage) = amount_of(*amount, x)
        {
            match who {
                PlayerRel::EachPlayer => {
                    to_me += damage;
                    to_them += damage;
                }
                PlayerRel::EachOpponent | PlayerRel::Opponent => to_them += damage,
                PlayerRel::You => to_me += damage,
                _ => {}
            }
        }
    }
    (to_me, to_them)
}

/// Floating mana spent to prevent damage: what prevents all of it, no more.
pub(in crate::netplay4) fn prevention(preventable: u32, min: u32, max: u32) -> u32 {
    preventable.clamp(min, max.max(min))
}
