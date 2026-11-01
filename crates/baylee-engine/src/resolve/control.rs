//! Choosing a neighbour expresses the left/right choice without a second
//! direction encoding on the wire: every player receives that neighbour's
//! nonland permanents, relative to their own seat (Aminatou's printed −6).

use super::{AwaitingOp, Flow, Pending, PlayerId, Resolution, gain_control, run};
use crate::state::GameState;
use baylee_core::types::TypeSet;

/// An opponent is chosen on resolution, independently of targetability.
pub(super) fn change(
    state: &mut GameState,
    res: &mut Resolution,
    relation: baylee_cards_dsl::PlayerRel,
) -> Option<Pending> {
    let subject =
        super::subjects::this(state, res).filter(|r| super::subjects::on_battlefield(state, *r))?;
    let options = super::players_of(relation, state, res.controller, res);
    match options.as_slice() {
        [] => None,
        [only] => {
            gain_control(state, &[(subject.object, *only)]);
            None
        }
        _ => {
            res.awaiting = Some(AwaitingOp::ControlRecipient { subject });
            Some(Pending::ChoosePlayer {
                player: res.controller,
                options,
            })
        }
    }
}

/// Schedule control of each selected player during their next actual turn.
pub(super) fn next_turn(
    state: &mut GameState,
    res: &Resolution,
    relation: baylee_cards_dsl::PlayerRel,
) {
    let turn = state.turn.number.saturating_add(1);
    for player in super::players_of(relation, state, res.controller, res) {
        state.turn_controls.push(crate::state::PlayerTurnControl {
            controller: res.controller,
            player,
            turn,
            began: None,
        });
    }
}

/// Resume a selected control transfer or a table rotation.
///
/// # Panics
/// If no control choice is suspended.
#[must_use]
pub fn resume_control_choice(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
) -> Flow {
    if let Some(AwaitingOp::ControlRecipient { subject }) = res.awaiting {
        res.awaiting = None;
        if !state.has_left(player) && super::subjects::on_battlefield(state, subject) {
            gain_control(state, &[(subject.object, player)]);
        }
        res.pc += 1;
        return run(state, res);
    }
    resume_control_rotation(state, res, player)
}

pub(super) fn ask(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let seats: Vec<_> = state
        .players
        .iter()
        .filter(|p| !p.has_lost())
        .map(|p| p.id)
        .collect();
    let at = seats.iter().position(|&p| p == res.controller)?;
    if seats.len() <= 1 {
        return None;
    }
    if seats.len() == 2 {
        rotate(state, res, &seats, 1);
        return None;
    }
    let options = vec![
        seats[(at + 1) % seats.len()],
        seats[(at + seats.len() - 1) % seats.len()],
    ];
    res.awaiting = Some(AwaitingOp::ControlRotation { seats });
    Some(Pending::ChoosePlayer {
        player: res.controller,
        options,
    })
}

/// Resumes a rotation by naming the neighbour whose permanents the chooser
/// receives. `Engine::apply` has already checked the offered neighbours.
///
/// # Panics
/// If no control rotation is suspended or the choice is not a neighbour.
#[must_use]
pub fn resume_control_rotation(
    state: &mut GameState,
    res: &mut Resolution,
    neighbour: PlayerId,
) -> Flow {
    let Some(AwaitingOp::ControlRotation { seats }) = res.awaiting.take() else {
        panic!("rotation not suspended");
    };
    let at = seats
        .iter()
        .position(|&p| p == res.controller)
        .expect("chooser is seated");
    let distance = if seats[(at + 1) % seats.len()] == neighbour {
        1
    } else {
        assert_eq!(seats[(at + seats.len() - 1) % seats.len()], neighbour);
        seats.len() - 1
    };
    rotate(state, res, &seats, distance);
    res.pc += 1;
    run(state, res)
}

fn rotate(state: &mut GameState, res: &Resolution, seats: &[PlayerId], distance: usize) {
    // Read every old controller before changing any: a rotation is
    // simultaneous, and a seat must never give away something it just got.
    // Not a phased-out permanent: a change of control from a resolution
    // leaves one out (CR 702.26e).
    let changes: Vec<_> = state
        .battlefield_seen()
        .filter_map(|id| {
            let object = state.object(id)?;
            if id == res.source || object.characteristics().types.contains(TypeSet::LAND) {
                return None;
            }
            let from = seats.iter().position(|&p| p == object.controller)?;
            Some((id, seats[(from + seats.len() - distance) % seats.len()]))
        })
        .collect();
    gain_control(state, &changes);
}

/// Preflight both transfers against the same board, then perform one exchange.
pub(super) fn exchange(
    state: &mut GameState,
    a: baylee_core::ids::ObjectId,
    b: baylee_core::ids::ObjectId,
) -> bool {
    let controllers = state
        .object(a)
        .zip(state.object(b))
        .filter(|(a, b)| {
            a.zone == crate::zone::Zone::Battlefield && b.zone == crate::zone::Zone::Battlefield
        })
        .map(|(a, b)| (a.controller, b.controller));
    let Some((a_controller, b_controller)) = controllers else {
        return false;
    };
    if a_controller == b_controller
        || !crate::permanent_guards::can_gain_control(state, a, b_controller)
        || !crate::permanent_guards::can_gain_control(state, b, a_controller)
    {
        return false;
    }
    gain_control(state, &[(a, b_controller), (b, a_controller)]);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone::ZoneLocation;
    use baylee_core::ids::{CardIndex, SeatSet};
    struct Lookup;
    impl crate::state::CardLookup for Lookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    #[test]
    fn rotation_asks_both_neighbours_and_moves_every_nonland_once() {
        let creature = baylee_cards::decks::by_name("Ondu Cleric").unwrap();
        let mut preset = baylee_cards::decks::probe_preset(11, creature).unwrap();
        while preset.seats.len() < 4 {
            preset.seats.push(preset.seats[1].clone());
        }
        for neighbour in [1, 3] {
            let mut state = GameState::from_preset(&preset, &Lookup).unwrap();
            let original: Vec<_> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .map(|&id| {
                    let o = state.object(id).unwrap();
                    (
                        id,
                        o.controller,
                        o.owner,
                        o.characteristics().types.contains(TypeSet::LAND),
                    )
                })
                .collect();
            let source = original
                .iter()
                .find(|(_, p, _, land)| p.get() == 0 && !land)
                .unwrap()
                .0;
            let mut res = Resolution {
                source,
                on_stack: source,
                controller: PlayerId::new(0),
                effects: vec![baylee_cards_dsl::Effect::ControlRotation],
                pc: 0,
                targets: smallvec::SmallVec::new(),
                second_targets: smallvec::SmallVec::new(),
                x: None,
                chosen_player: None,
                target_players: SeatSet::default(),
                event_object: None,
                awaiting: None,
                targeted: false,
                mana_ability: false,
                countered_source: None,
                target_lki: None,
                subject: crate::resolve::SubjectContext::default(),
                text: crate::text_changes::TextChangeMap::IDENTITY,
                event_mana: None,
                retarget_left: None,
                loot: None,
                draw_watch: None,
                this_way_from: None,
                coin_stake: None,
            };
            let Flow::Wait(Pending::ChoosePlayer { options, .. }) = run(&mut state, &mut res)
            else {
                panic!("a multiplayer rotation asks its direction");
            };
            assert_eq!(options, vec![PlayerId::new(1), PlayerId::new(3)]);
            assert!(matches!(
                resume_control_rotation(&mut state, &mut res, PlayerId::new(neighbour)),
                Flow::Complete
            ));
            for (id, old, owner, land) in original {
                let o = state.object(id).unwrap();
                let expected = if id == source || land {
                    old
                } else {
                    PlayerId::new((old.get() + 4 - neighbour) % 4)
                };
                assert_eq!(o.controller, expected);
                assert_eq!(o.owner, owner);
            }
        }
    }
}
