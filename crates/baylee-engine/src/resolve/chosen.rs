//! One player at a time in APNAP order, picking one of their own permanents,
//! then acting on every choice together (CR 101.4, CR 603.10a).
//!
//! Three effects share a shape rather than a meaning: the effect names a
//! filter and a set of players, and every one of those players is asked in
//! APNAP order to name one of their own permanents matching it. Each choice
//! is held on the battlefield until the last player has answered, and then
//! every chosen permanent moves at once — [`Effect::SacrificeFilter`]
//! sacrifices them together, [`Effect::DestroyChosenForPlayers`] destroys them
//! together, and [`Effect::ReturnChosenToHand`] puts them into their owners'
//! hands together — so leaves-the-battlefield and dies abilities see the full
//! simultaneous batch (CR 603.10a).
//!
//! What a given player may pick ([`options`]), the APNAP ordering
//! ([`order_apnap`]), who is asked next ([`next_asked`] — skipping any player
//! with no legal pick rather than showing an empty list), and the simultaneous
//! batch execution ([`record_choice`], [`sacrifice_held`], [`destroy_held`],
//! [`return_held`]) all live here.
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
use baylee_core::ids::{DamageSourceRef, ObjectId, PlayerId};

use crate::eval;
use crate::event::Cause;
use crate::object::{ObjectKind, Status};
use crate::state::GameState;
use crate::zone::{Zone, ZoneLocation, ZonePosition};

/// An opponent chooses a permanent, but the named player sacrifices it.
pub(super) fn opponent_sacrifice(
    state: &GameState,
    res: &mut super::Resolution,
    player: PlayerId,
    filter: &'static Filter,
) -> Option<crate::choice::Pending> {
    let options = options(state, player, filter, res.controller, res.source);
    if options.is_empty() {
        return None;
    }
    let opponents = eval::players(baylee_cards_dsl::PlayerRel::Opponent, state, player)?;
    match opponents.as_slice() {
        [] => None,
        [only] => Some(ask_sacrifice(res, player, *only, options)),
        _ => {
            res.awaiting = Some(super::AwaitingOp::SacrificeOpponent { player, options });
            Some(crate::choice::Pending::ChoosePlayer {
                player,
                options: opponents,
            })
        }
    }
}

/// The opponent selects among the sacrificing player's untargeted options.
pub(super) fn ask_sacrifice(
    res: &mut super::Resolution,
    player: PlayerId,
    chooser: PlayerId,
    options: Vec<ObjectId>,
) -> crate::choice::Pending {
    res.awaiting = Some(super::AwaitingOp::SacrificeChosen { player });
    crate::choice::Pending::ChooseCards {
        player: chooser,
        options,
        min: 1,
        max: 1,
        prompt: crate::choice::ChoicePrompt::SacrificeFor { player },
        total: None,
    }
}

/// The permanents `player` controls that `filter` matches
/// ([`eval::controlled_matching`], which `Condition::CanSacrifice` asks too,
/// so "if you can't" and the question agree).
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
    eval::controlled_matching(state, player, filter, you, source)
}

/// Orders `players` in APNAP order starting at the active player (CR 101.4),
/// dropping any player who has already left the game.
pub(super) fn order_apnap(state: &GameState, players: &mut Vec<PlayerId>) {
    players.retain(|&player| !state.has_left(player));
    let seats = state.players.len();
    if seats > 1 {
        let active = usize::from(state.turn.active.get()) % seats;
        players.sort_by_key(|player| (usize::from(player.get()) + seats - active) % seats);
    }
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
        if state.has_left(player) {
            continue;
        }
        let options = options(state, player, filter, you, source);
        if !options.is_empty() {
            return Some((player, options));
        }
    }
    None
}

/// Records `asked`'s battlefield pick so every player's choice happens
/// together once the last player answers (CR 101.4, CR 603.10a).
pub(super) fn record_choice(
    state: &GameState,
    asked: PlayerId,
    chosen: &[ObjectId],
    held: &mut Vec<(PlayerId, DamageSourceRef)>,
) {
    let Some(&id) = chosen.first() else {
        return;
    };
    let Some(obj) = state.object(id).filter(|o| {
        o.zone == Zone::Battlefield
            && o.controller == asked
            && !o.status.contains(Status::PHASED_OUT)
    }) else {
        return;
    };
    held.push((
        asked,
        DamageSourceRef {
            object: id,
            version: obj.version,
        },
    ));
}

/// Sacrifices every held pick simultaneously (CR 101.4, CR 603.10a).
///
/// Each permanent is sacrificed by the player who chose it and only while it
/// remains on the battlefield under their control (CR 701.21a).
pub(super) fn sacrifice_held(state: &mut GameState, held: &[(PlayerId, DamageSourceRef)]) {
    let victims: Vec<ObjectId> = held
        .iter()
        .filter(|(player, pick)| {
            !state.has_left(*player)
                && state.object(pick.object).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && o.version == pick.version
                        && o.controller == *player
                        && !o.status.contains(Status::PHASED_OUT)
                })
        })
        .map(|(_, pick)| pick.object)
        .collect();
    super::sacrifice_together(state, &victims);
}

/// Destroys every held pick simultaneously (CR 101.4, CR 603.10a), retaining
/// departure snapshots and regeneration checks across the batch.
pub(super) fn destroy_held(state: &mut GameState, held: &[(PlayerId, DamageSourceRef)]) {
    let victims: Vec<ObjectId> = held
        .iter()
        .filter(|(_, pick)| {
            state.object(pick.object).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && o.version == pick.version
                    && !o.status.contains(Status::PHASED_OUT)
            })
        })
        .map(|(_, pick)| pick.object)
        .collect();
    crate::sba::destroy_all(state, &victims, false);
}

/// Returns every held pick to its owner's hand simultaneously (CR 101.4,
/// CR 400.3, CR 603.10a), capturing every departure before any permanent
/// leaves the battlefield.
///
/// No `ask_commander_replace` here, and this sibling is the only one of the
/// three that would ever want it: CR 903.9b is a replacement for a hand or a
/// library, while a commander reaching a graveyard is CR 903.9a, a
/// state-based action. `resume_yes_no` re-enters its operation at the same
/// program counter (`pc` unchanged), which would restart the whole per-player
/// chain; listed in `docs/engine-internals.md` beside the other paths the
/// rule does not reach.
pub(super) fn return_held(state: &mut GameState, held: &[(PlayerId, DamageSourceRef)]) {
    let batch: Vec<_> = held
        .iter()
        .filter_map(|(player, pick)| {
            if state.has_left(*player) {
                return None;
            }
            let obj = state.object(pick.object).filter(|o| {
                o.zone == Zone::Battlefield
                    && o.version == pick.version
                    && o.controller == *player
                    && !o.status.contains(Status::PHASED_OUT)
                    && !state.has_left(o.owner)
            })?;
            Some((
                pick.object,
                obj.owner,
                state.departure_snapshot(pick.object),
            ))
        })
        .collect();
    for (id, owner, departure) in batch {
        if let Some(obj) = state.object_mut(id) {
            obj.kind = ObjectKind::Card;
        }
        let _ = state.move_object_with_departure(
            id,
            ZoneLocation::Hand(owner),
            ZonePosition::Top,
            Cause::Effect,
            departure,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{next_asked, options, order_apnap};
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

    /// A phased-out permanent is treated as though it does not exist
    /// (CR 702.26b): nobody is asked to sacrifice it.
    #[test]
    fn a_phased_out_permanent_is_not_offered() {
        let mut state = state(2);
        let seen = permanent(&mut state, seat(0), "seen");
        let phased = permanent(&mut state, seat(0), "phased out");
        state
            .object_mut(phased)
            .expect("seated")
            .status
            .insert(crate::object::Status::PHASED_OUT);
        assert_eq!(
            options(&state, seat(0), &Filter::ControlledByYou, seat(0), seen),
            vec![seen]
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

    #[test]
    fn order_apnap_rotates_from_active_player_and_skips_departed_seats() {
        let mut state = state(4);
        state.turn.active = seat(2);
        let mut players = vec![seat(0), seat(1), seat(2), seat(3)];
        order_apnap(&state, &mut players);
        assert_eq!(players, vec![seat(2), seat(3), seat(0), seat(1)]);

        crate::sba::eliminate_player(&mut state, seat(3), crate::event::LossReason::Conceded);
        let mut players = vec![seat(0), seat(1), seat(2), seat(3)];
        order_apnap(&state, &mut players);
        assert_eq!(players, vec![seat(2), seat(0), seat(1)]);
    }
}
