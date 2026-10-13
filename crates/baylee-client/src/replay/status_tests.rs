//! What a replay's status says of the seats it can be drawn from, and
//! whether the hand shows (`.claude/b8/REPLAY-DESIGN.md` §18.3): the picker's
//! choices and their order, the cycle `V` walks, which perspective hides its
//! hand, what each is called, and the bar's fold-away hand over them.
//!
//! The statuses are built whole, field by field: they are what the host
//! writes into the duel, and the questions asked of them are pure.

use baylee_client_core::Lang;
use baylee_client_core::replay::{
    Header, Perspective, Perspectives, Source, Speed, TAPE_VERSION, Verify, tape_perspective,
};
use baylee_core::ids::PlayerId;

use super::{Origin, ReplayStatus, SeatName, top_line};
use crate::hud::replay_bar::BarUi;

const EVERYTHING: Perspectives = Perspectives {
    seat: true,
    public: true,
    all_hands: true,
};

fn seat(n: u8) -> PlayerId {
    PlayerId::new(n)
}

fn names(count: u8) -> Vec<SeatName> {
    ["Alice#af03", "Bob#0012", "Cy#0003", "Dee#0004"]
        .iter()
        .take(usize::from(count))
        .enumerate()
        .map(|(i, name)| SeatName {
            seat: seat(i as u8),
            name: (*name).to_string(),
        })
        .collect()
}

/// A status of a hosted game of `seats` seats, viewed by `viewer`, showing
/// `perspective`, with no tape header yet.
fn status(viewer: u8, seats: u8, perspective: Perspective, offers: Perspectives) -> ReplayStatus {
    ReplayStatus {
        header: None,
        step: 0,
        steps: 0,
        playing: false,
        speed: Speed::One,
        timeline: None,
        stop: None,
        check: None,
        perspective,
        offers,
        verify: Verify::Stop,
        loading: None,
        problem: None,
        origin: Origin {
            place: "gw".to_string(),
            viewer: seat(viewer),
            ..Origin::default()
        },
        seats: names(seats),
    }
}

/// The header of a tape of `perspective` drawn from `drawn`.
fn header(perspective: Perspective, drawn: u8, own: bool) -> Header {
    Header {
        tape: TAPE_VERSION,
        view: baylee_view::VIEW_VERSION,
        build: "b".to_string(),
        source: Source::Hosted,
        perspective: tape_perspective(perspective),
        seat: drawn,
        game: "g".to_string(),
        recorded_by: "b".to_string(),
        own,
        hands: Vec::new(),
    }
}

// ------------------------------------------------------------ choices

/// The viewer's seat first, then every other seat in seat order, then the
/// table, then all hands from the viewer's.
#[test]
fn the_choices_are_the_viewers_seat_then_the_others_then_the_table_then_all_hands() {
    let status = status(1, 4, Perspective::Seat(seat(1)), EVERYTHING);
    assert_eq!(
        status.choices(),
        [
            Perspective::Seat(seat(1)),
            Perspective::Seat(seat(0)),
            Perspective::Seat(seat(2)),
            Perspective::Seat(seat(3)),
            Perspective::Public,
            Perspective::AllHands(seat(1)),
        ]
    );
    // The viewer in the last chair still leads, and appears once.
    let status = status_of_last_chair();
    assert_eq!(
        status.choices(),
        [
            Perspective::Seat(seat(3)),
            Perspective::Seat(seat(0)),
            Perspective::Seat(seat(1)),
            Perspective::Seat(seat(2)),
            Perspective::Public,
            Perspective::AllHands(seat(3)),
        ]
    );
}

fn status_of_last_chair() -> ReplayStatus {
    status(3, 4, Perspective::Seat(seat(3)), EVERYTHING)
}

/// An offer that is off adds nothing: not the table, not all hands, and with
/// no seat offered no seat at all, the viewer's included.
#[test]
fn an_offer_that_is_off_has_no_choice() {
    let offer = |seat: bool, public: bool, all_hands: bool| Perspectives {
        seat,
        public,
        all_hands,
    };
    let choices = |offers| status(0, 3, Perspective::Public, offers).choices();
    let seats = [
        Perspective::Seat(seat(0)),
        Perspective::Seat(seat(1)),
        Perspective::Seat(seat(2)),
    ];

    assert_eq!(
        choices(offer(true, false, false)),
        seats,
        "seats alone: no table, no all hands"
    );
    let mut with_table = seats.to_vec();
    with_table.push(Perspective::Public);
    assert_eq!(choices(offer(true, true, false)), with_table);
    let mut with_hands = seats.to_vec();
    with_hands.push(Perspective::AllHands(seat(0)));
    assert_eq!(choices(offer(true, false, true)), with_hands);

    assert_eq!(
        choices(offer(false, true, true)),
        [Perspective::Public, Perspective::AllHands(seat(0))],
        "no seat offered: not the viewer's either"
    );
    assert_eq!(choices(offer(false, true, false)), [Perspective::Public]);
    assert_eq!(
        choices(offer(false, false, true)),
        [Perspective::AllHands(seat(0))]
    );
    assert!(choices(offer(false, false, false)).is_empty());
}

/// Before a tape names the seats there is the viewer's own, and whatever
/// else is offered.
#[test]
fn before_the_seats_are_named_the_choices_are_the_viewers_own() {
    let mut status = status(2, 4, Perspective::Seat(seat(2)), EVERYTHING);
    status.seats.clear();
    assert_eq!(
        status.choices(),
        [
            Perspective::Seat(seat(2)),
            Perspective::Public,
            Perspective::AllHands(seat(2)),
        ]
    );
}

/// `V` walks the choices in the picker's order and wraps from the last to
/// the first, from every one of them.
#[test]
fn the_next_choice_cycles_and_wraps() {
    let mut status = status(1, 3, Perspective::Seat(seat(1)), EVERYTHING);
    let choices = status.choices();
    assert_eq!(choices.len(), 5);
    for (i, now) in choices.iter().enumerate() {
        status.perspective = *now;
        assert_eq!(
            status.next_choice(),
            choices[(i + 1) % choices.len()],
            "after {now:?}"
        );
    }
    // A full turn comes home.
    status.perspective = choices[0];
    for _ in 0..choices.len() {
        status.perspective = status.next_choice();
    }
    assert_eq!(status.perspective, choices[0]);
}

/// With one choice, or with the perspective shown among none of them, the
/// next is the one shown: nothing to step to, and no division by nothing.
#[test]
fn the_next_choice_stays_where_there_is_nowhere_to_go() {
    let only = Perspectives {
        seat: false,
        public: true,
        all_hands: false,
    };
    let status_only = status(0, 2, Perspective::Public, only);
    assert_eq!(status_only.next_choice(), Perspective::Public);

    let none = Perspectives::default();
    let nothing = status(0, 2, Perspective::Seat(seat(0)), none);
    assert!(nothing.choices().is_empty());
    assert_eq!(nothing.next_choice(), Perspective::Seat(seat(0)));

    // A perspective the source does not offer is not stepped from.
    let strange = status(
        0,
        2,
        Perspective::Seat(seat(3)),
        Perspectives {
            seat: true,
            public: true,
            all_hands: false,
        },
    );
    assert_eq!(strange.next_choice(), Perspective::Seat(seat(3)));
}

// -------------------------------------------------------- the hidden hand

/// The table has no hands, whatever any header or consent says.
#[test]
fn the_public_table_hides_the_hand_always() {
    let mut status = status(0, 3, Perspective::Public, EVERYTHING);
    assert!(status.hand_hidden(Perspective::Public));
    status.header = Some(header(Perspective::Public, 0, true));
    assert!(status.hand_hidden(Perspective::Public), "own: true, still");
    // Shown from a seat, the table is still asked about as the table.
    status.perspective = Perspective::Seat(seat(0));
    status.header = Some(header(Perspective::Seat(seat(0)), 0, true));
    assert!(status.hand_hidden(Perspective::Public));
}

/// The tape shown says it itself: its header's `own` decides for the
/// perspective shown, against the list's consent either way.
#[test]
fn the_tape_shown_says_whether_its_hand_is_hidden() {
    for shown in [Perspective::Seat(seat(2)), Perspective::AllHands(seat(2))] {
        for own in [true, false] {
            for unshared in [true, false] {
                let mut status = status(0, 3, shown, EVERYTHING);
                status.header = Some(header(shown, 2, own));
                if unshared {
                    status.origin.unshared.push(seat(2));
                }
                assert_eq!(
                    status.hand_hidden(shown),
                    !own,
                    "{shown:?}, own {own}, unshared {unshared}"
                );
            }
        }
    }
}

/// A perspective not shown, or a status with no tape yet, goes by the
/// seats whose players did not share.
#[test]
fn another_perspective_goes_by_who_shared() {
    let mut status = status(0, 4, Perspective::Seat(seat(0)), EVERYTHING);
    status.origin.unshared = vec![seat(2)];
    // No header yet: even the perspective "shown" goes by the list.
    assert!(!status.hand_hidden(Perspective::Seat(seat(0))));
    assert!(!status.hand_hidden(Perspective::Seat(seat(1))));
    assert!(status.hand_hidden(Perspective::Seat(seat(2))));
    assert!(!status.hand_hidden(Perspective::Seat(seat(3))));
    assert!(status.hand_hidden(Perspective::AllHands(seat(2))));
    assert!(!status.hand_hidden(Perspective::AllHands(seat(0))));

    // With a header, only the perspective it is for is read off it; the
    // others still go by the list.
    status.header = Some(header(Perspective::Seat(seat(0)), 0, true));
    assert!(!status.hand_hidden(Perspective::Seat(seat(0))));
    assert!(status.hand_hidden(Perspective::Seat(seat(2))));
    status.header = Some(header(Perspective::Seat(seat(0)), 0, false));
    assert!(status.hand_hidden(Perspective::Seat(seat(0))));
    assert!(
        !status.hand_hidden(Perspective::Seat(seat(1))),
        "seat 1 shared: the header of seat 0's tape says nothing of it"
    );
}

/// A game where everyone shared (a local record, or a table of consenting
/// players) hides no hand but the table's.
#[test]
fn nobody_unshared_hides_nothing_but_the_table() {
    let status = status(0, 3, Perspective::Seat(seat(0)), EVERYTHING);
    for s in 0..3 {
        assert!(!status.hand_hidden(Perspective::Seat(seat(s))));
        assert!(!status.hand_hidden(Perspective::AllHands(seat(s))));
    }
    assert!(status.hand_hidden(Perspective::Public));
}

// ------------------------------------------------------------- the words

/// The viewer's seat is "Your seat", another is the owner's name, the
/// table and all hands have their own.
#[test]
fn another_seat_is_called_by_its_players_name() {
    let status = status(1, 3, Perspective::Seat(seat(1)), EVERYTHING);
    assert_eq!(
        status.words(Perspective::Seat(seat(1)), Lang::En),
        "Your seat"
    );
    assert_eq!(
        status.words(Perspective::Seat(seat(0)), Lang::En),
        "Alice#af03\u{2019}s seat"
    );
    assert_eq!(
        status.words(Perspective::Seat(seat(2)), Lang::En),
        "Cy#0003\u{2019}s seat"
    );
    assert_eq!(
        status.words(Perspective::Seat(seat(2)), Lang::De),
        "Platz von Cy#0003"
    );
    assert_eq!(
        status.words(Perspective::Seat(seat(1)), Lang::De),
        "Dein Platz"
    );
    assert_eq!(status.words(Perspective::Public, Lang::En), "The table");
    assert_eq!(
        status.words(Perspective::AllHands(seat(1)), Lang::En),
        "All hands"
    );
    assert_eq!(
        status.words(Perspective::AllHands(seat(1)), Lang::De),
        "Alle H\u{e4}nde"
    );
}

/// A seat no tape has named yet is still a seat the words can say: by its
/// number, counted from one.
#[test]
fn an_unnamed_seat_is_called_by_its_number() {
    let mut status = status(0, 4, Perspective::Seat(seat(0)), EVERYTHING);
    status.seats.clear();
    let words = status.words(Perspective::Seat(seat(2)), Lang::En);
    assert!(words.contains('3'), "{words}");
    assert!(words.contains("seat"), "{words}");
}

/// The table's top line carries the seat shown by name.
#[test]
fn the_top_line_names_the_seat_shown() {
    let mut status = status(1, 3, Perspective::Seat(seat(0)), EVERYTHING);
    status.origin.place = "gw".to_string();
    let line = top_line(&status, Lang::En);
    assert!(line.starts_with("Replay"), "{line}");
    assert!(line.contains("gw"), "{line}");
    assert!(line.ends_with("Alice#af03\u{2019}s seat"), "{line}");
    status.perspective = Perspective::Seat(seat(1));
    assert!(top_line(&status, Lang::En).ends_with("Your seat"));
}

// ------------------------------------------------------ the folding hand

/// With no override the hand stands open wherever it shows, and shut
/// wherever it is hidden: the table, an unshared seat, a tape that says
/// `own: false`.
#[test]
fn the_hand_is_open_by_itself_unless_it_is_hidden() {
    let ui = BarUi::default();
    let mut mine = status(0, 3, Perspective::Seat(seat(0)), EVERYTHING);
    mine.header = Some(header(Perspective::Seat(seat(0)), 0, true));
    assert!(ui.hand_open(&mine));

    let table = status(0, 3, Perspective::Public, EVERYTHING);
    assert!(!ui.hand_open(&table));

    // A seat whose player did not share: the tape says it is the public
    // table drawn from there.
    let mut hidden = status(0, 3, Perspective::Seat(seat(2)), EVERYTHING);
    hidden.header = Some(header(Perspective::Seat(seat(2)), 2, false));
    assert!(!ui.hand_open(&hidden));
    // The same seat before its tape comes: the list's consent says it.
    hidden.header = None;
    hidden.origin.unshared = vec![seat(2)];
    assert!(!ui.hand_open(&hidden));
    hidden.origin.unshared.clear();
    assert!(ui.hand_open(&hidden));

    let mut all = status(0, 3, Perspective::AllHands(seat(0)), EVERYTHING);
    all.header = Some(header(Perspective::AllHands(seat(0)), 0, true));
    assert!(ui.hand_open(&all));
}

/// The viewer's choice holds for the perspective it was made on, hidden hand
/// or not, and only there.
#[test]
fn an_override_applies_only_to_the_perspective_it_was_set_for() {
    let mut ui = BarUi {
        hand: Some((Perspective::Seat(seat(0)), false)),
        ..BarUi::default()
    };
    let mut mine = status(0, 3, Perspective::Seat(seat(0)), EVERYTHING);
    mine.header = Some(header(Perspective::Seat(seat(0)), 0, true));
    assert!(!ui.hand_open(&mine), "folded by hand");

    // On another perspective it is ignored: the table stays shut, another
    // seat that shows its hand stays open.
    let table = status(0, 3, Perspective::Public, EVERYTHING);
    assert!(!ui.hand_open(&table));
    let mut other = status(0, 3, Perspective::Seat(seat(1)), EVERYTHING);
    other.header = Some(header(Perspective::Seat(seat(1)), 1, true));
    assert!(ui.hand_open(&other), "seat 0's fold is not seat 1's");
    // Another seat is another perspective, and so is all hands from the same
    // one.
    let mut all = status(0, 3, Perspective::AllHands(seat(0)), EVERYTHING);
    all.header = Some(header(Perspective::AllHands(seat(0)), 0, true));
    assert!(ui.hand_open(&all));

    // And it applies again when the perspective comes back.
    assert!(!ui.hand_open(&mine));

    // Opened by hand, it is open where it would be shut by itself.
    ui.hand = Some((Perspective::Public, true));
    assert!(ui.hand_open(&table));
    // And it does not carry over to the seat that shows its hand.
    assert!(ui.hand_open(&status(0, 3, Perspective::Seat(seat(2)), EVERYTHING)));
}
