//! The game log (#262), driven through `poll_host`: which frames' lines reach
//! the book, and what a frame that only carries lines does to the table.
//!
//! Through the real message loop, because the book's own rules are tested in
//! `baylee_client_core::gamelog` and say nothing about which frames the
//! client hands it. A seat with many lines waiting is sent several frames
//! repeating one view and `seq`, and a loop that read the log only from the
//! views it took would lose every part after the first.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::{Defender, ObjectId};
use baylee_view::{AttackerView, LogEntry, LogEvent, LogTail, Step};
use std::sync::{Arc, Mutex};

/// A host the test feeds frame by frame.
struct Feed(Arc<Mutex<Vec<HostMessage>>>);

impl DuelHost for Feed {
    fn poll(&mut self) -> Vec<HostMessage> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
    fn submit(&mut self, _: PlayerAction) {}
    fn ready(&mut self) {}
    fn seat(&self) -> PlayerId {
        PlayerId::new(0)
    }
}

/// An open duel over a [`Feed`], running the message loop.
struct Table {
    app: App,
    inbox: Arc<Mutex<Vec<HostMessage>>>,
}

impl Table {
    fn new() -> Self {
        let inbox = Arc::new(Mutex::new(Vec::new()));
        let mut app = App::new();
        app.add_plugins(bevy::asset::AssetPlugin::default())
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_asset::<Image>();
        let textures = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            textures::CardTextures::new(&mut images, 1 << 20)
        };
        app.insert_resource(textures)
            .init_resource::<Duel>()
            .init_state::<DuelPhase>()
            .add_message::<DuelReport>()
            .insert_resource(InstalledHost(Box::new(Feed(inbox.clone()))))
            .add_systems(Update, (poll_host, flush_outbox).chain());
        // `poll_host` reads nothing in `Closed`, where a phase starts.
        app.world_mut()
            .resource_mut::<NextState<DuelPhase>>()
            .set(DuelPhase::Opening);
        app.update();
        Self { app, inbox }
    }

    /// The host hands these over, and a frame runs.
    fn hear(&mut self, messages: Vec<HostMessage>) {
        self.inbox.lock().unwrap().extend(messages);
        self.app.update();
    }

    fn duel(&mut self) -> Mut<'_, Duel> {
        self.app.world_mut().resource_mut::<Duel>()
    }
}

/// `count` lines of the log, starting at line `from`: turn `from + i` began.
fn tail(from: u32, count: u32) -> LogTail {
    LogTail {
        from,
        entries: (from..from + count)
            .map(|turn| LogEntry {
                turn,
                repeat: 1,
                event: LogEvent::TurnStarted {
                    active: PlayerId::new(0),
                },
            })
            .collect(),
    }
}

/// A frame: the view at `seq`, and a part of the log.
fn frame(seq: u64, log: Option<LogTail>) -> HostMessage {
    let mut view = ViewBuilder::new(2).build();
    view.seq = seq;
    HostMessage::View(Box::new(view), log)
}

/// Every part of a log too long for one frame reaches the book, though all
/// but the first repeat the view the client already holds.
#[test]
fn a_frame_that_repeats_the_view_still_brings_its_lines() {
    let mut table = Table::new();
    table.hear(vec![
        frame(7, Some(tail(0, 2))),
        frame(7, Some(tail(2, 2))),
        frame(7, Some(tail(4, 1))),
    ]);
    // The last part a frame later, as a host that paced them would send it.
    table.hear(vec![frame(7, Some(tail(5, 1)))]);
    let turns: Vec<u32> = table.duel().log.entries().iter().map(|e| e.turn).collect();
    assert_eq!(
        turns,
        [0, 1, 2, 3, 4, 5],
        "the frames that repeated seq 7 lost their part of the log"
    );
}

/// Lines told again are held once, and a question asked again, whose frame
/// carries an empty tail that says it starts at 0, empties nothing.
#[test]
fn lines_told_again_are_held_once() {
    let mut table = Table::new();
    table.hear(vec![frame(3, Some(tail(0, 3)))]);
    // A snapshot tells the log from its start, and a reconnect does again.
    table.hear(vec![frame(4, Some(tail(0, 4)))]);
    table.hear(vec![frame(4, Some(tail(0, 4)))]);
    // A refused answer is handed back with no log at all, and a reask over
    // the network with an empty tail from 0.
    table.hear(vec![frame(4, None)]);
    table.hear(vec![frame(4, Some(LogTail::default()))]);
    assert_eq!(
        table.duel().log.len(),
        4,
        "a retold log or an empty tail changed what the book holds"
    );
}

/// A frame that repeats the view it came after replays nothing the table
/// shows: no second blow, no second sound, no reveal the player closed
/// opened again. Only its lines are new.
#[test]
fn a_repeated_view_replays_nothing_on_the_table() {
    let knight = token(1, 1, "Knight", 3, 3);
    let attack = vec![AttackerView {
        creature: ObjectId::new(1, 0),
        defending: Defender::Player(PlayerId::new(0)),
        blocked: false,
    }];
    let at = |seq: u64, step: Step, life: i32, shown: bool| {
        let mut builder = ViewBuilder::new(2)
            .with_battlefield(1, vec![knight.clone()])
            .with_combat(attack.clone(), vec![]);
        if shown {
            builder = builder.with_looking_at(vec![token(9, 1, "Shown", 1, 1)]);
        }
        let mut view = builder.build();
        view.seq = seq;
        view.turn = 3;
        view.step = step;
        view.seats[0].life = life;
        view
    };
    let mut table = Table::new();
    table.hear(vec![HostMessage::View(
        Box::new(at(6, Step::DeclareBlockers, 40, false)),
        Some(tail(0, 1)),
    )]);
    let damage = at(7, Step::CombatDamage, 37, true);
    table.hear(vec![HostMessage::View(
        Box::new(damage.clone()),
        Some(tail(1, 1)),
    )]);
    let (strikes, cues) = {
        let mut duel = table.duel();
        assert!(duel.browser.is_open(), "the reveal did not open the sheet");
        duel.browser.close();
        (duel.strikes.len(), duel.cues.pending().to_vec())
    };
    assert_eq!(
        strikes, 1,
        "the blow was never read, so its repeat proves nothing"
    );
    assert!(!cues.is_empty(), "the life lost was never heard");

    table.hear(vec![HostMessage::View(Box::new(damage), Some(tail(2, 1)))]);
    let duel = table.duel();
    assert_eq!(duel.strikes.len(), strikes, "the blow was struck twice");
    assert_eq!(duel.cues.pending(), cues, "the same frame was heard twice");
    assert!(
        !duel.browser.is_open(),
        "a reveal the player had put away opened again"
    );
    assert_eq!(duel.log.len(), 3, "the repeated frame's line was lost");
}

/// A game played in this process tells its log the way a socket does, and
/// the book has it: the path from the session's frames to the book, which
/// the scripted frames above stand in for.
#[test]
fn a_local_game_s_log_reaches_the_book() {
    let mut table = Table::new();
    let host = crate::host::LocalHost::new(
        &crate::host::tests::duel_preset(),
        PlayerId::new(0),
        &["You", "AI"],
    )
    .expect("host");
    table.app.insert_resource(InstalledHost(Box::new(host)));
    table.app.update();
    table.duel().submit(PlayerAction::MulliganKeep);
    // One frame sends the answer, the next reads what came back.
    table.app.update();
    table.app.update();
    let kept = table.duel().log.entries().iter().any(|entry| {
        matches!(
            entry.event,
            LogEvent::Kept { player, .. } if player == PlayerId::new(0)
        )
    });
    assert!(kept, "the seat kept its hand and its log does not say so");
}
