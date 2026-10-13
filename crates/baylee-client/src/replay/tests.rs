//! The replay host and the way `poll_host` plays it (`.claude/b8/REPLAY-DESIGN.md`
//! §13, "Lanes C and D").
//!
//! The game is real: a house-played duel run in this process by a
//! [`LocalHost`], its record projected as a kept record is. The host under
//! test is then driven the way the client drives it, through `poll`,
//! `control` and `ready`, and (for the duel's side of a `Jump`) through
//! `poll_host` in a Bevy app, because a `Duel` literal proves nothing about
//! the fields the wire fills (crate `CLAUDE.md` §Tests).

use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use baylee_client_core::replay::{
    Command, Header, Perspective, Perspectives, Source, Tape, Verify,
};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::PlayerView;
use bevy::prelude::*;

use super::host::{HostedAccess, ReplayHost, ReplaySource};
use super::{Origin, ReplayAsk, ReplayStatus};
use crate::host::{DuelHost, HostMessage, LocalHost};
use crate::{Duel, DuelPhase, DuelReport, InstalledHost};

/// The record file's name the sources carry (the tape header's `game`).
const FILE: &str = "game-0000000000001-0000000000000003.jsonl.gz";

/// How long a test waits for a worker before it calls the replay stuck.
const PATIENCE: Duration = Duration::from_secs(60);

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn names() -> Vec<String> {
    vec!["You".to_string(), "House AI".to_string()]
}

// ------------------------------------------------------------ the game

/// What a played game leaves behind.
struct Played {
    /// The record, JSON Lines, as the local host kept it.
    record: Vec<u8>,
    /// Its projection for seat 0, made here the way the host's worker makes it.
    tape: Tape,
}

/// One game, played once for every test: the house answers for seat 0 until
/// the fourth turn, then seat 0 concedes, so the record is complete and the
/// tape ends in a `GameOver`.
static GAME: LazyLock<Played> = LazyLock::new(|| {
    let record = play_a_game();
    let tape = project(&record);
    Played { record, tape }
});

/// Plays seat 0 with the house's own answers, then concedes.
fn play_a_game() -> Vec<u8> {
    let agent = baylee_ai::HeuristicAgent::new(baylee_core::preset::AIProfile::default());
    let mut host = LocalHost::new(
        &crate::host::tests::duel_preset(),
        me(),
        &["You", "House AI"],
    )
    .expect("the duel starts");
    let mut latest: Option<PlayerView> = None;
    // What the host said since the last call: the newest view, and the
    // question if there was one.
    let mut hear = |messages: Vec<HostMessage>| {
        let mut question = None;
        for message in messages {
            match message {
                HostMessage::View(v, _) => latest = Some(*v),
                HostMessage::Choice(p) => question = Some(*p),
                HostMessage::Failed(why) => panic!("the host refused: {why}"),
                _ => {}
            }
        }
        (latest.clone(), question)
    };
    let (mut now, mut asked) = hear(host.poll());
    for _ in 0..2_000 {
        let Some(question) = asked.take() else {
            break;
        };
        let seen = now.as_ref().expect("a view comes with the first question");
        if seen.turn >= 4 {
            asked = Some(question);
            break;
        }
        host.submit(agent.act(seen, &question));
        (now, asked) = hear(host.poll());
    }
    assert!(asked.is_some(), "the game stopped asking before turn 4");
    host.submit(PlayerAction::Concede);
    let (_, over) = hear(host.poll());
    assert!(
        matches!(over, Some(Pending::GameOver(_))),
        "conceding did not end the game: {over:?}"
    );
    host.local_record()
        .expect("a local host keeps one")
        .to_vec()
}

/// The header a local projection is made under (as `ReplayHost::start`).
fn header() -> Header {
    Header {
        tape: baylee_client_core::replay::TAPE_VERSION,
        view: baylee_view::VIEW_VERSION,
        build: baylee_build::short().to_string(),
        source: Source::Local,
        perspective: baylee_client_core::replay::tape_perspective(Perspective::Seat(me())),
        seat: 0,
        game: FILE.to_string(),
        recorded_by: String::new(),
        own: true,
        hands: Vec::new(),
    }
}

/// The record projected for seat 0.
fn project(record: &[u8]) -> Tape {
    super::local::project(
        record,
        names(),
        header(),
        Perspective::Seat(me()),
        Verify::Stop,
        &AtomicU8::new(0),
        &AtomicBool::new(false),
    )
    .expect("the record plays again")
}

fn record_source() -> ReplaySource {
    ReplaySource::Record {
        bytes: Arc::new(GAME.record.clone()),
        names: names(),
        file: FILE.to_string(),
        by_hand: false,
    }
}

fn tape_source() -> ReplaySource {
    ReplaySource::Tape {
        gzip: Arc::new(GAME.tape.write()),
    }
}

fn host_of(source: ReplaySource) -> ReplayHost {
    ReplayHost::new(
        source,
        Perspective::Seat(me()),
        Verify::Stop,
        Perspectives::LOCAL,
        Origin::default(),
    )
}

/// Polls until the tape is shown, handing back every message of every poll
/// (the poll that takes the tape in is the one that opens the table).
fn open(host: &mut ReplayHost) -> Vec<HostMessage> {
    let until = Instant::now() + PATIENCE;
    let mut heard = Vec::new();
    loop {
        heard.extend(host.poll());
        let status = host.replay().expect("a replay has a status");
        assert!(status.problem.is_none(), "{:?}", status.problem);
        if status.steps > 0 && status.loading.is_none() {
            return heard;
        }
        assert!(Instant::now() < until, "the tape never came: {status:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn ask(host: &mut ReplayHost, command: Command) -> Vec<HostMessage> {
    host.control(ReplayAsk::Transport(command));
    host.poll()
}

/// A short name for what a message is, for comparing orders.
fn kind(message: &HostMessage) -> &'static str {
    match message {
        HostMessage::Static(_) => "static",
        HostMessage::View(..) => "view",
        HostMessage::Choice(_) => "choice",
        HostMessage::Failed(_) => "failed",
        HostMessage::Curtain => "curtain",
        HostMessage::Jump => "jump",
        _ => "other",
    }
}

fn kinds(messages: &[HostMessage]) -> Vec<&'static str> {
    messages.iter().map(kind).collect()
}

/// The `View`s among `messages`: the view and its log tail.
fn views(messages: &[HostMessage]) -> Vec<(&PlayerView, Option<&baylee_view::LogTail>)> {
    messages
        .iter()
        .filter_map(|m| match m {
            HostMessage::View(view, tail) => Some((&**view, tail.as_ref())),
            _ => None,
        })
        .collect()
}

/// A step with log lines before it and fewer than the end holds, so that
/// "the log up to here" is told apart from the whole log, and from none.
fn middle() -> usize {
    let tape = &GAME.tape;
    let last = tape.frames.len() - 1;
    // Not the step before the last: a step on from it would end the game.
    (2..last - 1)
        .rev()
        .find(|i| {
            let held = tape.log_to(*i).entries.len();
            held > 0 && held < tape.log_to(last).entries.len()
        })
        .expect("the game has a step between its lines")
}

/// The fixture is what the tests below assume, so a game that stopped
/// producing one does not turn them hollow.
#[test]
fn the_game_the_tests_replay_has_turns_a_log_and_an_end() {
    let tape = &GAME.tape;
    assert!(tape.frames.len() > 10, "{} steps", tape.frames.len());
    assert!(tape.over().is_some(), "the tape does not end in a GameOver");
    let last = tape.frames.len() - 1;
    assert!(tape.frames[last].place.turn >= 4, "{:?}", tape.frames[last]);
    assert!(tape.log_to(0).entries.len() < tape.log_to(middle()).entries.len());
    assert!(tape.log_to(middle()).entries.len() < tape.log_to(last).entries.len());
}

// ------------------------------------------------------------ 1. the host

/// A replay asks nothing of anyone: whatever the table answers is dropped
/// without a trace, in the status and in the next poll alike.
#[test]
fn submit_changes_nothing() {
    for source in [record_source(), tape_source()] {
        let mut host = host_of(source);
        open(&mut host);
        let before: ReplayStatus = host.replay().cloned().expect("a status");
        for action in [
            PlayerAction::PassPriority,
            PlayerAction::MulliganKeep,
            PlayerAction::Concede,
        ] {
            host.submit(action);
        }
        assert!(
            host.poll().is_empty(),
            "an answer to a replay was heard as something"
        );
        assert_eq!(host.replay(), Some(&before));
    }
}

#[test]
fn a_replay_watches() {
    let mut host = host_of(tape_source());
    assert!(host.watching(), "before the tape is in");
    open(&mut host);
    assert!(host.watching(), "after");
    assert_eq!(host.seat(), me());
}

/// The duel's one curtain: the host never opens it by itself, and `ready`
/// queues it once, for the next poll only.
#[test]
fn ready_queues_exactly_one_curtain() {
    let mut host = host_of(record_source());
    let opening = open(&mut host);
    assert!(
        !kinds(&opening).contains(&"curtain"),
        "the tape's own curtain line was sent: {:?}",
        kinds(&opening)
    );
    host.ready();
    let after = host.poll();
    assert_eq!(kinds(&after), ["curtain"]);
    assert!(host.poll().is_empty(), "the curtain came down and up again");
    // Moving the replay about never raises it a second time.
    for command in [Command::End, Command::Start, Command::Step(1)] {
        assert!(
            !kinds(&ask(&mut host, command)).contains(&"curtain"),
            "{command:?} raised the curtain again"
        );
    }
}

/// A report about a replay offers the record only where this client has it.
#[test]
fn only_a_record_source_has_a_local_record() {
    let mut record = host_of(record_source());
    assert_eq!(
        record.local_record(),
        Some(GAME.record.as_slice()),
        "the record the replay plays is the one F8 attaches"
    );
    open(&mut record);
    assert_eq!(record.local_record(), Some(GAME.record.as_slice()));

    let mut tape = host_of(tape_source());
    assert!(tape.local_record().is_none());
    open(&mut tape);
    assert!(tape.local_record().is_none());
}

/// Only a hosted game has a gateway id (the report form names it). The
/// gateway here answers nothing, and the replay says why instead of waiting.
#[test]
fn only_a_hosted_replay_has_a_game_id() {
    let mut record = host_of(record_source());
    let mut tape = host_of(tape_source());
    assert_eq!(record.game_id(), None);
    assert_eq!(tape.game_id(), None);
    open(&mut record);
    open(&mut tape);
    assert_eq!(record.game_id(), None);
    assert_eq!(tape.game_id(), None);

    let mut hosted = host_of(ReplaySource::Hosted(HostedAccess {
        gateway: "http://127.0.0.1:9".to_string(),
        name: "Nowhere".to_string(),
        token: "not-a-session".to_string(),
        game: "the-game-of-the-replay-tests".to_string(),
    }));
    assert_eq!(hosted.game_id(), Some("the-game-of-the-replay-tests"));
    let loading = hosted.replay().and_then(|s| s.loading.clone());
    assert_eq!(
        loading,
        Some(super::Loading::Gateway {
            name: "Nowhere".to_string()
        }),
        "a hosted replay waits on its gateway, by name"
    );
    let until = Instant::now() + PATIENCE;
    while hosted.replay().is_some_and(|s| s.problem.is_none()) {
        assert!(Instant::now() < until, "the failed fetch was never told");
        std::thread::sleep(Duration::from_millis(10));
        hosted.poll();
    }
    let status = hosted.replay().expect("a status");
    assert!(status.loading.is_none(), "{status:?}");
    assert_eq!(status.steps, 0);
    assert_eq!(hosted.game_id(), Some("the-game-of-the-replay-tests"));
    assert!(hosted.poll().is_empty(), "a failed fetch drew a table");
}

// ------------------------------------------------------------ 2. the frames

/// The opening is a socket's: the payload, then the view, the log (if it
/// has lines yet) told from line 0, and no jump.
#[test]
fn the_opening_poll_is_a_static_then_a_view_with_the_log_from_the_start() {
    for source in [record_source(), tape_source()] {
        let mut host = host_of(source);
        let opening = open(&mut host);
        assert_eq!(kinds(&opening), ["static", "view"], "{:?}", kinds(&opening));
        let (view, tail) = views(&opening)[0];
        assert!(view.seq > 0);
        assert_eq!(view.turn, GAME.tape.frames[0].place.turn);
        let told = GAME.tape.log_to(0);
        match tail {
            None => assert!(told.entries.is_empty(), "the opening's lines were not told"),
            Some(tail) => assert_eq!(*tail, told),
        }
        let status = host.replay().expect("a status");
        assert_eq!((status.step, status.steps), (0, GAME.tape.frames.len()));
    }
}

/// One step on is a live frame (a view, the lines since the last, no jump),
/// and every other move is a jump followed by the table at its step with the
/// log from its first line. The `seq` only ever grows, jumps included.
#[test]
fn a_step_on_is_a_view_and_every_other_move_a_jump() {
    let mid = middle();
    for source in [record_source(), tape_source()] {
        let mut host = host_of(source);
        let mut every_seq = Vec::new();
        let mut note = |messages: &[HostMessage]| {
            every_seq.extend(views(messages).iter().map(|(v, _)| v.seq));
        };
        let opening = open(&mut host);
        note(&opening);

        // One step on.
        let step = ask(&mut host, Command::Step(1));
        note(&step);
        assert!(!kinds(&step).contains(&"jump"), "{:?}", kinds(&step));
        assert_eq!(kinds(&step).iter().filter(|k| **k == "view").count(), 1);
        let (view, tail) = views(&step)[0];
        assert_eq!(view.turn, GAME.tape.frames[1].place.turn);
        assert_eq!(
            tail.cloned(),
            GAME.tape.tail_at(1),
            "a live frame carries the lines since the last"
        );

        // A jump forward, to somewhere with lines before it.
        let seek = ask(&mut host, Command::Seek(mid));
        note(&seek);
        assert_eq!(kinds(&seek)[0], "jump", "{:?}", kinds(&seek));
        assert_eq!(*kinds(&seek).last().expect("a frame"), "view");

        // One step back from there: a jump, the payload, the table at
        // `mid - 1` told from line 0.
        let back = ask(&mut host, Command::Step(-1));
        note(&back);
        assert_eq!(
            kinds(&back),
            ["jump", "static", "view"],
            "{:?}",
            kinds(&back)
        );
        let (view, tail) = views(&back)[0];
        assert_eq!(view.turn, GAME.tape.frames[mid - 1].place.turn);
        let told = GAME.tape.log_to(mid - 1);
        assert_eq!(told.from, 0);
        match tail {
            None => assert!(told.entries.is_empty()),
            Some(tail) => {
                assert_eq!(tail.from, 0, "a jump tells the log from its first line");
                assert_eq!(tail.entries, told.entries);
            }
        }
        assert_eq!(host.replay().expect("a status").step, mid - 1);

        // To the end: the table, then the question that is the game's end.
        let end = ask(&mut host, Command::End);
        note(&end);
        let order = kinds(&end);
        assert_eq!(order[0], "jump", "{order:?}");
        assert_eq!(order[order.len() - 2..], ["view", "choice"], "{order:?}");
        let (_, tail) = views(&end)[0];
        let whole = GAME.tape.log_to(GAME.tape.frames.len() - 1);
        assert_eq!(tail.expect("the whole log").entries, whole.entries);
        assert!(
            end.iter().any(|m| matches!(
                m,
                HostMessage::Choice(p) if matches!(**p, Pending::GameOver(_))
            )),
            "the end is a GameOver"
        );

        // Back to the start; and a step back from the start stays put.
        let start = ask(&mut host, Command::Start);
        note(&start);
        assert_eq!(kinds(&start)[0], "jump");
        assert!(ask(&mut host, Command::Step(-1)).is_empty());
        assert!(ask(&mut host, Command::Start).is_empty());

        assert!(
            every_seq.len() >= 6 && every_seq.windows(2).all(|pair| pair[0] < pair[1]),
            "a view was sent with a seq no greater than the last: {every_seq:?}"
        );
    }
}

/// A step onto the last frame is a live frame that also ends the game.
#[test]
fn the_last_step_on_ends_the_game() {
    let mut host = host_of(tape_source());
    open(&mut host);
    let last = GAME.tape.frames.len() - 1;
    ask(&mut host, Command::Seek(last - 1));
    let step = ask(&mut host, Command::Step(1));
    assert_eq!(kinds(&step), ["view", "choice"], "{:?}", kinds(&step));
    assert!(!host.replay().expect("a status").playing);
}

// ------------------------------------------------------------ 3. poll_host

/// An open duel over a replay host, with the systems under test.
struct Table {
    app: App,
}

impl Table {
    fn new(source: ReplaySource) -> Self {
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_asset::<Image>();
        let textures = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            crate::textures::CardTextures::new(&mut images, 1 << 20)
        };
        app.insert_resource(textures)
            .init_resource::<Duel>()
            .init_state::<DuelPhase>()
            .add_message::<DuelReport>()
            .insert_resource(InstalledHost(Box::new(host_of(source))))
            .add_systems(Update, (crate::poll_host, crate::flush_outbox).chain());
        // `poll_host` reads nothing in `Closed`, where a phase starts.
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Opening);
        app.update();
        let mut table = Self { app };
        table.until(|t| t.duel().view.is_some() && t.duel().curtain_up);
        table
    }

    fn duel(&self) -> &Duel {
        self.app.world().resource::<Duel>()
    }

    fn duel_mut(&mut self) -> Mut<'_, Duel> {
        self.app.world_mut().resource_mut::<Duel>()
    }

    fn phase(&self) -> DuelPhase {
        *self.app.world().resource::<State<DuelPhase>>().get()
    }

    /// Frames, with time for the worker between them, until `done`.
    fn until(&mut self, done: impl Fn(&Self) -> bool) {
        let until = Instant::now() + PATIENCE;
        while !done(self) {
            assert!(Instant::now() < until, "the table never stood");
            self.app.update();
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Asks the replay and runs the frames that carry it out: one for the
    /// host's frames, one for the phase it moves.
    fn ask(&mut self, command: Command) {
        self.app
            .world_mut()
            .resource_mut::<InstalledHost>()
            .0
            .control(ReplayAsk::Transport(command));
        self.app.update();
        self.app.update();
    }

    fn step(&self) -> usize {
        self.duel().replay.as_ref().expect("a replay").step
    }
}

/// The duel's side of a jump, through the real message loop: the log book
/// holds the lines up to the step and nothing from later, the end sheet's
/// state is taken down, and the frames that follow are accepted.
#[test]
fn a_jump_back_leaves_the_log_up_to_its_step_and_takes_the_end_down() {
    let tape = &GAME.tape;
    let last = tape.frames.len() - 1;
    let mid = middle();
    let lines = |i: usize| tape.log_to(i).entries.len();
    let mut table = Table::new(record_source());

    // The opening: step 0, nothing wrong, the replay's place copied over.
    assert_eq!(table.step(), 0);
    assert_eq!(table.duel().log.len(), lines(0));
    assert!(table.duel().watching);
    assert_eq!(
        table.duel().replay.as_ref().map(|s| s.steps),
        Some(last + 1)
    );
    assert!(table.duel().ending().is_none());

    // The end: the whole log, the question that is the game's end, and the
    // phase that goes with it.
    table.ask(Command::End);
    assert_eq!(table.step(), last);
    assert_eq!(table.duel().log.len(), lines(last));
    assert!(table.duel().ending().is_some(), "the end was not shown");
    assert_eq!(table.phase(), DuelPhase::Finished);

    // Back to the middle of the game.
    table.duel_mut().cues.note_refusal();
    table.ask(Command::Seek(mid));
    assert_eq!(table.step(), mid);
    assert_eq!(
        table.duel().log.entries(),
        tape.log_to(mid).entries.as_slice(),
        "the book holds the lines to step {mid}, no more, no fewer"
    );
    assert_eq!(table.duel().log.gaps(), 0);
    assert!(table.duel().ending().is_none(), "the end sheet stayed up");
    assert_eq!(table.phase(), DuelPhase::Playing, "the game is not over");
    assert!(
        table.duel().cues.pending().is_empty(),
        "a cue sounded across a jump"
    );
    assert!(table.duel().last_error.is_none());
    let seen = table.duel().view.as_ref().expect("the board stands");
    assert_eq!(seen.turn, tape.frames[mid].place.turn);
    let seq = seen.seq;

    // The next step on is accepted: its view is the duel's, its lines are
    // appended.
    table.ask(Command::Step(1));
    assert_eq!(table.step(), mid + 1);
    let seen = table.duel().view.as_ref().expect("the board stands");
    assert!(seen.seq > seq, "the step after a jump was dropped as older");
    assert_eq!(seen.turn, tape.frames[mid + 1].place.turn);
    assert_eq!(table.duel().log.len(), lines(mid + 1));
    assert_eq!(table.phase(), DuelPhase::Playing);

    // And back to the very start: the book empties down to the opening's.
    table.ask(Command::Start);
    assert_eq!(table.step(), 0);
    assert_eq!(table.duel().log.len(), lines(0));
    assert!(table.duel().last_error.is_none());
}

/// Stepping onto the end finishes the table as a live game's last frame
/// does, and a jump from it reopens it.
#[test]
fn the_last_step_finishes_the_table_and_a_jump_from_it_reopens_it() {
    let tape = &GAME.tape;
    let last = tape.frames.len() - 1;
    let mut table = Table::new(tape_source());
    table.ask(Command::Seek(last - 1));
    assert!(table.duel().ending().is_none());
    assert_eq!(table.phase(), DuelPhase::Playing);
    table.ask(Command::Step(1));
    assert!(table.duel().ending().is_some());
    assert_eq!(table.phase(), DuelPhase::Finished);
    assert_eq!(table.duel().log.len(), tape.log_to(last).entries.len());
    // One step back is a jump; the end is gone with it.
    table.ask(Command::Step(-1));
    assert_eq!(table.step(), last - 1);
    assert!(table.duel().ending().is_none());
    assert_eq!(table.phase(), DuelPhase::Playing);
    assert_eq!(table.duel().log.len(), tape.log_to(last - 1).entries.len());
}

// ------------------------------------------------------------ 4. Duel::rewind

/// A jump forgets what the frames told and keeps what the player chose.
#[test]
fn rewind_forgets_the_moment_and_keeps_the_player_s_table() {
    let mut table = Table::new(record_source());
    table.ask(Command::End);
    // What the player chose (set on purpose, as the settings and the menus do).
    {
        let mut duel = table.duel_mut();
        duel.arrangement = baylee_client_core::tableview::Arrangement::Turntable;
        duel.log_open = true;
        duel.game_menu = true;
        duel.cues.note_refusal();
    }
    // Everything the wire filled is there to be forgotten.
    {
        let duel = table.duel();
        assert!(duel.view.is_some() && duel.board.is_some());
        assert!(duel.interaction.is_some(), "the GameOver question");
        assert!(!duel.log.is_empty());
        assert!(!duel.cues.pending().is_empty(), "the cue was never queued");
        assert!(duel.statics.is_some());
        assert!(duel.watching);
        assert!(duel.curtain_up && duel.ready_sent);
    }
    let kept = {
        let duel = table.duel();
        (
            duel.statics.clone(),
            duel.replay.clone(),
            duel.arrangement,
            duel.own_deck.clone(),
        )
    };

    table.duel_mut().rewind();

    let duel = table.duel();
    // Forgotten.
    assert!(duel.interaction.is_none());
    assert!(duel.ending().is_none());
    assert!(duel.view.is_none());
    assert!(duel.board.is_none());
    assert!(duel.log.is_empty());
    assert_eq!(duel.log.gaps(), 0);
    assert!(duel.cues.pending().is_empty());
    assert!(duel.outbox().is_empty());
    // Kept.
    assert_eq!(duel.statics, kept.0);
    assert_eq!(duel.replay, kept.1);
    assert_eq!(duel.arrangement, kept.2);
    assert_eq!(duel.own_deck, kept.3);
    assert!(duel.log_open, "the log panel was closed");
    assert!(duel.game_menu, "the game menu was closed");
    assert!(duel.watching, "a replay watches still");
    assert!(
        duel.curtain_up && duel.ready_sent,
        "the table is still open: the host is not asked to say it is ready again"
    );
}

// ------------------------------------------------------------ 5. keys

mod keys {
    use baylee_client_core::prefs::{Action, Keymap};
    use baylee_client_core::replay::Command;
    use bevy::input::ButtonInput;
    use bevy::prelude::KeyCode;

    use crate::keys::{Fired, REPLAY_BORROWS, replay_asks};
    use crate::replay::ReplayAsk;

    fn pressed(down: &[KeyCode]) -> Fired {
        let mut keys = ButtonInput::default();
        for key in down {
            keys.press(*key);
        }
        Fired::of(&keys, &Keymap::standard())
    }

    fn asks(down: &[KeyCode]) -> Vec<ReplayAsk> {
        replay_asks(&pressed(down), false, false)
    }

    fn transport(command: Command) -> Vec<ReplayAsk> {
        vec![ReplayAsk::Transport(command)]
    }

    const SHIFT: KeyCode = KeyCode::ShiftLeft;

    #[test]
    fn each_key_asks_its_transport_command() {
        for (down, command) in [
            (vec![KeyCode::Space], Command::Toggle),
            (vec![KeyCode::ArrowLeft], Command::Step(-1)),
            (vec![KeyCode::ArrowRight], Command::Step(1)),
            (vec![KeyCode::Tab], Command::NextPhase),
            (vec![SHIFT, KeyCode::Tab], Command::Turn(1)),
            (vec![SHIFT, KeyCode::ArrowLeft], Command::Turn(-1)),
            (vec![SHIFT, KeyCode::ArrowRight], Command::Turn(1)),
            (vec![KeyCode::Home], Command::Start),
            (vec![KeyCode::End], Command::End),
            (vec![KeyCode::Minus], Command::Slower),
            (vec![KeyCode::Equal], Command::Faster),
        ] {
            assert_eq!(asks(&down), transport(command), "{down:?}");
        }
        assert_eq!(
            asks(&[KeyCode::KeyV]),
            vec![ReplayAsk::NextPerspective],
            "V"
        );
    }

    /// The arrows are also the live table's number keys, and a shifted arrow
    /// is a different chord: ⇧← is a turn back and not also a step back.
    #[test]
    fn a_shifted_arrow_is_a_turn_and_not_also_a_step() {
        let fired = pressed(&[SHIFT, KeyCode::ArrowLeft]);
        assert!(fired.has(Action::ReplayTurnBack));
        assert!(!fired.has(Action::NumberDown));
        assert_eq!(
            replay_asks(&fired, false, false),
            transport(Command::Turn(-1))
        );
    }

    #[test]
    fn a_key_nobody_bound_asks_nothing() {
        assert!(asks(&[KeyCode::KeyQ]).is_empty());
        assert!(asks(&[]).is_empty());
    }

    /// Over the end sheet Space is the sheet's lead answer, not play/pause.
    #[test]
    fn space_over_the_end_sheet_is_the_sheet_s() {
        let space = pressed(&[KeyCode::Space]);
        assert_eq!(
            replay_asks(&space, false, false),
            transport(Command::Toggle)
        );
        assert!(replay_asks(&space, true, false).is_empty());
        // The other keys still move a finished replay.
        let left = pressed(&[KeyCode::ArrowLeft]);
        assert_eq!(
            replay_asks(&left, true, false),
            transport(Command::Step(-1))
        );
    }

    /// Esc stops waiting for a tape, and is nothing to a tape on screen.
    #[test]
    fn escape_cancels_only_while_loading() {
        let escape = pressed(&[KeyCode::Escape]);
        assert_eq!(replay_asks(&escape, false, true), vec![ReplayAsk::Cancel]);
        assert!(replay_asks(&escape, false, false).is_empty());
        assert!(replay_asks(&escape, true, false).is_empty());
    }

    /// What the live table is handed in a replay: everything but the five
    /// actions the replay borrows; and at a live table, everything.
    #[test]
    fn a_replay_borrows_exactly_its_five_actions_from_the_live_table() {
        let every = Fired::of_actions(&Action::ALL);
        let live = every.in_replay(false);
        let replay = every.in_replay(true);
        for action in Action::ALL {
            assert!(live.has(action), "{action:?} was taken from a live table");
            assert_eq!(
                replay.has(action),
                !REPLAY_BORROWS.contains(&action),
                "{action:?} in a replay"
            );
        }
        assert_eq!(REPLAY_BORROWS.len(), 5);
        // A replay's own actions are never borrowed: they have no live use.
        for action in [
            Action::ReplayTurnBack,
            Action::ReplayTurnForward,
            Action::ReplayStart,
            Action::ReplayEnd,
            Action::ReplaySlower,
            Action::ReplayFaster,
            Action::ReplayPerspective,
        ] {
            assert!(replay.has(action), "{action:?}");
        }
    }

    /// The borrowed actions are exactly those `replay_asks` answers with a
    /// live meaning (Space, Tab, ⇧Tab, the arrows): none is borrowed that the
    /// replay does not use, none used that the live table still hears.
    #[test]
    fn every_borrowed_action_is_one_the_replay_answers() {
        for action in REPLAY_BORROWS {
            let asked = replay_asks(&Fired::of_actions(&[action]), false, false);
            assert!(!asked.is_empty(), "{action:?} is borrowed and unused");
        }
        for action in Action::ALL {
            if REPLAY_BORROWS.contains(&action) {
                continue;
            }
            let asked = replay_asks(&Fired::of_actions(&[action]), false, false);
            let is_replays = matches!(
                action,
                Action::ReplayTurnBack
                    | Action::ReplayTurnForward
                    | Action::ReplayStart
                    | Action::ReplayEnd
                    | Action::ReplaySlower
                    | Action::ReplayFaster
                    | Action::ReplayPerspective
            );
            assert_eq!(
                !asked.is_empty(),
                is_replays,
                "{action:?}: the replay answers a live action it does not borrow"
            );
        }
    }
}
