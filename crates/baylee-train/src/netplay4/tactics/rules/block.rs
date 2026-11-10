//! Blocks the damage count does not see. The tactical layer blocks for life and
//! for kills a blocker survives; a Goblin Lackey that connects is worth a card
//! however little it hits for, and a fair trade thins the other side's board.

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

/// What a block is judged by: whether `blocker` should stand in `attacker`'s way.
pub(super) type BlockTest = fn(&PublicObject, &PublicObject) -> bool;

fn hit(attacker: &PublicObject) -> i32 {
    i32::from(attacker.power.unwrap_or(0)).max(0)
}

/// Whether `blocker` outlasts `attacker`, which it should stop for that alone.
pub(in crate::netplay4) fn lives(attacker: &PublicObject, blocker: &PublicObject) -> bool {
    hit(attacker) > 0
        && toughness_left(blocker) > hit(attacker)
        && !has(attacker, KeywordSet::DEATHTOUCH)
}

/// Whether `blocker` kills `attacker` in their fight, first strike counted.
pub(in crate::netplay4) fn kills(attacker: &PublicObject, blocker: &PublicObject) -> bool {
    let first = has(attacker, KeywordSet::FIRST_STRIKE)
        && !has(blocker, KeywordSet::FIRST_STRIKE)
        && !lives(attacker, blocker);
    !first
        && !has(attacker, KeywordSet::INDESTRUCTIBLE)
        && (i32::from(blocker.power.unwrap_or(0)) >= toughness_left(attacker)
            || has(blocker, KeywordSet::DEATHTOUCH))
}

/// Whether `blocker` is a fair price for stopping a connection: it lives, or
/// takes the attacker with it at no great loss, or is worth no more.
pub(super) fn stops_a_connection(attacker: &PublicObject, blocker: &PublicObject) -> bool {
    hits_for_value(attacker)
        && (lives(attacker, blocker)
            || (kills(attacker, blocker) && worth(blocker) <= worth(attacker) + 2)
            || worth(blocker) <= worth(attacker))
}

/// Whether `blocker` takes `attacker` with it at no loss of worth.
pub(super) fn is_a_fair_trade(attacker: &PublicObject, blocker: &PublicObject) -> bool {
    kills(attacker, blocker) && worth(blocker) <= worth(attacker)
}

fn may_solo_block_attacker(attacker: &PublicObject, pending: &Pending) -> bool {
    if has(attacker, KeywordSet::MENACE) {
        return false;
    }
    if let Pending::ChooseBlockers { bounds, .. } = pending {
        for bound in bounds {
            if bound.attacker == attacker.id && (bound.min_blockers > 1 || bound.max_blockers == 0)
            {
                return false;
            }
        }
    }
    true
}

/// The attackers on `me` a declaration left open that a free blocker passes
/// `test` for.
pub(super) fn left_open(
    view: &PlayerView,
    pending: &Pending,
    declared: &[(ObjectId, ObjectId)],
    test: BlockTest,
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
        .filter(|hitter| may_solo_block_attacker(hitter, pending))
        .filter(|hitter| {
            blockers.iter().any(|option| {
                !busy.contains(&option.blocker)
                    && option.attackers.contains(&hitter.id)
                    && view
                        .object(option.blocker)
                        .is_some_and(|blocker| test(hitter, blocker))
            })
        })
        .map(|hitter| hitter.id)
        .collect()
}

/// Sets the cheapest pair that passes `test` for each attacker the logits leave
/// open. Returns whether any block was added.
fn add_blocks(
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
    test: BlockTest,
) -> bool {
    if !matches!(pending, Pending::ChooseBlockers { .. }) {
        return false;
    }
    let me = actor(view);
    let n = logits.len().min(cands.pair_objects.len());
    let taken = |logits: &[f32]| -> Vec<(ObjectId, ObjectId)> {
        cands.pair_objects[..n]
            .iter()
            .zip(logits)
            .filter(|(_, logit)| **logit > 0.0)
            .map(|(pair, _)| *pair)
            .collect()
    };
    let declared = taken(logits);
    let mut busy: BTreeSet<ObjectId> = declared.iter().map(|pair| pair.0).collect();
    let blocked: BTreeSet<ObjectId> = declared.iter().map(|pair| pair.1).collect();
    let max_blks = if cands.max_choices > 0 {
        cands.max_choices as usize
    } else {
        usize::MAX
    };
    let mut open: Vec<&PublicObject> = view
        .combat
        .attackers
        .iter()
        .filter(|attacker| {
            attacker.defending == Defender::Player(me) && !blocked.contains(&attacker.creature)
        })
        .filter_map(|attacker| view.object(attacker.creature))
        .filter(|attacker| may_solo_block_attacker(attacker, pending))
        .collect();
    open.sort_by_key(|attacker| std::cmp::Reverse(hit(attacker)));
    let mut added = false;
    for hitter in open {
        if busy.len() >= max_blks {
            break;
        }
        let best = (0..n)
            .filter(|&index| cands.pair_objects[index].1 == hitter.id)
            .filter_map(|index| {
                let blocker = view.object(cands.pair_objects[index].0)?;
                (!busy.contains(&blocker.id) && test(hitter, blocker)).then_some((index, blocker))
            })
            .min_by_key(|(_, blocker)| worth(blocker));
        let Some((index, blocker)) = best else {
            continue;
        };
        let blocker_id = blocker.id;
        for (pair, logit) in cands.pair_objects.iter().zip(logits.iter_mut()) {
            if pair.0 == blocker_id {
                *logit = NEVER;
            }
        }
        logits[index] = SURE;
        busy.insert(blocker_id);
        added = true;
    }
    added
}

/// Blocks what would connect for value. Returns whether it added a block.
pub(super) fn stop_connections(
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
) -> bool {
    add_blocks(cands, logits, view, pending, stops_a_connection)
}

/// Takes the fair trades left open. Returns whether it added a block.
pub(super) fn take_trades(
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
) -> bool {
    add_blocks(cands, logits, view, pending, is_a_fair_trade)
}

/// Puts a blocker that lives in the way of what is left open.
pub(super) fn stand_firm(
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
) -> bool {
    add_blocks(cands, logits, view, pending, lives)
}

/// Whether a seat at `life` must not take `damage` this turn: it is left within
/// one swing of the same size, or nearly dead.
const fn too_much(life: i32, damage: i32) -> bool {
    damage > 0 && (life - damage <= 3 || (life - damage <= damage && damage >= 6))
}

/// The chumps that bring an attack down to what the seat can take, biggest
/// attacker first and cheapest blocker first, on top of `declared`. Empty when
/// the seat can take what comes, or when no chump helps.
pub(super) fn chumps(
    view: &PlayerView,
    pending: &Pending,
    declared: &[(ObjectId, ObjectId)],
) -> Vec<(ObjectId, ObjectId)> {
    let Pending::ChooseBlockers { blockers, .. } = pending else {
        return Vec::new();
    };
    let me = actor(view);
    let my_life = super::read::life(view, me);
    let mut busy: BTreeSet<ObjectId> = declared.iter().map(|pair| pair.0).collect();
    let blocked: BTreeSet<ObjectId> = declared.iter().map(|pair| pair.1).collect();
    let mut open: Vec<&PublicObject> = view
        .combat
        .attackers
        .iter()
        .filter(|attacker| {
            attacker.defending == Defender::Player(me) && !blocked.contains(&attacker.creature)
        })
        .filter_map(|attacker| view.object(attacker.creature))
        .collect();
    open.sort_by_key(|attacker| std::cmp::Reverse(hit(attacker)));
    let mut damage: i32 = open.iter().map(|attacker| hit(attacker)).sum();
    let mut added = Vec::new();
    for attacker in open {
        if !too_much(my_life, damage) {
            break;
        }
        if !may_solo_block_attacker(attacker, pending) || has(attacker, KeywordSet::TRAMPLE) {
            continue;
        }
        let chump = blockers
            .iter()
            .filter(|option| {
                !busy.contains(&option.blocker) && option.attackers.contains(&attacker.id)
            })
            .filter_map(|option| view.object(option.blocker))
            .min_by_key(|blocker| worth(blocker));
        if let Some(chump) = chump {
            busy.insert(chump.id);
            added.push((chump.id, attacker.id));
            damage -= hit(attacker);
        }
    }
    added
}

/// Adds the chumps [`chumps`] asks for. Returns whether it added a block.
pub(super) fn chump_for_life(
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
) -> bool {
    let n = logits.len().min(cands.pair_objects.len());
    let declared: Vec<(ObjectId, ObjectId)> = cands.pair_objects[..n]
        .iter()
        .zip(logits.iter())
        .filter(|(_, logit)| **logit > 0.0)
        .map(|(pair, _)| *pair)
        .collect();
    let max_blks = if cands.max_choices > 0 {
        cands.max_choices as usize
    } else {
        usize::MAX
    };
    let mut busy_count = declared.iter().map(|p| p.0).collect::<BTreeSet<_>>().len();
    let added = chumps(view, pending, &declared);
    let mut any_added = false;
    for (blocker, attacker) in &added {
        if busy_count >= max_blks {
            break;
        }
        for (pair, logit) in cands.pair_objects.iter().zip(logits.iter_mut()) {
            if pair.0 == *blocker {
                *logit = if pair.1 == *attacker { SURE } else { NEVER };
            }
        }
        busy_count += 1;
        any_added = true;
    }
    any_added
}
