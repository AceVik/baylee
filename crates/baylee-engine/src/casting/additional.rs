//! Distinct cards pay distinct additional costs; maximize usable escape fodder.
use crate::{state::GameState, zone::ZoneLocation};
use baylee_cards_dsl::{CostPart, FaceDef};
use baylee_core::ids::{ObjectId, PlayerId};

pub(crate) fn additional_card_enters_graveyard(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    let destination = ZoneLocation::Graveyard(player);
    state
        .object(card)
        .is_some_and(|object| object.is_card() && object.owner == player)
        && crate::replacement::graveyard_destination(state, card, destination).0 == destination
}

/// Augmenting paths match cards to cost parts. Visiting graveyard-bound cards
/// first maximizes their count; later augmentations preserve already matched
/// cards. This avoids enumerating combinations of a large battlefield.
pub(crate) fn additional_payment_capacity(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static FaceDef,
    chosen: &[ObjectId],
) -> Option<usize> {
    additional_payment_capacity_for(state, player, card, face, None, chosen)
}

pub(crate) fn additional_payment_capacity_for(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static FaceDef,
    option: Option<crate::choice::CastModeKind>,
    chosen: &[ObjectId],
) -> Option<usize> {
    let life = crate::engine::cast_wizard::wizard_mandatory_parts(face, option).fold(
        0_i32,
        |sum, part| {
            sum.saturating_add(if let CostPart::PayLife(life) = part {
                i32::from(*life)
            } else {
                0
            })
        },
    );
    if !state.can_pay_life(player, life) {
        return None;
    }
    let menus: Vec<Vec<_>> = crate::engine::cast_wizard::additional_card_costs_for(face, option)
        .skip(chosen.len())
        .map(|part| {
            crate::engine::cost_wizard::options(state, player, card, part)
                .into_iter()
                .filter(|id| *id != card && !chosen.contains(id))
                .collect()
        })
        .collect();
    let mut cards: Vec<_> = menus.iter().flatten().copied().collect();
    cards.sort_unstable();
    cards.dedup();
    // Replacement evaluation can inspect continuous effects. Cache it once per
    // candidate instead of repeating it inside the sorting comparator.
    let mut candidates: Vec<_> = cards
        .into_iter()
        .map(|id| (id, additional_card_enters_graveyard(state, player, id)))
        .collect();
    candidates.sort_by_key(|(_, enters_graveyard)| !enters_graveyard);
    let assignments = preferred_assignment(&menus, candidates.iter().map(|(id, _)| *id))?;
    let mut paid: Vec<_> = chosen.iter().copied().chain(assignments).collect();
    if crate::engine::cast_wizard::wizard_mandatory_parts(face, option)
        .any(|p| matches!(p, CostPart::DiscardHand))
    {
        paid.extend(
            state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .copied()
                .filter(|id| *id != card),
        );
    }
    paid.sort_unstable();
    paid.dedup();
    Some(
        paid.into_iter()
            .filter(|id| additional_card_enters_graveyard(state, player, *id))
            .count(),
    )
}

/// The left-side subsets matchable to distinct cost parts form a transversal
/// matroid. Processing preferred cards first therefore maximizes their number
/// among complete matchings: augmentation never drops a previously matched card.
/// The caller supplies distinct candidates in descending preference order.
fn preferred_assignment(
    menus: &[Vec<ObjectId>],
    candidates: impl IntoIterator<Item = ObjectId>,
) -> Option<Vec<ObjectId>> {
    let mut assignments = vec![None; menus.len()];
    let mut visited = vec![false; menus.len()];
    let mut matched = 0;
    for candidate in candidates {
        if matched == menus.len() {
            break;
        }
        visited.fill(false);
        matched += usize::from(augment(candidate, menus, &mut assignments, &mut visited));
    }
    assignments.into_iter().collect()
}

fn augment(
    card: ObjectId,
    menus: &[Vec<ObjectId>],
    assignments: &mut [Option<ObjectId>],
    visited: &mut [bool],
) -> bool {
    for index in 0..menus.len() {
        if visited[index] || !menus[index].contains(&card) {
            continue;
        }
        visited[index] = true;
        if assignments[index].is_none_or(|old| augment(old, menus, assignments, visited)) {
            assignments[index] = Some(card);
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute_force_capacity(menus: &[Vec<ObjectId>], preferred: u32, used: u32) -> Option<usize> {
        let Some((menu, rest)) = menus.split_first() else {
            return Some(0);
        };
        menu.iter()
            .filter_map(|card| {
                let bit = 1 << card.slot();
                if used & bit != 0 {
                    return None;
                }
                brute_force_capacity(rest, preferred, used | bit)
                    .map(|other| other + usize::from(preferred & bit != 0))
            })
            .max()
    }

    #[test]
    fn additional_cost_matching_is_optimal_for_every_small_graph() {
        // Every graph on up to four cards and three costs, and every subset
        // of cards whose payments enter the graveyard. Brute force assigns
        // each cost independently, without augmenting paths or greedy choices.
        for card_count in 0_u32..=4 {
            for cost_count in 0_u32..=3 {
                for edges in 0_u32..(1 << (card_count * cost_count)) {
                    let menus: Vec<Vec<_>> = (0..cost_count)
                        .map(|cost| {
                            (0..card_count)
                                .filter(|card| edges & (1 << (cost * card_count + card)) != 0)
                                .map(|card| ObjectId::new(card, 0))
                                .collect()
                        })
                        .collect();
                    for preferred in 0_u32..(1 << card_count) {
                        let mut cards: Vec<_> =
                            (0..card_count).map(|card| ObjectId::new(card, 0)).collect();
                        cards.sort_by_key(|card| preferred & (1 << card.slot()) == 0);
                        let expected = brute_force_capacity(&menus, preferred, 0);
                        let actual = preferred_assignment(&menus, cards);
                        if let Some(assignment) = &actual {
                            assert_eq!(assignment.len(), menus.len());
                            for (index, card) in assignment.iter().enumerate() {
                                assert!(menus[index].contains(card));
                                assert!(!assignment[..index].contains(card));
                            }
                        }
                        let capacity = actual.map(|assignment| {
                            assignment
                                .into_iter()
                                .filter(|card| preferred & (1 << card.slot()) != 0)
                                .count()
                        });
                        assert_eq!(
                            capacity, expected,
                            "cards={card_count}, costs={cost_count}, edges={edges:#b}, preferred={preferred:#b}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn additional_cost_matching_follows_long_augmenting_paths() {
        let cards: Vec<_> = (0..64).map(|slot| ObjectId::new(slot, 0)).collect();
        // The last candidate can pay only the first cost and displaces all
        // 63 preceding cards along one successful augmenting path.
        let mut menus = vec![vec![cards[0], cards[63]]];
        menus.extend(cards[..63].windows(2).map(<[ObjectId]>::to_vec));
        menus.push(vec![cards[62]]);
        let assignment = preferred_assignment(&menus, cards.iter().copied()).unwrap();
        assert_eq!(assignment.len(), 64);
        for (index, card) in assignment.iter().enumerate() {
            assert!(menus[index].contains(card));
            assert!(!assignment[..index].contains(card));
        }
        // An extra cost cannot reuse the card that already paid the last one.
        menus.push(vec![cards[0]]);
        assert!(preferred_assignment(&menus, cards).is_none());
    }
}
