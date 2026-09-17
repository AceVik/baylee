//! Choosing a neighbour expresses the left/right choice without a second
//! direction encoding on the wire: every player receives that neighbour's
//! nonland permanents, relative to their own seat (Aminatou's printed −6).

use super::{AwaitingOp, Flow, Pending, PlayerId, Resolution, change_controller, run};
use crate::state::GameState;
use crate::zone::ZoneLocation;
use baylee_core::types::TypeSet;

pub(super) fn ask(state: &mut GameState, res: &mut Resolution) -> Option<Pending> {
    let seats: Vec<_> = state
        .players
        .iter()
        .filter(|p| !p.has_lost)
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
    let changes: Vec<_> = state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter_map(|&id| {
            let object = state.object(id)?;
            if id == res.source || object.characteristics().types.contains(TypeSet::LAND) {
                return None;
            }
            let from = seats.iter().position(|&p| p == object.controller)?;
            Some((id, seats[(from + seats.len() - distance) % seats.len()]))
        })
        .collect();
    for (id, controller) in changes {
        change_controller(state, id, controller);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                x: None,
                chosen_player: None,
                target_players: SeatSet::default(),
                event_object: None,
                awaiting: None,
                targeted: false,
                mana_ability: false,
                countered_source: None,
                target_lki: None,
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
