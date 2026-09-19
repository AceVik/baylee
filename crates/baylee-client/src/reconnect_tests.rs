use super::*;
use baylee_client_core::reconnect::Retry;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A host whose link state the test holds the other end of.
///
/// It never actually comes back on its own: `reconnect` counts the dial
/// and leaves the state alone, so the schedule can be watched running out
/// rather than being cut short by a lucky reconnection.
struct FakeHost {
    state: Arc<Mutex<LinkState>>,
    dials: Arc<Mutex<usize>>,
}

impl DuelHost for FakeHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        Vec::new()
    }
    fn submit(&mut self, _: PlayerAction) {}
    fn seat(&self) -> PlayerId {
        PlayerId::new(0)
    }
    fn link(&self) -> LinkState {
        *self.state.lock().unwrap()
    }
    fn reconnect(&mut self) -> Result<(), String> {
        *self.dials.lock().unwrap() += 1;
        Ok(())
    }
}

/// How many times the app said the table could not be reached.
///
/// Counted by a real reader rather than by inspecting the buffer, which
/// is also what proves the report is deliverable at all.
#[derive(Resource, Default)]
struct Unreachables(usize);

/// Drains the reports the way an embedding shell would.
fn count_unreachable(mut reader: MessageReader<DuelReport>, mut seen: ResMut<Unreachables>) {
    seen.0 += reader
        .read()
        .filter(|r| matches!(r, DuelReport::Unreachable))
        .count();
}

/// An app with just enough in it to run the one system.
fn app_with(state: LinkState) -> (App, Arc<Mutex<LinkState>>, Arc<Mutex<usize>>) {
    let link = Arc::new(Mutex::new(state));
    let dials = Arc::new(Mutex::new(0));
    let host = FakeHost {
        state: Arc::clone(&link),
        dials: Arc::clone(&dials),
    };
    let mut app = App::new();
    app.init_resource::<Duel>()
        .init_resource::<Reconnect>()
        .insert_resource(Time::<()>::default())
        .insert_resource(InstalledHost(Box::new(host)))
        .add_message::<DuelReport>()
        .init_resource::<Unreachables>()
        .add_systems(
            Update,
            (super::keep_the_table_connected, count_unreachable).chain(),
        );
    (app, link, dials)
}

/// Moves the clock on and runs one frame.
fn advance(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time<()>>()
        .advance_by(Duration::from_secs_f32(seconds));
    app.update();
}

/// The whole point: nothing outside the client asks for this. A socket
/// that goes away is dialled again on the client's own initiative, which
/// is what `NetworkHost::redial` could always do and what nothing ever
/// called — a dropped connection simply ended the game.
#[test]
fn a_table_that_drops_is_dialled_again_without_anyone_asking() {
    let (mut app, _link, dials) = app_with(LinkState::Down);

    advance(&mut app, 0.1);
    assert_eq!(*dials.lock().unwrap(), 0, "not instantly");
    assert_eq!(
        app.world().resource::<Duel>().link_note,
        Some(Phrase::LinkLost),
        "but the player is told at once"
    );

    advance(&mut app, 0.5);
    assert_eq!(*dials.lock().unwrap(), 1, "half a second in");
}

/// A host with no socket must never enter the schedule. An in-process
/// engine cannot be disconnected from, so a client that treated it as a
/// dead link would "reconnect" to it twelve times and then tell a solo
/// player their table was unreachable.
#[test]
fn a_local_host_is_never_dialled() {
    let (mut app, _link, dials) = app_with(LinkState::Local);
    for _ in 0..40 {
        advance(&mut app, 5.0);
    }
    assert_eq!(*dials.lock().unwrap(), 0);
    assert_eq!(app.world().resource::<Duel>().link_note, None);
}

/// A dial in flight is not a reason to dial again. Without this the
/// system would fire once per frame for as long as the socket took to
/// open, which is every frame of the two seconds a bad network needs.
///
/// The second half of this test used to assert `LinkLost` after a full
/// minute inside `Connecting`, and that assertion was the defect written
/// down: the schedule does not advance during a dial, so a client that
/// measured the outage by the schedule would have gone on saying the
/// connection had just dropped for as long as one socket took to fail.
/// `stayed_down` is what it is measured by now, and this is the case that
/// separates the two.
#[test]
fn a_dial_in_flight_is_left_alone_but_still_counts() {
    let (mut app, _link, dials) = app_with(LinkState::Connecting);
    advance(&mut app, 0.1);
    assert_eq!(
        app.world().resource::<Duel>().link_note,
        Some(Phrase::LinkLost),
        "a dial that has just gone out is still a hiccup"
    );
    for _ in 0..120 {
        advance(&mut app, 0.5);
    }
    assert_eq!(*dials.lock().unwrap(), 0, "it is already dialling");
    assert_eq!(
        app.world().resource::<Duel>().link_note,
        Some(Phrase::LinkStandIn),
        "a minute inside one dial is not a hiccup, whatever the schedule did"
    );
}

/// The sentence turns once and stays turned.
///
/// `Connecting` and `Down` alternate for the whole of a long outage — one
/// arm for the dial, the other for the wait after it — so the two have to
/// answer the question the same way. The `Connecting` arm named `LinkLost`
/// outright, which on a capped schedule would have flipped the bar back to
/// the short sentence once every fifteen seconds for as long as the drop
/// lasted.
#[test]
fn the_sentence_does_not_flicker_as_dials_come_and_go() {
    let (mut app, link, _dials) = app_with(LinkState::Down);
    advance(&mut app, Retry::PATIENCE + 1.0);
    assert_eq!(
        app.world().resource::<Duel>().link_note,
        Some(Phrase::LinkStandIn),
        "the drop has outlived its patience"
    );

    for _ in 0..3 {
        *link.lock().unwrap() = LinkState::Connecting;
        advance(&mut app, 0.1);
        assert_eq!(
            app.world().resource::<Duel>().link_note,
            Some(Phrase::LinkStandIn),
            "a dial going out took the bar back to the first sentence"
        );
        *link.lock().unwrap() = LinkState::Down;
        advance(&mut app, 0.1);
        assert_eq!(
            app.world().resource::<Duel>().link_note,
            Some(Phrase::LinkStandIn),
            "and the wait after it took the bar back"
        );
    }
}

/// The schedule ends, and says so once rather than once a frame. An
/// unbounded retry against a game the gateway has already finished would
/// spin until the player closed the window — and "that game is over"
/// arrives as a refusal string, not as something a client can match on.
#[test]
fn a_table_that_cannot_be_reached_stops_and_says_so() {
    let (mut app, _link, dials) = app_with(LinkState::Down);
    for _ in 0..80 {
        advance(&mut app, 20.0);
    }
    assert_eq!(
        *dials.lock().unwrap(),
        Retry::GIVE_UP as usize,
        "it stopped where the schedule said it would"
    );
    assert_eq!(
        app.world().resource::<Duel>().link_note,
        Some(Phrase::LinkGaveUp)
    );

    // Counted as they were written rather than read off the buffer at the
    // end: `Messages` is double-buffered and drops what nobody read
    // within two frames, so a test that looked afterwards would find
    // nothing however many had been sent.
    assert_eq!(
        app.world().resource::<Unreachables>().0,
        1,
        "told once, not once a frame"
    );
}

/// A table that comes back takes the notice off the bar and resets the
/// schedule, so the *next* drop is dialled promptly rather than at the
/// cap the last one ended on.
#[test]
fn a_table_that_comes_back_clears_the_notice_and_the_schedule() {
    let (mut app, link, dials) = app_with(LinkState::Down);
    for _ in 0..4 {
        advance(&mut app, 20.0);
    }
    let during = *dials.lock().unwrap();
    assert!(during >= 4, "it was dialling: {during}");

    *link.lock().unwrap() = LinkState::Up;
    advance(&mut app, 0.1);
    assert_eq!(app.world().resource::<Duel>().link_note, None);
    assert_eq!(*dials.lock().unwrap(), during, "and stopped dialling");

    // The *next* drop is a hiccup again, not the outage this one became.
    // Eighty seconds ago the house was being announced; a player whose link
    // wobbles for a frame should not be told that a second time.
    *link.lock().unwrap() = LinkState::Down;
    advance(&mut app, 0.1);
    assert_eq!(
        app.world().resource::<Duel>().link_note,
        Some(Phrase::LinkLost),
        "the new drop inherited the old one's outage"
    );

    // Down again: prompt, not at the cap the last outage ended on.
    advance(&mut app, 0.6);
    assert_eq!(
        *dials.lock().unwrap(),
        during + 1,
        "the next drop starts the schedule over"
    );
}
