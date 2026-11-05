//! The deck builder's saves, against a loopback gateway that answers a save
//! only when a test tells it to, so a save can be held in flight.

#[allow(clippy::wildcard_imports)] // the lobby's own test vocabulary
use super::*;
use crate::buildui::SaveState;
use std::collections::VecDeque;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// One save as it reached the gateway.
#[derive(Clone, Debug)]
struct Seen {
    /// `PUT /decks/d1`, the request line's first two words.
    route: String,
    body: serde_json::Value,
}

#[derive(Default)]
struct Book {
    saves: Vec<Seen>,
    answers: VecDeque<(u16, String)>,
    /// Everything else that was asked.
    asked: Vec<String>,
}

/// A private, bounded gateway: every deck save waits for [`Gateway::answer`],
/// and everything else the lobby asks is answered at once with nothing.
struct Gateway {
    base: String,
    stop: Arc<AtomicBool>,
    book: Arc<Mutex<Book>>,
    worker: Option<JoinHandle<()>>,
}

impl Gateway {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let book = Arc::new(Mutex::new(Book::default()));
        let (stopping, booked) = (Arc::clone(&stop), Arc::clone(&book));
        let worker = thread::spawn(move || {
            let mut open = Vec::new();
            while !stopping.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let (stop, book) = (Arc::clone(&stopping), Arc::clone(&booked));
                        open.push(thread::spawn(move || serve(stream, &book, &stop)));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(_) => break,
                }
            }
            for connection in open {
                let _ = connection.join();
            }
        });
        Self {
            base,
            stop,
            book,
            worker: Some(worker),
        }
    }

    /// Every deck save that has arrived, answered or not.
    fn saves(&self) -> Vec<Seen> {
        self.book.lock().unwrap().saves.clone()
    }

    /// Whether anything else has asked `route`.
    fn asked(&self, route: &str) -> bool {
        self.book.lock().unwrap().asked.iter().any(|r| r == route)
    }

    /// Answers the save waiting longest.
    fn answer(&self, status: u16, body: &str) {
        self.book
            .lock()
            .unwrap()
            .answers
            .push_back((status, body.to_owned()));
    }
}

impl Drop for Gateway {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// One request, whole: its head and its body.
fn read_request(stream: &mut TcpStream) -> Option<(String, Vec<u8>)> {
    let until = Instant::now() + Duration::from_secs(2);
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&bytes[..end]).into_owned();
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            if bytes.len() >= end + 4 + length {
                return Some((head, bytes[end + 4..end + 4 + length].to_vec()));
            }
        }
        if Instant::now() > until {
            return None;
        }
        let count = stream.read(&mut buffer).ok()?;
        if count == 0 {
            return None;
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}

fn serve(mut stream: TcpStream, book: &Mutex<Book>, stop: &AtomicBool) {
    // An accepted socket inherits the listener's non-blocking mode on macOS
    // and the BSDs.
    if stream.set_nonblocking(false).is_err()
        || stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .is_err()
    {
        return;
    }
    let Some((head, body)) = read_request(&mut stream) else {
        return;
    };
    let mut words = head.split_whitespace();
    let route = format!(
        "{} {}",
        words.next().unwrap_or_default(),
        words.next().unwrap_or_default()
    );
    let saves = route == "POST /decks" || route.starts_with("PUT /decks/");
    let (status, answer) = if saves {
        book.lock().unwrap().saves.push(Seen {
            route,
            body: serde_json::from_slice(&body).unwrap_or_default(),
        });
        loop {
            if stop.load(Ordering::Acquire) {
                return;
            }
            if let Some(answer) = book.lock().unwrap().answers.pop_front() {
                break answer;
            }
            thread::sleep(Duration::from_millis(1));
        }
    } else {
        let listed = route == "GET /decks";
        book.lock().unwrap().asked.push(route);
        if listed {
            (200, r#"[{"id":"d1","name":"Allytifact"}]"#.to_owned())
        } else {
            (200, "{}".to_owned())
        }
    };
    let _ = write!(
        stream,
        "HTTP/1.1 {status} Answer\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{answer}",
        answer.len()
    );
}

/// A frame of the app, by the clock a test keeps: 50 ms each, whatever the
/// machine is doing.
const FRAME: Duration = Duration::from_millis(50);

fn state(app: &App) -> &LobbyState {
    app.world().resource::<LobbyState>()
}

fn state_mut(app: &mut App) -> Mut<'_, LobbyState> {
    app.world_mut().resource_mut::<LobbyState>()
}

/// Runs frames until `done`, or fails naming what never came.
fn until(app: &mut App, what: &str, mut done: impl FnMut(&mut App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done(app) {
        assert!(Instant::now() < deadline, "never: {what}");
        app.update();
        thread::sleep(Duration::from_millis(1));
    }
}

/// `count` frames: far enough on a test's clock for a settled edit to act.
fn frames(app: &mut App, count: usize) {
    for _ in 0..count {
        app.update();
    }
}

/// A signed-in lobby speaking to `gateway`.
fn signed_in(base: &str) -> App {
    let mut app = headless();
    stocked(&mut app);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(FRAME));
    state_mut(&mut app).gateway = base.to_owned();
    app
}

/// The builder open on the stored deck `d1`, four elves in it.
fn editing_a_stored_deck(base: &str) -> App {
    let mut app = signed_in(base);
    open_stored(&mut app, &["4 Llanowar Elves"]);
    app
}

fn open_stored(app: &mut App, cards: &[&str]) {
    let mut state = state_mut(app);
    state.build = crate::buildui::BuildUi::opened();
    let _ = state.lobby.edit_deck(0);
    state.lobby.apply(LobbyEvent::DeckLoaded {
        id: "d1".to_owned(),
        name: "Allytifact".to_owned(),
        cards: cards.iter().map(|&row| row.to_owned()).collect(),
        sideboard: Vec::new(),
        commanders: Vec::new(),
        format: String::new(),
    });
    drop(state);
    frames(app, 3);
}

fn add_a_forest(app: &mut App) {
    assert!(state_mut(app).lobby.builder_mut().add(1, Zone::Main));
}

/// A refusal leaves the deck unsaved, says so, keeps its draft, and Back
/// asks: nothing has reached the gateway, and the player has to hear it.
#[test]
fn a_refused_save_leaves_the_deck_dirty_and_tells_the_player() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the save went out", |_| {
        gateway.saves().len() == 1
    });
    assert!(
        state(&app).lobby.builder().dirty(),
        "unsaved until it is answered"
    );

    gateway.answer(400, r#"{"error":"invalid card list"}"#);
    until(&mut app, "the refusal arrived", |app| {
        state(app).build.save == SaveState::Failed
    });
    assert!(state(&app).lobby.builder().dirty());
    assert!(
        state(&app).lobby.status().contains("invalid card list"),
        "said: {}",
        state(&app).lobby.status()
    );

    frames(&mut app, 30);
    let kept = state_mut(&mut app).drafts.of(Some("d1")).cloned();
    assert!(
        kept.is_some_and(|draft| draft.main.iter().any(|row| row.contains("Forest"))),
        "the draft is the only copy now"
    );
    assert_eq!(gateway.saves().len(), 1, "and no save loops on a refusal");

    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert!(state(&app).confirm_leave, "Back asks");
    assert_eq!(state(&app).lobby.screen(), &Screen::Build);
}

/// A network that is gone is a failed save like any other.
#[test]
fn a_save_nobody_answers_leaves_the_deck_dirty_and_tells_the_player() {
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let mut app = editing_a_stored_deck(&base);
    add_a_forest(&mut app);
    until(&mut app, "the save failed", |app| {
        state(app).build.save == SaveState::Failed
    });
    assert!(state(&app).lobby.builder().dirty());
    assert!(!state(&app).lobby.status().is_empty());
}

/// Retry, the Save button, sends the deck again and clears the failure.
#[test]
fn save_retries_a_failed_save() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the save went out", |_| {
        gateway.saves().len() == 1
    });
    gateway.answer(500, r#"{"error":"the database is down"}"#);
    until(&mut app, "the refusal arrived", |app| {
        state(app).build.save == SaveState::Failed
    });

    press(&mut app, Press::Build(BuildPress::SaveDeck));
    until(&mut app, "the retry went out", |_| {
        gateway.saves().len() == 2
    });
    gateway.answer(204, "");
    until(&mut app, "the retry landed", |app| {
        !state(app).lobby.builder().dirty()
    });
    assert_ne!(state(&app).build.save, SaveState::Failed);
    assert!(state_mut(&mut app).drafts.of(Some("d1")).is_none());
}

/// A session that ended under a save keeps what the player was writing.
#[test]
fn a_session_that_ends_under_a_save_keeps_the_draft() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the save went out", |_| {
        gateway.saves().len() == 1
    });
    gateway.answer(401, r#"{"error":"invalid session"}"#);
    until(&mut app, "the session ended", |app| {
        matches!(state(app).lobby.screen(), Screen::SignIn { .. })
    });
    frames(&mut app, 2);
    let kept = state_mut(&mut app).drafts.of(Some("d1")).cloned();
    assert!(
        kept.is_some_and(|draft| draft.main.iter().any(|row| row.contains("Forest"))),
        "nothing else holds the edit now"
    );
}

/// A deck the gateway would refuse, or that would lose a card the pool has
/// not got, is never sent: `rows` leaves such a card out.
#[test]
fn a_deck_that_may_not_save_is_not_sent() {
    let gateway = Gateway::new();
    let mut app = signed_in(&gateway.base);
    open_stored(&mut app, &["4 Llanowar Elves", "1 No Such Card"]);
    assert!(!state(&app).lobby.builder().saveable());
    add_a_forest(&mut app);
    frames(&mut app, 30);
    assert!(gateway.saves().is_empty(), "{:?}", gateway.saves());

    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert!(state(&app).confirm_leave, "and Back says it was not saved");
}

/// A deck that saves leaves at once, the save going out on the way.
#[test]
fn a_deck_that_saves_leaves_at_once() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert!(!state(&app).confirm_leave);
    assert_eq!(state(&app).lobby.screen(), &Screen::Table);
    until(&mut app, "the save went out", |_| {
        !gateway.saves().is_empty()
    });
}

/// Discard means discard: after a failed save the edit is not sent on the
/// way out, and its draft goes.
#[test]
fn discarding_a_failed_save_sends_nothing_more() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the save went out", |_| {
        gateway.saves().len() == 1
    });
    gateway.answer(500, r#"{"error":"the database is down"}"#);
    until(&mut app, "the refusal arrived", |app| {
        state(app).build.save == SaveState::Failed
    });
    frames(&mut app, 30);

    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    press(&mut app, Press::Build(BuildPress::DiscardAndLeave));
    assert_eq!(state(&app).lobby.screen(), &Screen::Table);
    frames(&mut app, 30);
    assert_eq!(gateway.saves().len(), 1, "{:?}", gateway.saves());
    assert!(state_mut(&mut app).drafts.of(Some("d1")).is_none());
}

/// What the deck held before this visit reaches the history with the visit's
/// first change: every save asks for it until the gateway says it was kept,
/// and only then does a save update the deck where it stands.
#[test]
fn a_visits_first_change_keeps_history_and_the_rest_save_in_place() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the first save", |_| gateway.saves().len() == 1);
    assert_eq!(gateway.saves()[0].body["versioned"], true);
    gateway.answer(200, r#"{"archived":true}"#);
    until(&mut app, "it landed", |app| {
        !state(app).lobby.builder().dirty()
    });

    add_a_forest(&mut app);
    until(&mut app, "the second save", |_| gateway.saves().len() == 2);
    assert_eq!(gateway.saves()[1].body["versioned"], false);
}

/// A rename writes no history, so the change after it still has to.
#[test]
fn a_rename_leaves_the_history_for_the_first_change() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    state_mut(&mut app).lobby.builder_mut().set_name("Elves");
    until(&mut app, "the rename's save", |_| {
        gateway.saves().len() == 1
    });
    gateway.answer(200, r#"{"archived":false}"#);
    until(&mut app, "it landed", |app| {
        !state(app).lobby.builder().dirty()
    });

    add_a_forest(&mut app);
    until(&mut app, "the change's save", |_| {
        gateway.saves().len() == 2
    });
    assert_eq!(gateway.saves()[1].body["versioned"], true);
}

/// A gateway that does not say keeps asking to keep history, which at worst
/// writes a row too many and never loses one.
#[test]
fn a_gateway_that_says_nothing_never_saves_in_place() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the first save", |_| gateway.saves().len() == 1);
    gateway.answer(204, "");
    until(&mut app, "it landed", |app| {
        !state(app).lobby.builder().dirty()
    });

    add_a_forest(&mut app);
    until(&mut app, "the second save", |_| gateway.saves().len() == 2);
    assert_eq!(gateway.saves()[1].body["versioned"], true);
}

/// A deck this visit created has nothing behind it to keep: after the POST
/// the saves go in place, and leaving writes no row.
#[test]
fn a_new_deck_has_no_history_to_write() {
    let gateway = Gateway::new();
    let mut app = signed_in(&gateway.base);
    {
        let mut state = state_mut(&mut app);
        state.build = crate::buildui::BuildUi::opened();
        let _ = state.lobby.build_deck();
    }
    frames(&mut app, 3);
    add_a_forest(&mut app);
    until(&mut app, "the deck was created", |_| {
        gateway.saves().len() == 1
    });
    assert_eq!(gateway.saves()[0].route, "POST /decks");
    gateway.answer(200, r#"{"deck_id":"n1"}"#);
    until(&mut app, "it landed", |app| {
        state(app).lobby.builder().editing() == Some("n1")
    });

    add_a_forest(&mut app);
    until(&mut app, "the next save", |_| gateway.saves().len() == 2);
    assert_eq!(gateway.saves()[1].route, "PUT /decks/n1");
    assert_eq!(gateway.saves()[1].body["versioned"], false);
}

/// Opening the deck again is a new visit: its first change keeps history.
#[test]
fn every_visit_writes_its_own_entry() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the first save", |_| gateway.saves().len() == 1);
    gateway.answer(200, r#"{"archived":true}"#);
    until(&mut app, "it landed", |app| {
        !state(app).lobby.builder().dirty()
    });
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    until(&mut app, "the lobby settled", |app| {
        !state(app).lobby.busy()
    });

    open_stored(&mut app, &["4 Llanowar Elves", "1 Forest"]);
    add_a_forest(&mut app);
    until(&mut app, "the next visit's save", |_| {
        gateway.saves().len() == 2
    });
    assert_eq!(gateway.saves()[1].body["versioned"], true);
}

/// Long enough on a test's clock for an edit to settle and go out.
const SETTLED: usize = 12;

/// One save is out at a time: requests run on threads of their own, so a
/// second one out could land first and leave the gateway the older list. The
/// edits made under a save go out as one save behind it.
#[test]
fn a_deck_has_one_save_out_and_the_rest_wait_as_one() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the first save", |_| gateway.saves().len() == 1);
    for _ in 0..3 {
        add_a_forest(&mut app);
        frames(&mut app, SETTLED);
    }
    assert_eq!(gateway.saves().len(), 1, "nothing else goes out under it");

    gateway.answer(200, r#"{"archived":true}"#);
    until(&mut app, "the rest went out", |_| {
        gateway.saves().len() == 2
    });
    let last = &gateway.saves()[1];
    assert_eq!(last.body["cards"][1], "4 Forest", "{last:?}");
    assert_eq!(
        last.body["versioned"], false,
        "told by the answer before it"
    );

    gateway.answer(200, r#"{"archived":false}"#);
    until(&mut app, "all of it landed", |app| {
        !state(app).lobby.builder().dirty()
    });
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 2);
}

/// A name typed is a run of edits, and a run of edits is one save.
#[test]
fn a_name_typed_is_one_save() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    for ch in " Elves".chars() {
        state_mut(&mut app).lobby.builder_mut().type_name(ch);
        frames(&mut app, 1);
    }
    until(&mut app, "the save went out", |_| {
        !gateway.saves().is_empty()
    });
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 1, "{:?}", gateway.saves());
    assert_eq!(gateway.saves()[0].body["name"], "Allytifact Elves");
}

/// A letter typed under a save is one change with the letters before it, and
/// still not saved: the answer must not call the deck clean.
#[test]
fn a_letter_typed_under_a_save_is_not_lost() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    state_mut(&mut app).lobby.builder_mut().type_name('A');
    until(&mut app, "the save went out", |_| {
        gateway.saves().len() == 1
    });
    state_mut(&mut app).lobby.builder_mut().type_name('B');
    frames(&mut app, 2);

    gateway.answer(200, r#"{"archived":false}"#);
    until(&mut app, "the letter went out", |_| {
        gateway.saves().len() == 2
    });
    assert_eq!(gateway.saves()[1].body["name"], "AllytifactAB");
    assert!(
        state(&app).lobby.builder().dirty(),
        "until that is answered"
    );
    gateway.answer(200, r#"{"archived":false}"#);
    until(&mut app, "it landed", |app| {
        !state(app).lobby.builder().dirty()
    });
}

/// Save, the button, is a save like the others: it skips the wait and stands
/// in the same line, and the player hears when it has landed.
#[test]
fn the_save_button_stands_in_the_same_line() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the first save", |_| gateway.saves().len() == 1);
    add_a_forest(&mut app);
    press(&mut app, Press::Build(BuildPress::SaveDeck));
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 1, "{:?}", gateway.saves());

    gateway.answer(200, r#"{"archived":true}"#);
    until(&mut app, "the second went out", |_| {
        gateway.saves().len() == 2
    });
    gateway.answer(200, r#"{"archived":false}"#);
    until(&mut app, "it landed", |app| {
        !state(app).lobby.builder().dirty()
    });
    assert_eq!(state(&app).lobby.status(), "deck saved");
}

/// What leaving sends goes behind the save out, so a late save can never
/// land on top of the last one.
#[test]
fn leaving_under_a_save_sends_the_rest_behind_it() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the first save", |_| gateway.saves().len() == 1);
    add_a_forest(&mut app);
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert_eq!(state(&app).lobby.screen(), &Screen::Table);
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 1, "{:?}", gateway.saves());

    gateway.answer(200, r#"{"archived":true}"#);
    until(&mut app, "the last went out", |_| {
        gateway.saves().len() == 2
    });
    assert_eq!(gateway.saves()[1].body["cards"][1], "2 Forest");
    assert_eq!(gateway.saves()[1].body["versioned"], false);
    gateway.answer(200, r#"{"archived":false}"#);
    until(&mut app, "the draft was forgotten", |app| {
        state_mut(app).drafts.of(Some("d1")).is_none()
    });
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 2, "{:?}", gateway.saves());
}

/// The builder open on a new deck, one forest in it, that deck's `POST` out.
fn creating_a_deck(gateway: &Gateway) -> App {
    let mut app = signed_in(&gateway.base);
    {
        let mut state = state_mut(&mut app);
        state.build = crate::buildui::BuildUi::opened();
        let _ = state.lobby.build_deck();
    }
    frames(&mut app, 3);
    add_a_forest(&mut app);
    until(&mut app, "the deck's POST went out", |_| {
        gateway.saves().len() == 1
    });
    assert_eq!(gateway.saves()[0].route, "POST /decks");
    app
}

/// Leaving under a deck's first save makes one deck: what leaving sends
/// waits for the id, however many ways there are out of the builder.
#[test]
fn leaving_a_new_deck_under_its_post_makes_one_deck() {
    let gateway = Gateway::new();
    let mut app = creating_a_deck(&gateway);
    add_a_forest(&mut app);
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert_eq!(state(&app).lobby.screen(), &Screen::Table);
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 1, "{:?}", gateway.saves());

    gateway.answer(200, r#"{"deck_id":"n1"}"#);
    until(&mut app, "the rest went out", |_| {
        gateway.saves().len() == 2
    });
    assert_eq!(gateway.saves()[1].route, "PUT /decks/n1");
    assert_eq!(gateway.saves()[1].body["cards"][0], "2 Forest");
    gateway.answer(200, r#"{"archived":false}"#);
    until(&mut app, "nothing is left to send", |app| {
        !state(app).deck_save_out()
    });
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 2, "{:?}", gateway.saves());
}

/// A late answer is about the deck it was sent for. Opening another deck
/// under a `POST` used to give that deck the new id and its favourites, and
/// its next edit went over the first deck.
#[test]
fn an_answer_belongs_to_the_deck_it_was_sent_for() {
    let gateway = Gateway::new();
    let mut app = creating_a_deck(&gateway);
    {
        let mut prefs = app.world_mut().resource_mut::<crate::prefs::Prefs>();
        prefs.edit().toggle_deck_card_favourite("new", "Forest");
        prefs.edit().toggle_deck_card_favourite("d1", "Island");
    }
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    until(&mut app, "the lobby settled", |app| {
        !state(app).lobby.busy()
    });
    open_stored(&mut app, &["4 Llanowar Elves"]);

    gateway.answer(200, r#"{"deck_id":"n1"}"#);
    until(&mut app, "the answer landed", |app| {
        !state(app).deck_save_out()
    });
    assert_eq!(state(&app).lobby.builder().editing(), Some("d1"));
    let prefs = app.world().resource::<crate::prefs::Prefs>().all();
    assert_eq!(prefs.deck_card_favourites_for("d1"), ["Island"]);
    assert_eq!(
        prefs.deck_card_favourites_for("n1"),
        ["Forest"],
        "the new deck's own favourites did follow it"
    );

    add_a_forest(&mut app);
    until(&mut app, "the open deck's save", |_| {
        gateway.saves().len() == 2
    });
    assert_eq!(gateway.saves()[1].route, "PUT /decks/d1");
    assert_eq!(gateway.saves()[1].body["versioned"], true);
}

/// A refusal is the same: the deck it was about stays on the device, and the
/// deck open now has nothing to answer for.
#[test]
fn a_failure_belongs_to_the_deck_it_was_sent_for() {
    let gateway = Gateway::new();
    let mut app = creating_a_deck(&gateway);
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    until(&mut app, "the lobby settled", |app| {
        !state(app).lobby.busy()
    });
    open_stored(&mut app, &["4 Llanowar Elves"]);

    gateway.answer(500, r#"{"error":"the database is down"}"#);
    until(&mut app, "the player heard", |app| {
        state(app).lobby.status().contains("the database is down")
    });
    assert_ne!(state(&app).build.save, SaveState::Failed);
    assert!(state_mut(&mut app).drafts.of(None).is_some());
}

/// A late answer forgets the draft of what it saved and nothing else: "new"
/// is the next new deck's by then.
#[test]
fn a_late_answer_leaves_the_next_decks_draft() {
    let gateway = Gateway::new();
    let mut app = creating_a_deck(&gateway);
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    until(&mut app, "the lobby settled", |app| {
        !state(app).lobby.busy()
    });
    {
        let mut state = state_mut(&mut app);
        state.build = crate::buildui::BuildUi::opened();
        let _ = state.lobby.build_deck();
    }
    frames(&mut app, 3);
    assert!(state_mut(&mut app).lobby.builder_mut().add(0, Zone::Main));
    frames(&mut app, 30);

    gateway.answer(200, r#"{"deck_id":"n1"}"#);
    until(&mut app, "the next deck's save went out", |_| {
        gateway.saves().len() == 2
    });
    let kept = state_mut(&mut app).drafts.of(None).cloned();
    assert!(
        kept.is_some_and(|draft| draft.main.iter().any(|row| row.contains("Elves"))),
        "the second deck's draft is still its only copy"
    );
    assert_eq!(gateway.saves()[1].route, "POST /decks", "and its own deck");
}

/// A new deck that never reached the gateway has its draft and nothing else:
/// saving some other deck must not take it.
#[test]
fn a_failed_new_deck_keeps_its_draft_through_another_decks_save() {
    let gateway = Gateway::new();
    let mut app = creating_a_deck(&gateway);
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    gateway.answer(500, r#"{"error":"the database is down"}"#);
    until(&mut app, "the refusal arrived", |app| {
        !state(app).deck_save_out()
    });
    until(&mut app, "the lobby settled", |app| !state(app).lobby.busy());
    assert!(state_mut(&mut app).drafts.of(None).is_some());

    open_stored(&mut app, &["4 Llanowar Elves"]);
    add_a_forest(&mut app);
    until(&mut app, "the stored deck's save", |_| {
        gateway.saves().len() == 2
    });
    gateway.answer(200, r#"{"archived":true}"#);
    until(&mut app, "it landed", |app| {
        !state(app).lobby.builder().dirty()
    });
    assert!(
        state_mut(&mut app).drafts.of(None).is_some(),
        "the new deck's draft is its only copy"
    );
}

/// Discard takes back what had not gone out.
#[test]
fn discarding_takes_back_what_waited() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    until(&mut app, "the first save", |_| gateway.saves().len() == 1);
    add_a_forest(&mut app);
    frames(&mut app, SETTLED);
    state_mut(&mut app).lobby.builder_mut().clear_deck();
    frames(&mut app, 2);

    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert!(state(&app).confirm_leave, "an empty deck cannot save");
    press(&mut app, Press::Build(BuildPress::DiscardAndLeave));
    gateway.answer(200, r#"{"archived":true}"#);
    until(&mut app, "the answer landed", |app| {
        !state(app).deck_save_out()
    });
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 1, "{:?}", gateway.saves());
}

/// A deck deleted while its `POST` is out cannot be called back, so it goes
/// the moment it has an id. Nothing the player deleted turns up on the shelf.
#[test]
fn a_deck_deleted_under_its_post_goes_when_it_lands() {
    let gateway = Gateway::new();
    let mut app = creating_a_deck(&gateway);
    add_a_forest(&mut app);
    frames(&mut app, SETTLED);
    press(&mut app, Press::Build(BuildPress::ToggleHeaderMenu));
    press(&mut app, Press::Build(BuildPress::DeleteBuildDeck));
    press(&mut app, Press::Shared(SharedPress::ConfirmDestructive));
    assert_eq!(state(&app).lobby.screen(), &Screen::Table);

    gateway.answer(200, r#"{"deck_id":"n1"}"#);
    until(&mut app, "the deck was deleted", |_| {
        gateway.asked("DELETE /decks/n1")
    });
    frames(&mut app, SETTLED);
    assert_eq!(gateway.saves().len(), 1, "{:?}", gateway.saves());
}

/// Closing the client inside the wait keeps the edit on the device: its save
/// had not gone out, and nothing will send it now.
#[test]
fn closing_the_client_keeps_what_had_no_time_to_go_out() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    app.world_mut()
        .resource_mut::<Messages<AppExit>>()
        .write(AppExit::Success);
    app.update();
    let kept = state_mut(&mut app).drafts.of(Some("d1")).cloned();
    assert!(
        kept.is_some_and(|draft| draft.main.iter().any(|row| row.contains("Forest"))),
        "nothing else holds it"
    );
}

/// Leaving under a save does not hide its failure: the draft survives the
/// builder, the player hears it, and opening the deck takes the edit back.
#[test]
fn a_save_that_fails_after_leaving_comes_back_with_the_deck() {
    let gateway = Gateway::new();
    let mut app = editing_a_stored_deck(&gateway.base);
    add_a_forest(&mut app);
    press(&mut app, Press::Build(BuildPress::CloseBuilder));
    assert_eq!(state(&app).lobby.screen(), &Screen::Table);
    until(&mut app, "the save went out", |_| {
        !gateway.saves().is_empty()
    });
    let sent = gateway.saves().len();
    for _ in 0..sent {
        gateway.answer(500, r#"{"error":"the database is down"}"#);
    }
    until(&mut app, "the player heard", |app| {
        state(app).lobby.status().contains("the database is down")
    });
    assert!(state_mut(&mut app).drafts.of(Some("d1")).is_some());

    until(&mut app, "the lobby settled", |app| {
        !state(app).lobby.busy()
    });
    open_stored(&mut app, &["4 Llanowar Elves"]);
    let rows = state(&app).lobby.builder().rows(Zone::Main);
    assert!(rows.iter().any(|row| row.contains("Forest")), "{rows:?}");
    assert!(state(&app).lobby.builder().dirty());
}
