//! The stall guard of a search's roots and of the search student, as the
//! house's (`baylee_ai::StallMemory`): an answer the table took back from a
//! position is not given there again.

use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;

/// The answers given to one seat's priority questions from one position,
/// and the last question's number. Asked again from that position by a new
/// question, the engine took them back (a cast it could not finish leaves
/// the game where it was). The same question asked again is not a taking
/// back. A pass is never taken back. The position is
/// `baylee_ai::HeuristicAgent::position` of the seat's view.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StallMemory4 {
    given: Option<Given>,
}

/// One position's answers ([`StallMemory4`]).
#[derive(Clone, Debug, PartialEq)]
struct Given {
    seat: PlayerId,
    position: u64,
    seq: u64,
    actions: Vec<PlayerAction>,
}

impl StallMemory4 {
    /// The answers the table took back from `position` before `seat`'s
    /// question `seq`; none when the last answer was given elsewhere, or to
    /// this same question.
    #[must_use]
    pub fn taken_back(&self, seat: PlayerId, position: u64, seq: u64) -> &[PlayerAction] {
        match &self.given {
            Some(given) if given.seat == seat && given.position == position && given.seq != seq => {
                &given.actions
            }
            _ => &[],
        }
    }

    /// Notes `action`, the answer to `seat`'s question `seq` from
    /// `position`: beside the answers already given there, or in their
    /// place when the position is another.
    pub fn give(&mut self, seat: PlayerId, position: u64, seq: u64, action: &PlayerAction) {
        if matches!(action, PlayerAction::PassPriority) {
            return;
        }
        match &mut self.given {
            Some(given) if given.seat == seat && given.position == position => {
                given.seq = seq;
                if !given.actions.contains(action) {
                    given.actions.push(action.clone());
                }
            }
            _ => {
                self.given = Some(Given {
                    seat,
                    position,
                    seq,
                    actions: vec![action.clone()],
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asked again from the same position by a new question, the answers
    /// were taken back; the same question again, another position or
    /// another seat is not that.
    #[test]
    fn the_stall_memory_names_what_the_table_took_back_from_a_position() {
        let me = PlayerId::new(0);
        let other = PlayerId::new(1);
        let cast = PlayerAction::CastSpell {
            card: baylee_core::ids::ObjectId::new(7, 0),
        };
        let land = PlayerAction::PlayLand {
            card: baylee_core::ids::ObjectId::new(9, 0),
        };
        let mut memory = StallMemory4::default();
        assert!(memory.taken_back(me, 1, 10).is_empty(), "nothing given yet");
        memory.give(me, 1, 10, &cast);
        assert!(
            memory.taken_back(me, 1, 10).is_empty(),
            "the same question asked again"
        );
        assert_eq!(memory.taken_back(me, 1, 11), std::slice::from_ref(&cast));
        assert!(memory.taken_back(me, 2, 11).is_empty(), "another position");
        assert!(memory.taken_back(other, 1, 11).is_empty(), "another seat");

        // Taken back again, the other answer joins it.
        memory.give(me, 1, 11, &land);
        assert_eq!(memory.taken_back(me, 1, 12), [cast.clone(), land.clone()]);
        memory.give(me, 1, 12, &PlayerAction::PassPriority);
        assert_eq!(
            memory.taken_back(me, 1, 13),
            [cast.clone(), land],
            "a pass is never taken back"
        );

        // The game moved on: a new position starts afresh.
        memory.give(me, 3, 14, &cast);
        assert!(memory.taken_back(me, 1, 15).is_empty());
        assert_eq!(memory.taken_back(me, 3, 15), [cast]);
    }
}
