//! A block the damage count does not see: an attacker that does something when
//! it connects. The tactical layer blocks for life and for kills; a Goblin
//! Lackey that gets through is worth a card however little it hits for.

use std::collections::BTreeSet;

use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::{Defender, ObjectId};
use baylee_engine::choice::Pending;
use baylee_view::{PlayerView, PublicObject};

use super::read::{actor, has, hits_for_value, toughness_left, worth};
use crate::features4::EncodedCandidates4;

/// A pair the sampler takes for certain.
const SURE: f32 = 8.0;
/// A pair it will not take.
const NEVER: f32 = -8.0;

/// Whether `blocker` is a fair price for stopping `attacker`: it lives, or it
/// takes the attacker with it at no great loss, or it is cheap.
fn worth_the_block(attacker: &PublicObject, blocker: &PublicObject) -> bool {
    let hit = i32::from(attacker.power.unwrap_or(0)).max(0);
    let lives = toughness_left(blocker) > hit && !has(attacker, KeywordSet::DEATHTOUCH);
    let kills = i32::from(blocker.power.unwrap_or(0)) >= toughness_left(attacker)
        || has(blocker, KeywordSet::DEATHTOUCH);
    lives || (kills && worth(blocker) <= worth(attacker) + 2) || worth(blocker) <= worth(attacker)
}

/// The attackers a declaration left connecting that a free blocker could stop.
pub(super) fn left_open(
    view: &PlayerView,
    pending: &Pending,
    declared: &[(ObjectId, ObjectId)],
) -> Vec<ObjectId> {
    let Pending::ChooseBlockers { blockers, .. } = pending else {
        return Vec::new();
    };
    let me = actor(view);
    let busy: BTreeSet<ObjectId> = declared.iter().map(|pair| pair.0).collect();
    let blocked: BTreeSet<ObjectId> = declared.iter().map(|pair| pair.1).collect();
    view.combat
        .attackers
        .iter()
        .filter(|attacker| {
            attacker.defending == Defender::Player(me) && !blocked.contains(&attacker.creature)
        })
        .filter_map(|attacker| view.object(attacker.creature))
        .filter(|hitter| hits_for_value(hitter) && !has(hitter, KeywordSet::MENACE))
        .filter(|hitter| {
            blockers.iter().any(|option| {
                !busy.contains(&option.blocker)
                    && option.attackers.contains(&hitter.id)
                    && view
                        .object(option.blocker)
                        .is_some_and(|blocker| worth_the_block(hitter, blocker))
            })
        })
        .map(|hitter| hitter.id)
        .collect()
}

/// Sets the pairs that stop a connecting trigger. Returns whether it changed a
/// block the layer would have left open.
pub(super) fn stop_connections(
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
) -> bool {
    if !matches!(pending, Pending::ChooseBlockers { .. }) {
        return false;
    }
    let me = actor(view);
    let n = logits.len().min(cands.pair_objects.len());
    let chosen = |logit: f32| logit > 0.0;
    let mut busy: BTreeSet<ObjectId> = cands.pair_objects[..n]
        .iter()
        .zip(logits.iter())
        .filter(|(_, logit)| chosen(**logit))
        .map(|(pair, _)| pair.0)
        .collect();
    let blocked: BTreeSet<ObjectId> = cands.pair_objects[..n]
        .iter()
        .zip(logits.iter())
        .filter(|(_, logit)| chosen(**logit))
        .map(|(pair, _)| pair.1)
        .collect();
    let mut changed = false;
    for attacker in &view.combat.attackers {
        if attacker.defending != Defender::Player(me) || blocked.contains(&attacker.creature) {
            continue;
        }
        let Some(hitter) = view.object(attacker.creature) else {
            continue;
        };
        if !hits_for_value(hitter) || has(hitter, KeywordSet::MENACE) {
            continue;
        }
        let best = (0..n)
            .filter(|&index| cands.pair_objects[index].1 == attacker.creature)
            .filter_map(|index| {
                let blocker = view.object(cands.pair_objects[index].0)?;
                (!busy.contains(&blocker.id) && worth_the_block(hitter, blocker))
                    .then_some((index, blocker))
            })
            .min_by_key(|(_, blocker)| worth(blocker));
        let Some((index, blocker)) = best else {
            continue;
        };
        let blocker_id = blocker.id;
        for other in 0..n {
            if cands.pair_objects[other].0 == blocker_id {
                logits[other] = NEVER;
            }
        }
        logits[index] = SURE;
        busy.insert(blocker_id);
        changed = true;
    }
    changed
}
