//! An attack against the visible swing back. The tactical layer holds a blocker
//! when the swing back kills outright; this reads the race: a seat that cannot
//! outrun the other side keeps what it needs at home before the turn it dies on.

use std::collections::BTreeMap;

use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_engine::choice::Pending;
use baylee_view::PlayerView;

use super::read::{Hitter, actor, blockers_of, damage_through, has, hitters_of, is_tapped, life};
use crate::features4::EncodedCandidates4;

/// Turns left, the swing back counted as it stands, within which a seat stops
/// racing and defends.
const DEFEND_WITHIN: i32 = 2;

/// The logit that keeps an attacker at home, as the tactical layer's.
const STAY: f32 = -20.0;

/// The attackers the sampler would send on these logits: its greedy reading.
fn sent(cands: &EncodedCandidates4, logits: &[f32], required: &[ObjectId]) -> Vec<ObjectId> {
    let mut best: BTreeMap<ObjectId, (f32, f32)> = BTreeMap::new();
    for (cand, &logit) in cands.candidates.iter().zip(logits) {
        if let Some(id) = cand.object_id {
            let entry = best.entry(id).or_insert((f32::NEG_INFINITY, 0.0));
            entry.0 = entry.0.max(logit);
            entry.1 += 1.0 / (1.0 + (-logit).exp());
        }
    }
    best.into_iter()
        .filter(|(id, (top, chance))| {
            required.contains(id) || *top > 0.0 || (*chance > 0.50 && *top > -0.65)
        })
        .map(|(id, _)| id)
        .collect()
}

/// What `going` deals through the blocks `defender` has now.
fn swing_of(view: &PlayerView, defender: PlayerId, going: &[ObjectId]) -> i32 {
    let hitters: Vec<Hitter<'_>> = going
        .iter()
        .filter_map(|id| view.object(*id))
        .map(Hitter::of)
        .collect();
    let blockers: Vec<_> = view
        .battlefield_of(defender)
        .filter(|object| super::read::is_creature(object) && !is_tapped(object))
        .collect();
    damage_through(view, defender, hitters, &blockers)
}

/// What the other side swings back for with `going` tapped. A vigilant attacker
/// stays a blocker.
fn swing_back(view: &PlayerView, me: PlayerId, other: PlayerId, going: &[ObjectId]) -> i32 {
    let tapped: Vec<ObjectId> = going
        .iter()
        .copied()
        .filter(|id| {
            view.object(*id)
                .is_some_and(|object| !has(object, KeywordSet::VIGILANCE))
        })
        .collect();
    let blockers = blockers_of(view, me, &tapped);
    damage_through(view, me, hitters_of(view, other), &blockers)
}

/// Turns a clock of `per_turn` needs for `life`; none for no clock.
fn turns(life: i32, per_turn: i32) -> i32 {
    if per_turn <= 0 {
        i32::MAX
    } else {
        (life + per_turn - 1) / per_turn
    }
}

/// The attackers that should stay home so the seat does not lose the race,
/// smallest cost first.
pub(super) fn stay_home(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
) -> Vec<ObjectId> {
    let Pending::ChooseAttackers { required, .. } = pending else {
        return Vec::new();
    };
    decide(view, pending, sent(cands, logits, required))
}

/// Which of `going` should stay home, as the race stands.
pub(super) fn decide(
    view: &PlayerView,
    pending: &Pending,
    mut going: Vec<ObjectId>,
) -> Vec<ObjectId> {
    let Pending::ChooseAttackers {
        defenders,
        required,
        ..
    } = pending
    else {
        return Vec::new();
    };
    let me = actor(view);
    let Some(other) = defenders.iter().find_map(|defender| match defender {
        Defender::Player(player) => Some(*player),
        _ => None,
    }) else {
        return Vec::new();
    };
    if going.is_empty() {
        return Vec::new();
    }
    let my_life = life(view, me);
    let their_life = life(view, other);
    let clock = swing_of(view, other, &going);
    if clock >= their_life {
        return Vec::new();
    }
    let mut back = swing_back(view, me, other, &going);
    if turns(my_life, back) > DEFEND_WITHIN || turns(their_life, clock) <= turns(my_life, back) {
        return Vec::new();
    }
    let mut staying = Vec::new();
    while turns(my_life, back) <= DEFEND_WITHIN {
        // The one whose staying stops the most for what it gives up.
        let best = going
            .iter()
            .copied()
            .filter(|id| !required.contains(id))
            .map(|id| {
                let without: Vec<ObjectId> =
                    going.iter().copied().filter(|other| *other != id).collect();
                (id, swing_back(view, me, other, &without), without)
            })
            .filter(|(_, then, _)| *then < back)
            .min_by_key(|(id, then, _)| {
                let lost = view.object(*id).map_or(0, |object| i32::from(object.power.unwrap_or(0)));
                (*then, lost)
            });
        let Some((id, then, without)) = best else {
            break;
        };
        staying.push(id);
        going = without;
        back = then;
    }
    staying
}

/// Keeps `staying` at home on these logits.
pub(super) fn apply(cands: &EncodedCandidates4, logits: &mut [f32], staying: &[ObjectId]) {
    for (cand, logit) in cands.candidates.iter().zip(logits.iter_mut()) {
        if cand.object_id.is_some_and(|id| staying.contains(&id)) {
            *logit = STAY;
        }
    }
}
