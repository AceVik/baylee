//! Opt-in announce-before-payment candidates. Encoder 6 remains unchanged.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use baylee_core::ids::{Defender, ObjectId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;

use crate::features4::{
    Candidate4, EncodedCandidates4, EncodedState4, HeadKind4, MAX_CANDIDATES, encode_candidates4,
    pack_object_id,
};

/// A complete Priority menu cannot fit in the candidate ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateLimit7 {
    /// Number of distinct entries that would be needed.
    pub required: usize,
}

impl fmt::Display for CandidateLimit7 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "encoder 7 needs {} candidates; limit is {MAX_CANDIDATES}",
            self.required
        )
    }
}

impl std::error::Error for CandidateLimit7 {}

/// Preserves the legacy menu and appends distinct legal unpaid spell entries.
///
/// No existing candidate feature slot is reassigned. The `view` must come from
/// a context-aware observation path so an in-progress cast offers `CancelCast`.
/// Cancellation is offered here only in Priority/payment menus. Non-Priority
/// cast questions retain their legacy heads; their legal cancellation sideband
/// still needs a separate contract rather than a fake selectable target.
/// The original question, including its payment offers, is never mutated and
/// remains the authority used to validate/decode the chosen answer.
///
/// # Errors
/// Returns [`CandidateLimit7`] rather than silently truncating a Priority menu,
/// including when the legacy portion alone would already have been truncated.
pub fn encode_candidates7(
    state: &EncodedState4,
    view: &PlayerView,
    pending: &Pending,
) -> Result<EncodedCandidates4, CandidateLimit7> {
    let Pending::Priority { player, legal } = pending else {
        return Ok(encode_candidates4(state, view, pending));
    };
    let existing: BTreeSet<_> = legal.castable.iter().copied().collect();
    let mut seen = existing;
    let additional: Vec<_> = legal
        .payable
        .iter()
        .copied()
        .filter(|id| seen.insert(*id))
        .collect();
    let required = usize::from(legal.can_pass)
        + usize::from(view.casting.is_some())
        + legal.lands.len()
        + legal.castable.len()
        + legal.suspendable.len()
        + legal.mana_abilities.len()
        + legal.abilities.len()
        + legal.granted_actions.len()
        + additional.len();
    if required > MAX_CANDIDATES {
        return Err(CandidateLimit7 { required });
    }
    let mut out = encode_candidates4(state, view, pending);
    if additional.is_empty() {
        return Ok(out);
    }
    // Reuse the exact CastSpell feature/entity encoding without reordering the
    // original candidates. Other legal fields remain intact in this local copy.
    let mut announced = legal.clone();
    announced.castable = additional;
    let extra = encode_candidates4(
        state,
        view,
        &Pending::Priority {
            player: *player,
            legal: announced,
        },
    );
    out.candidates.extend(
        extra
            .candidates
            .into_iter()
            .filter(|candidate| matches!(candidate.action, Some(PlayerAction::CastSpell { .. }))),
    );
    Ok(out)
}

/// Complete Encoder 7 candidate encoder (`encode_candidates7_grouped`):
/// - Deduplicates symmetric `SingleChoice` candidates acting on the same grouped battlefield permanent
///   (storing all member `ObjectId`s in `Candidate4::group_object_ids` so softmax mass is never split
///   across identical untapped lands/tokens),
/// - Populates within-group rank features (`feat[46..48]`) on `SubsetPick` candidates for grouped permanents.
/// - Note: `legal.payable` (announce-before-payment) is intentionally omitted here because `baylee-ai`
///   always taps mana abilities before casting from `legal.castable`, and `legal.payable` includes
///   spells whose mana costs cannot actually be paid by the available lands.
#[must_use]
pub fn encode_candidates7_grouped(
    state: &EncodedState4,
    view: &PlayerView,
    pending: &Pending,
) -> EncodedCandidates4 {
    let mut out = encode_candidates4(state, view, pending);

    if state.entity_group_members.is_empty() {
        return out;
    }

    match out.head {
        HeadKind4::SingleChoice => {
            deduplicate_grouped_single_choices(state, &mut out.candidates);
        }
        HeadKind4::SubsetPick => {
            populate_grouped_subset_ranks(state, &mut out.candidates);
        }
        HeadKind4::PairPick | HeadKind4::ScalarValue => {}
    }

    out
}

fn deduplicate_grouped_single_choices(state: &EncodedState4, candidates: &mut Vec<Candidate4>) {
    let mut deduped = Vec::<Candidate4>::with_capacity(candidates.len());
    let mut group_slot = BTreeMap::<(u8, i16, u32), usize>::new();

    for mut cand in std::mem::take(candidates) {
        let eidx = cand.entity_idx;
        let is_multi_group = eidx >= 0
            && state
                .entity_group_members
                .get(eidx as usize)
                .is_some_and(|m| m.len() > 1);
        if !is_multi_group {
            deduped.push(cand);
            continue;
        }

        let key_and_obj: Option<((u8, i16, u32), ObjectId)> = match &cand.action {
            Some(PlayerAction::ActivateManaAbility { source }) => Some(((1, eidx, 0), *source)),
            Some(PlayerAction::ActivateAbility {
                source,
                ability_index,
            }) => Some(((2, eidx, *ability_index), *source)),
            Some(PlayerAction::ChooseObjects { objects }) if objects.len() == 1 => {
                Some(((3, eidx, 0), objects[0]))
            }
            Some(PlayerAction::ChooseTargets { objects, players })
                if objects.len() == 1 && players.is_empty() =>
            {
                Some(((4, eidx, 0), objects[0]))
            }
            _ => None,
        };

        if let Some((key, oid)) = key_and_obj {
            if let Some(&canon_idx) = group_slot.get(&key) {
                let canon = &mut deduped[canon_idx];
                if !canon.group_object_ids.contains(&oid) {
                    canon.group_object_ids.push(oid);
                }
                canon.feat[46] = (canon.group_object_ids.len() as f32) / 10.0;
            } else {
                cand.group_object_ids.push(oid);
                cand.feat[46] = 0.1;
                let idx = deduped.len();
                group_slot.insert(key, idx);
                deduped.push(cand);
            }
        } else {
            deduped.push(cand);
        }
    }

    *candidates = deduped;
}

fn populate_grouped_subset_ranks(state: &EncodedState4, candidates: &mut [Candidate4]) {
    let mut groups = BTreeMap::<(i16, i8, u64), Vec<usize>>::new();
    for (idx, cand) in candidates.iter().enumerate() {
        let eidx = cand.entity_idx;
        let is_multi_group = eidx >= 0
            && state
                .entity_group_members
                .get(eidx as usize)
                .is_some_and(|m| m.len() > 1);
        if !is_multi_group {
            continue;
        }
        let def_key = match cand.defender {
            None => 0,
            Some(Defender::Player(p)) => 1 + u64::from(p.get()),
            Some(Defender::Planeswalker(id)) => (1 << 16) | pack_object_id(id),
            Some(Defender::Battle(id)) => (2 << 16) | pack_object_id(id),
        };
        groups
            .entry((eidx, cand.rel_seat_idx, def_key))
            .or_default()
            .push(idx);
    }

    for indices in groups.values() {
        if indices.len() <= 1 {
            continue;
        }
        let total = indices.len() as f32;
        for (rank, &i) in indices.iter().enumerate() {
            candidates[i].feat[46] = (rank as f32) / total;
            candidates[i].feat[47] = rank as f32;
        }
    }
}

#[cfg(test)]
mod tests;
