//! One player at a time, picking one of their own permanents.
//!
//! Three effects share a shape rather than a meaning: the effect names a
//! filter and a set of players, and every one of those players is asked, in
//! turn, to name one of their own permanents matching it. Only what happens
//! to the pick differs — [`Effect::SacrificeFilter`] sends it to the
//! graveyard, [`Effect::DestroyChosenForPlayers`] destroys it, and
//! [`Effect::ReturnChosenToHand`] puts it in its owner's hand.
//!
//! Two questions are the same in all of them and live here. What a given
//! player may pick ([`options`]), and who is asked next ([`next_asked`]) —
//! which is not simply "the first player left", because a player with no
//! legal pick is skipped rather than shown an empty list.
//!
//! What deliberately does **not** live here is `min`. It reads as a detail
//! and is the sentence: "sacrifices a creature" is `min: 1` and the player
//! must give something up, while "destroy **up to one**" is `min: 0` and
//! they may decline. A helper that averaged the two would silently make one
//! of those cards wrong, so each call site states its own.
//!
//! [`Effect::SacrificeFilter`]: baylee_cards_dsl::Effect::SacrificeFilter
//! [`Effect::DestroyChosenForPlayers`]: baylee_cards_dsl::Effect::DestroyChosenForPlayers
//! [`Effect::ReturnChosenToHand`]: baylee_cards_dsl::Effect::ReturnChosenToHand

use baylee_cards_dsl::Filter;
use baylee_core::ids::{ObjectId, PlayerId};

use crate::eval;
use crate::state::GameState;
use crate::zone::ZoneLocation;

/// The permanents `player` controls that `filter` matches.
///
/// `you` and `source` are the effect's own controller and source, which is
/// what a filter reads "you" and "this" as — not the player being asked.
pub(super) fn options(
    state: &GameState,
    player: PlayerId,
    filter: &'static Filter,
    you: PlayerId,
    source: ObjectId,
) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                o.controller == player && eval::matches(filter, state, o, you, source)
            })
        })
        .copied()
        .collect()
}

/// Takes players off the front of `remaining` until one has a legal pick,
/// and hands back that player with their options.
///
/// `None` means nobody left can pick, which ends the chain. The skipping is
/// the point: a player with nothing matching is not asked an unanswerable
/// question, and `Pending::ChooseCards` with an empty list and `min: 1`
/// would be exactly that.
pub(super) fn next_asked(
    state: &GameState,
    remaining: &mut Vec<PlayerId>,
    filter: &'static Filter,
    you: PlayerId,
    source: ObjectId,
) -> Option<(PlayerId, Vec<ObjectId>)> {
    while !remaining.is_empty() {
        let player = remaining.remove(0);
        let options = options(state, player, filter, you, source);
        if !options.is_empty() {
            return Some((player, options));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{next_asked, options};
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::object::ObjectKind;
    use crate::state::GameState;
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::Filter;
    use baylee_core::ids::{ObjectId, PlayerId};

    fn seat(n: u8) -> PlayerId {
        PlayerId::new(n)
    }

    fn state(seats: usize) -> GameState {
        let mut preset = preset(11, &[]);
        while preset.seats.len() < seats {
            preset.seats.push(preset.seats[0].clone());
        }
        GameState::from_preset(&preset, &SyntheticLookup::new(vec![])).expect("a game")
    }

    /// A permanent on the battlefield under `controller`, and nothing else:
    /// what these two functions read is the battlefield list, the controller
    /// and the filter.
    fn permanent(state: &mut GameState, controller: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    }

    /// **"You" is the effect's controller and never the player being
    /// asked.**
    ///
    /// The two are the same on the seat that cast the spell and different on
    /// every other one, which is what makes this worth an assertion: "each
    /// player sacrifices a permanent you control" is a sentence that reads
    /// wrongly in the most natural way to write it. A reader that passed the
    /// asked player as `you` would offer each opponent their own permanents
    /// under a filter that says mine.
    ///
    /// `Filter::This` is the same mistake in the other field: it is the
    /// effect's source, so it names one object at one seat however many
    /// players are asked.
    #[test]
    fn a_filter_reads_you_and_this_from_the_effect_and_not_from_the_player_asked() {
        let mut state = state(2);
        let mine = permanent(&mut state, seat(0), "mine");
        let theirs = permanent(&mut state, seat(1), "theirs");

        assert_eq!(
            options(&state, seat(0), &Filter::ControlledByYou, seat(0), mine),
            vec![mine]
        );
        assert_eq!(
            options(&state, seat(1), &Filter::ControlledByYou, seat(0), mine),
            Vec::<ObjectId>::new(),
            "seat 1 controls nothing that seat 0 controls"
        );
        assert_eq!(
            options(&state, seat(1), &Filter::ControlledByYou, seat(1), theirs),
            vec![theirs],
            "and the same call for an effect seat 1 controls finds theirs"
        );

        assert_eq!(
            options(&state, seat(0), &Filter::This, seat(0), theirs),
            Vec::<ObjectId>::new(),
            "`This` is the source, so seat 0 is offered nothing by an effect \
             whose source seat 1 controls"
        );
        assert_eq!(
            options(&state, seat(1), &Filter::This, seat(0), theirs),
            vec![theirs],
            "and it names that one object whichever seat is being asked"
        );
    }

    /// **A player with nothing to pick is skipped, not shown an empty
    /// list.**
    ///
    /// `Pending::ChooseCards` with no options and `min: 1` is an
    /// unanswerable question — the table stops on a player who cannot say
    /// anything. So the queue is walked from the front until somebody has a
    /// legal pick, and what is consumed is every player passed over *and*
    /// the one handed back, so the next call carries on from behind them.
    ///
    /// `None` is how the chain ends, and it ends with the queue empty rather
    /// than with a player still in it.
    #[test]
    fn a_player_with_no_legal_pick_is_passed_over_and_not_asked() {
        let mut state = state(4);
        let third = permanent(&mut state, seat(2), "only one on the board");

        let mut remaining = vec![seat(0), seat(1), seat(2), seat(3)];
        assert_eq!(
            next_asked(
                &state,
                &mut remaining,
                &Filter::ControlledByYou,
                seat(2),
                third
            ),
            Some((seat(2), vec![third])),
            "the first two have nothing and are passed over"
        );
        assert_eq!(
            remaining,
            vec![seat(3)],
            "and the one who answered is off the queue too"
        );

        assert_eq!(
            next_asked(
                &state,
                &mut remaining,
                &Filter::ControlledByYou,
                seat(2),
                third
            ),
            None,
            "nobody left can pick, which ends the chain"
        );
        assert!(remaining.is_empty(), "and the queue is spent, not stuck");

        assert_eq!(
            next_asked(
                &state,
                &mut Vec::new(),
                &Filter::ControlledByYou,
                seat(2),
                third
            ),
            None,
            "an empty queue is the same answer and not a panic"
        );
    }

    /// The order the queue is asked in is the order it was given in, which
    /// is turn order at every call site — taken from the **front**, so a
    /// reader that popped from the back would ask a table backwards and be
    /// invisible in a duel.
    #[test]
    fn the_queue_is_asked_from_the_front() {
        let mut state = state(3);
        let first = permanent(&mut state, seat(0), "first");
        let second = permanent(&mut state, seat(1), "second");

        for (queue, expected) in [
            (vec![seat(0), seat(1)], (seat(0), first)),
            (vec![seat(1), seat(0)], (seat(1), second)),
        ] {
            let mut remaining = queue;
            let asked = next_asked(&state, &mut remaining, &Filter::Any, seat(0), first);
            assert_eq!(asked, Some((expected.0, vec![expected.1])));
        }
    }
}
