//! Where each object was last seen, and what has moved since.
//!
//! A view is a photograph of the zones a seat may look into, and it carries no
//! log of what happened between two of them. That is enough, because an object
//! keeps its [`ObjectId`] across a zone change — the engine bumps a version and
//! clears the projection (CR 400.7) but does not renumber the handle — so two
//! consecutive views name the same card in two different lists, and the
//! difference between them *is* the event.
//!
//! This is where the client reads that difference. It exists so that the table
//! can draw a permanent leaving for somewhere in particular: a creature that
//! died and a creature that was exiled both vanish from `view.battlefield`, and
//! nothing about the battlefield alone tells them apart.
//!
//! What it cannot answer, it says so about rather than guessing. A card put on
//! the bottom of a library, or bounced to an opponent's hand, is in a zone this
//! seat sees as a *count*: there is no object to find, so the move is reported
//! with [`None`] and the renderer draws the neutral exit. The alternative —
//! watching a seat's hand count go up in the same view — is a guess that reads
//! exactly like a fact, and one wrong frame of it would show a card flying to a
//! hand it never reached. Answering that properly means the view saying where a
//! card went, which is a change to the engine's side of the wire.

use std::collections::HashMap;

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::PlayerView;

/// A place this seat can see an object in.
///
/// There is no `Library` arm and none for another seat's hand: a view carries
/// those as counts and nothing else, so an object in one of them is not
/// somewhere *else* to a client — it is nowhere it can be found, which is what
/// [`None`] says wherever a `Place` is optional.
///
/// The three zones there is one of per seat carry that seat, and that is not
/// decoration. A graveyard is a *pile standing beside a particular chair*, so
/// a renderer told only "a graveyard" knows every fact about the move except
/// the one it needs: where on the table to send the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// The shared battlefield.
    Battlefield,
    /// The stack.
    Stack,
    /// One seat's graveyard.
    Graveyard(PlayerId),
    /// One seat's public exile.
    Exile(PlayerId),
    /// One seat's command zone.
    Command(PlayerId),
    /// This seat's own hand. Another seat's is a count.
    Hand,
}

/// One object's change of place between two views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    /// The object that moved. The same handle in both views.
    pub object: ObjectId,
    /// Where it was. `None` when the previous view did not show it.
    pub from: Option<Place>,
    /// Where it is now. `None` when this view does not show it.
    pub to: Option<Place>,
}

/// A seat's index in one of the view's per-seat lists, as a [`PlayerId`].
///
/// Saturating rather than panicking. A seat number this client cannot hold is
/// a view it should not have been sent, and drawing a card at the wrong pile is
/// a better answer to that than taking the window down.
fn seat_of(index: usize) -> PlayerId {
    PlayerId::new(u8::try_from(index).unwrap_or(u8::MAX))
}

/// What a view says about where everything is.
///
/// Precedence is the scan order and the first answer wins, so an object listed
/// in two zones is reported in the one nearest the table. Nothing should be
/// listed twice; a fixed order is what stops a duplicate from flipping its
/// answer between frames if something ever is.
///
/// [`PlayerView::looking_at`] is deliberately not scanned. Those objects are in
/// no zone this seat can see — they are being *shown* to it for the length of
/// one question — so counting them as a place would move a card into a zone
/// when a dialog opened and out of it again when the dialog closed.
fn places(view: &PlayerView) -> HashMap<ObjectId, Place> {
    let mut out: HashMap<ObjectId, Place> = HashMap::new();
    for obj in &view.battlefield {
        out.entry(obj.id).or_insert(Place::Battlefield);
    }
    for obj in &view.stack {
        out.entry(obj.id).or_insert(Place::Stack);
    }
    // The index into each of these three is the seat, which is the whole
    // reason the arm carries one: `graveyards[2]` is the pile beside the
    // third chair and nothing else in the view says so.
    for (seat, zone) in view.graveyards.iter().enumerate() {
        for obj in zone {
            out.entry(obj.id).or_insert(Place::Graveyard(seat_of(seat)));
        }
    }
    for (seat, zone) in view.exile.iter().enumerate() {
        for obj in zone {
            out.entry(obj.id).or_insert(Place::Exile(seat_of(seat)));
        }
    }
    for (seat, zone) in view.command.iter().enumerate() {
        for obj in zone {
            out.entry(obj.id).or_insert(Place::Command(seat_of(seat)));
        }
    }
    for obj in &view.hand {
        out.entry(obj.id).or_insert(Place::Hand);
    }
    out
}

/// Remembers where everything was, so the next view can be read as a list of
/// moves.
#[derive(Default)]
pub struct Tracker {
    seen: HashMap<ObjectId, Place>,
    seq: Option<u64>,
}

impl Tracker {
    /// Reads one view and answers with every move that touched the
    /// battlefield, in object order.
    ///
    /// Only moves that leave or reach the battlefield are reported, because a
    /// permanent is the only thing the table draws leaving or arriving: a spell
    /// going from the stack to its owner's graveyard is a scene the table never
    /// had. The order is by object id and not by discovery, because discovery
    /// is a `HashMap` walk and a scene built from one would differ between two
    /// clients watching the same game.
    ///
    /// A view already read answers with nothing. The renderer runs every frame
    /// and the engine does not, so without this the same death would be
    /// reported until the next question was asked.
    pub fn observe(&mut self, view: &PlayerView) -> Vec<Move> {
        if self.seq == Some(view.seq) {
            return Vec::new();
        }
        self.seq = Some(view.seq);
        let now = places(view);
        let mut moves = Vec::new();
        for (&object, &from) in &self.seen {
            let to = now.get(&object).copied();
            if to != Some(from) && (from == Place::Battlefield || to == Some(Place::Battlefield)) {
                moves.push(Move {
                    object,
                    from: Some(from),
                    to,
                });
            }
        }
        for (&object, &to) in &now {
            if to == Place::Battlefield && !self.seen.contains_key(&object) {
                moves.push(Move {
                    object,
                    from: None,
                    to: Some(to),
                });
            }
        }
        moves.sort_by_key(|m| m.object);
        self.seen = now;
        moves
    }

    /// Forgets everything, for a table that is being torn down.
    ///
    /// A tracker carried into the next duel would answer the first view of it
    /// with a move for every object whose handle happened to be reused.
    pub fn clear(&mut self) {
        self.seen.clear();
        self.seq = None;
    }
}

/// Which door a permanent went through, when the door is one the player is
/// owed a picture of.
///
/// The table already sends a departing card somewhere in particular — a pile,
/// a rise towards the hand, a shrink to nothing — and that trajectory is the
/// whole of what a zone change has looked like so far. It is not enough on a
/// busy board: a creature exiled and a creature destroyed both leave the felt
/// and slide to a pile a few centimetres apart, and in a still frame they are
/// the same event. This is the second half of the answer, and it is a
/// *claim about the move* rather than about the card, which is why it lives
/// beside [`Move`] and not on anything drawn.
///
/// Five doors, because those are the five the owner asked for: back to hand,
/// into exile and out of it again, into a graveyard and out of it again.
/// Everything else is [`None`] — a spell resolving from the stack on to the
/// battlefield is the ordinary arrival this client has always drawn, and
/// giving it a door of its own would put a mark on the commonest event in the
/// game, which is the everywhere-at-once the sheen was cut back from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Passage {
    /// Battlefield to this seat's hand: the entrance played backwards.
    Bounce,
    /// Battlefield to exile: a way out with no way back in it.
    Exiled,
    /// Exile to the battlefield: the same door, opening.
    Flickered,
    /// Battlefield to a graveyard.
    Destroyed,
    /// A graveyard to the battlefield.
    Returned,
}

impl Passage {
    /// The door a move went through, if it went through one of the five.
    ///
    /// Deliberately blind to *whose* pile it was. A card exiled to an
    /// opponent's exile zone went through the same door as one exiled to its
    /// owner's, and the seat is already carried by the [`Place`] that decides
    /// where on the table the card is sent — asking it twice would be two
    /// answers that could disagree.
    ///
    /// A move with no `from` or no `to` has no door either: `None` is this
    /// seat being unable to see one end of the move at all (a card bounced to
    /// an opponent's hand, a permanent put on the bottom of a library), and
    /// the neutral exit is the only honest drawing of that.
    #[must_use]
    pub fn of(from: Option<Place>, to: Option<Place>) -> Option<Self> {
        match (from?, to?) {
            (Place::Battlefield, Place::Hand) => Some(Self::Bounce),
            (Place::Battlefield, Place::Exile(_)) => Some(Self::Exiled),
            (Place::Exile(_), Place::Battlefield) => Some(Self::Flickered),
            (Place::Battlefield, Place::Graveyard(_)) => Some(Self::Destroyed),
            (Place::Graveyard(_), Place::Battlefield) => Some(Self::Returned),
            _ => None,
        }
    }

    /// Whether this door is one a card leaves the battlefield through.
    ///
    /// The renderer needs the split because the two halves are drawn in
    /// different places: a departure is baked into a material once, on the
    /// frame the card stops being a card, and an arrival rides the sheen the
    /// board model has already started for it.
    #[must_use]
    pub fn is_departure(self) -> bool {
        matches!(self, Self::Bounce | Self::Exiled | Self::Destroyed)
    }
}

impl Move {
    /// The door this move went through, if it went through one.
    #[must_use]
    pub fn passage(&self) -> Option<Passage> {
        Passage::of(self.from, self.to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, printed};

    fn id(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    fn seat(n: u8) -> PlayerId {
        PlayerId::new(n)
    }

    /// A bear on the battlefield and nothing else, at the given sequence.
    fn standing(seq: u64) -> PlayerView {
        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .build();
        view.seq = seq;
        view
    }

    /// The same bear, in seat 0's graveyard.
    fn buried(seq: u64) -> PlayerView {
        let mut view = ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .build();
        view.seq = seq;
        view
    }

    #[test]
    fn a_permanent_that_dies_moves_to_the_graveyard() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        assert_eq!(
            tracker.observe(&buried(2)),
            vec![Move {
                object: id(1),
                from: Some(Place::Battlefield),
                to: Some(Place::Graveyard(seat(0))),
            }]
        );
    }

    /// Whose graveyard is the half of the answer a renderer draws with. A card
    /// that dies under an opponent's control goes to *their* pile, on the far
    /// side of the table, and a move that said only "a graveyard" would send it
    /// to the near one.
    #[test]
    fn an_opponents_graveyard_is_not_this_seats() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        let mut theirs = ViewBuilder::new(2)
            .with_graveyard(1, vec![printed(1, 0, "Grizzly Bears", 1)])
            .build();
        theirs.seq = 2;
        assert_eq!(
            tracker.observe(&theirs)[0].to,
            Some(Place::Graveyard(seat(1)))
        );
        assert_ne!(Place::Graveyard(seat(1)), Place::Graveyard(seat(0)));
    }

    #[test]
    fn a_permanent_that_is_exiled_moves_to_exile() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        let mut gone = ViewBuilder::new(2)
            .with_exile(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .build();
        gone.seq = 2;
        assert_eq!(
            tracker.observe(&gone),
            vec![Move {
                object: id(1),
                from: Some(Place::Battlefield),
                to: Some(Place::Exile(seat(0))),
            }]
        );
    }

    #[test]
    fn a_permanent_bounced_to_this_seats_hand_moves_to_the_hand() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        let mut back = ViewBuilder::new(2)
            .with_hand(vec![("Grizzly Bears", 2, 1)])
            .build();
        back.seq = 2;
        assert_eq!(
            tracker.observe(&back),
            vec![Move {
                object: id(1),
                from: Some(Place::Battlefield),
                to: Some(Place::Hand),
            }]
        );
    }

    /// The bottom of a library and an opponent's hand look alike from here,
    /// and both of them look like a card that stopped existing. The tracker
    /// says only that it is no longer anywhere it can be found.
    #[test]
    fn a_permanent_that_goes_where_this_seat_cannot_look_moves_to_nowhere() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        let mut empty = ViewBuilder::new(2).build();
        empty.seq = 2;
        assert_eq!(
            tracker.observe(&empty),
            vec![Move {
                object: id(1),
                from: Some(Place::Battlefield),
                to: None,
            }]
        );
    }

    #[test]
    fn a_permanent_returning_from_the_graveyard_says_where_it_came_from() {
        let mut tracker = Tracker::default();
        tracker.observe(&buried(1));
        assert_eq!(
            tracker.observe(&standing(2)),
            vec![Move {
                object: id(1),
                from: Some(Place::Graveyard(seat(0))),
                to: Some(Place::Battlefield),
            }]
        );
    }

    #[test]
    fn a_permanent_flickering_back_from_exile_says_so() {
        let mut tracker = Tracker::default();
        let mut gone = ViewBuilder::new(2)
            .with_exile(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .build();
        gone.seq = 1;
        tracker.observe(&gone);
        assert_eq!(
            tracker.observe(&standing(2)),
            vec![Move {
                object: id(1),
                from: Some(Place::Exile(seat(0))),
                to: Some(Place::Battlefield),
            }]
        );
    }

    /// The renderer runs every frame and the engine does not.
    #[test]
    fn the_same_view_twice_is_not_a_move() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        assert_eq!(tracker.observe(&buried(2)).len(), 1);
        assert!(tracker.observe(&buried(2)).is_empty());
    }

    /// A spell going from the stack to a graveyard is a scene the table never
    /// had, so it is not reported at all.
    #[test]
    fn a_card_that_never_touches_the_battlefield_is_not_reported() {
        let mut tracker = Tracker::default();
        let mut cast = ViewBuilder::new(2)
            .with_stack(vec![printed(2, 0, "Lightning Bolt", 2)])
            .build();
        cast.seq = 1;
        tracker.observe(&cast);
        let mut spent = ViewBuilder::new(2)
            .with_graveyard(0, vec![printed(2, 0, "Lightning Bolt", 2)])
            .build();
        spent.seq = 2;
        assert!(tracker.observe(&spent).is_empty());
    }

    /// A flicker is two moves and not one, and the second must not be lost by
    /// the first having already forgotten the card.
    #[test]
    fn a_permanent_that_leaves_and_comes_back_is_two_moves() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        let mut gone = ViewBuilder::new(2)
            .with_exile(0, vec![printed(1, 0, "Grizzly Bears", 1)])
            .build();
        gone.seq = 2;
        assert_eq!(tracker.observe(&gone)[0].to, Some(Place::Exile(seat(0))));
        let back = tracker.observe(&standing(3));
        assert_eq!(back[0].from, Some(Place::Exile(seat(0))));
        assert_eq!(back[0].to, Some(Place::Battlefield));
    }

    /// Which is the same thing the renderer does for a card it has no history
    /// for: today's entrance, not a resurrection.
    #[test]
    fn the_first_view_of_a_board_reports_every_permanent_as_coming_from_nowhere() {
        let mut tracker = Tracker::default();
        let mut view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![
                    printed(1, 0, "Grizzly Bears", 1),
                    printed(2, 0, "Island", 2),
                ],
            )
            .build();
        view.seq = 9;
        let moves = tracker.observe(&view);
        assert_eq!(moves.len(), 2);
        assert!(moves.iter().all(|m| m.from.is_none()));
        assert_eq!(moves[0].object, id(1));
        assert_eq!(moves[1].object, id(2));
    }

    /// A tracker carried into the next duel would answer its first view with a
    /// move for every handle that happened to be reused.
    #[test]
    fn a_cleared_tracker_has_no_memory_of_the_last_table() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        tracker.clear();
        let moves = tracker.observe(&buried(2));
        assert!(moves.is_empty(), "{moves:?}");
    }

    /// The five doors the owner asked for, each read off the move that goes
    /// through it. Written out one by one rather than swept, because the
    /// point of the table is that these five are *different* and a loop over
    /// a list of pairs would be the same statement said once.
    #[test]
    fn each_of_the_five_doors_is_read_off_the_move_that_uses_it() {
        let bf = Some(Place::Battlefield);
        assert_eq!(Passage::of(bf, Some(Place::Hand)), Some(Passage::Bounce));
        assert_eq!(
            Passage::of(bf, Some(Place::Exile(seat(0)))),
            Some(Passage::Exiled)
        );
        assert_eq!(
            Passage::of(Some(Place::Exile(seat(1))), bf),
            Some(Passage::Flickered)
        );
        assert_eq!(
            Passage::of(bf, Some(Place::Graveyard(seat(0)))),
            Some(Passage::Destroyed)
        );
        assert_eq!(
            Passage::of(Some(Place::Graveyard(seat(1))), bf),
            Some(Passage::Returned)
        );
    }

    /// The counter-test, and the one that matters most: a table that answered
    /// *something* for every move would put a door on the commonest event in
    /// the game — a spell resolving from the stack — which is the
    /// everywhere-at-once the sheen was cut back from in the first place.
    #[test]
    fn an_ordinary_arrival_goes_through_no_door_at_all() {
        let bf = Some(Place::Battlefield);
        assert_eq!(Passage::of(Some(Place::Stack), bf), None);
        assert_eq!(Passage::of(bf, Some(Place::Stack)), None);
        assert_eq!(Passage::of(Some(Place::Hand), bf), None);
        assert_eq!(Passage::of(bf, Some(Place::Command(seat(0)))), None);
        // Battlefield to battlefield is not a move at all; the tracker never
        // reports one, and this is the belt to that brace.
        assert_eq!(Passage::of(bf, bf), None);
    }

    /// Half a move is not a door. This seat cannot see an opponent's hand or
    /// any library, so a card sent to one of them is reported with `None` —
    /// and a client that guessed at those would draw a card flying to a hand
    /// it never reached.
    #[test]
    fn a_move_this_seat_cannot_see_both_ends_of_has_no_door() {
        assert_eq!(Passage::of(Some(Place::Battlefield), None), None);
        assert_eq!(Passage::of(None, Some(Place::Battlefield)), None);
        assert_eq!(Passage::of(None, None), None);
    }

    /// Whose pile it was is the [`Place`]'s business, not the door's: a
    /// creature exiled to an opponent's exile zone went out through the same
    /// door as one exiled to its owner's.
    #[test]
    fn a_door_does_not_care_whose_pile_is_on_the_other_side_of_it() {
        let bf = Some(Place::Battlefield);
        for who in 0..4u8 {
            assert_eq!(
                Passage::of(bf, Some(Place::Graveyard(seat(who)))),
                Some(Passage::Destroyed)
            );
        }
    }

    /// The split the renderer needs, and it is not cosmetic: a departure is
    /// baked into a material on the frame the card stops being a card, and an
    /// arrival rides a sheen the board model has already started.
    #[test]
    fn the_three_doors_out_are_departures_and_the_two_back_in_are_not() {
        assert!(Passage::Bounce.is_departure());
        assert!(Passage::Exiled.is_departure());
        assert!(Passage::Destroyed.is_departure());
        assert!(!Passage::Flickered.is_departure());
        assert!(!Passage::Returned.is_departure());
    }

    /// Read through a real pair of views rather than by hand, because
    /// `Passage::of` is only worth anything if the moves the tracker actually
    /// produces reach it with both ends filled in.
    #[test]
    fn a_creature_that_dies_between_two_views_is_read_as_destroyed() {
        let mut tracker = Tracker::default();
        tracker.observe(&standing(1));
        let moves = tracker.observe(&buried(2));
        let died = moves
            .iter()
            .find(|m| m.object == id(1))
            .expect("the bear left the battlefield");
        assert_eq!(died.passage(), Some(Passage::Destroyed));
    }
}
