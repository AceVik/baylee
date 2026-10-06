use super::*;

// ---- the game log (#262)

/// Every state frame `routed` sends `seat`, in order: its `seq`, its view
/// as sent, and its log tail when it carries one.
fn state_frames_to(
    routed: &[(PlayerId, Envelope)],
    seat: PlayerId,
) -> Vec<(u64, Vec<u8>, Option<LogTail>)> {
    routed
        .iter()
        .filter(|(to, _)| *to == seat)
        .filter_map(|(_, env)| match &env.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => Some((
                delta.seq,
                delta.view_json.clone(),
                (!delta.log_json.is_empty())
                    .then(|| serde_json::from_slice(&delta.log_json).expect("the log decodes")),
            )),
            _ => None,
        })
        .collect()
}

/// The tails in `frames`, checked to be what a client appending by
/// `from` needs: the first starts at `held`, each starts where the one
/// before ended, and none is longer than a frame may carry. Returns every
/// line they carry, in order.
fn contiguous_from(held: usize, frames: &[(u64, Vec<u8>, Option<LogTail>)]) -> Vec<LogEntry> {
    let mut next = held;
    let mut lines = Vec::new();
    for tail in frames.iter().filter_map(|(_, _, tail)| tail.as_ref()) {
        assert_eq!(tail.from as usize, next, "a gap or an overlap in one pump");
        assert!(
            !tail.entries.is_empty(),
            "a tail that says nothing is sent as none"
        );
        assert!(
            tail.entries.len() <= LOG_TAIL_CAP,
            "{} lines in one frame",
            tail.entries.len()
        );
        next += tail.entries.len();
        lines.extend(tail.entries.iter().cloned());
    }
    lines
}

/// Lines enough that no one frame may carry them, each different from the
/// last so that none folds into another.
fn overflow(session: &mut Session, player: PlayerId) -> usize {
    let lines = LOG_TAIL_CAP * 2 + 3;
    for result in 0..lines {
        session.log.note(LogEvent::DiceRolled {
            player,
            sides: 1_000,
            result: u32::try_from(result).expect("a small number"),
        });
    }
    lines
}

/// More lines than one frame may carry, pending at once mid-game, arrive
/// over several frames in one pump, every line once and in order. Every
/// frame is a whole view, the same one at the same `seq`, so a client
/// that drops all but the first as not newer still reads every line; and
/// the pump after sends none of them again.
#[test]
fn an_overflowing_log_arrives_once_and_in_order_mid_game() {
    let (mut session, human) = started_session();
    let _ = session.pump();
    let held = session.told[human.get() as usize];
    assert_eq!(held, session.log.len(), "the human holds every line so far");
    let added = overflow(&mut session, human);

    let routed = session.pump();
    let frames = state_frames_to(&routed, human);
    assert_eq!(frames.len(), added.div_ceil(LOG_TAIL_CAP), "{frames:?}");
    assert!(
        frames
            .iter()
            .all(|(seq, view, _)| (*seq, view) == (frames[0].0, &frames[0].1)),
        "every frame repeats the one view"
    );
    let lines = contiguous_from(held, &frames);
    assert_eq!(held + lines.len(), session.log.len(), "every line arrived");
    assert_eq!(lines, session.log.told(human, held, session.log.len()));

    let again = session.pump();
    assert!(
        state_frames_to(&again, human)
            .iter()
            .all(|(_, _, tail)| tail.is_none()),
        "a line is sent once"
    );
}

/// The same at the end of the game, where no later view could carry what
/// is left: the frames that tell the seat the game is over carry every
/// line, down to the last.
#[test]
fn an_overflowing_log_arrives_whole_at_game_over() {
    let (mut session, human) = started_session();
    let _ = session.pump();
    let held = session.told[human.get() as usize];
    overflow(&mut session, human);

    let routed = session
        .act(human, PlayerAction::Concede)
        .expect("a player may always concede");
    assert!(matches!(session.pending(), Pending::GameOver(_)));
    let lines = contiguous_from(held, &state_frames_to(&routed, human));
    assert_eq!(held + lines.len(), session.log.len(), "every line arrived");
    assert_eq!(lines, session.log.told(human, held, session.log.len()));
    assert!(
        matches!(
            lines.last().map(|line| &line.event),
            Some(LogEvent::GameOver { winners }) if winners.contains(PlayerId::new(1))
        ),
        "the last line is the end: {:?}",
        lines.last()
    );
    let after = session.pump();
    assert!(
        state_frames_to(&after, human)
            .iter()
            .all(|(_, _, tail)| tail.is_none()),
        "and nothing is left to send"
    );
}

/// An AI seat's agent is handed a view and a question, never a log, and
/// its chair is never sent a frame. Pinned here rather than left to the
/// view not having a log field: the one door a log leaves by refuses
/// every seat nobody answers over a socket, a chair the house holds for
/// an absent player included.
#[test]
fn a_seat_the_house_answers_is_never_told_the_log() {
    let (mut session, human) = started_session();
    let ai = PlayerId::new(1);
    for _ in 0..40 {
        let Some(action) = session.timeout_action(human) else {
            break;
        };
        let routed = session.act(human, action).expect("a legal answer");
        assert!(
            routed.iter().all(|(to, _)| *to != ai),
            "a frame was sent to the AI chair"
        );
    }
    assert!(session.log.len() > 10, "the game was logged");
    assert_eq!(session.told[1], 0, "no line was ever counted as sent to it");
    assert_eq!(
        session.log_tail(ai),
        LogTail::default(),
        "asked outright, it tells nothing"
    );
    assert_eq!(session.log_sent(ai), LogTail::default());

    let mut session = Session::new(&two_humans()).expect("session builds");
    let _ = session.pump();
    assert!(session.stand_in(ai));
    assert_eq!(
        session.log_tail(ai),
        LogTail::default(),
        "nor to a chair the house holds"
    );
}

/// A socket that attaches again holds none of the log, so its first view
/// carries all of it.
#[test]
fn a_socket_that_attaches_again_is_told_the_log_from_the_first_line() {
    let (mut session, human) = started_session();
    let _ = session.pump();
    let lines = session.log.len();
    assert!(lines > 0);

    session.retell_log(human);
    let frames = state_frames_to(&session.pump(), human);
    let told = contiguous_from(0, &frames);
    assert_eq!(told, session.log.told(human, 0, lines));
}

/// A rebuild after lost frames carries every line the seat was sent, and
/// marks nothing; a refused answer is handed its question back with no
/// log at all.
#[test]
fn a_snapshot_carries_the_lines_sent_and_a_reask_none() {
    let (mut session, human) = started_session();
    let _ = session.pump();
    let sent = session.told[human.get() as usize];
    session.log.note(LogEvent::Shuffled { player: human });

    let snapshot = session.snapshot(human);
    let rebuilt = contiguous_from(
        0,
        &state_frames_to(
            &snapshot
                .iter()
                .map(|env| (human, env.clone()))
                .collect::<Vec<_>>(),
            human,
        ),
    );
    assert_eq!(
        rebuilt,
        session.log.told(human, 0, sent),
        "the lines sent, and not the one still to go"
    );
    assert_eq!(
        session.told[human.get() as usize],
        sent,
        "a snapshot marks nothing sent"
    );

    let reask: Vec<_> = session
        .reask(human)
        .into_iter()
        .map(|env| (human, env))
        .collect();
    let frames = state_frames_to(&reask, human);
    assert_eq!(frames.len(), 1);
    assert!(frames[0].2.is_none(), "a refusal lost no line");
    assert_eq!(asked(&reask).len(), 1, "and the question is handed back");
}

/// The opening mulligans are logged by their counts, which are public,
/// and nothing names a card before the first turn.
#[test]
fn the_opening_mulligans_are_logged_as_counts() {
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
    let mut session = Session::new(&two_humans()).expect("session builds");
    let _ = session.pump();
    // Two, so that at least one is not free and the keep puts a card
    // on the bottom.
    for _ in 0..2 {
        session
            .act(zero, PlayerAction::MulliganTake)
            .expect("a mulligan");
    }
    session
        .act(zero, PlayerAction::MulliganKeep)
        .expect("a keep");
    let Some(Pending::MulliganBottom { count, .. }) = session.engine.pending_for(zero) else {
        panic!("the keep puts a card on the bottom")
    };
    let bottom = usize::from(*count);
    let hand = session.state().zones.list(ZoneLocation::Hand(zero))[..bottom].to_vec();
    session
        .act(zero, PlayerAction::ChooseObjects { objects: hand })
        .expect("the bottom");
    session
        .act(one, PlayerAction::MulliganKeep)
        .expect("a keep");

    let lines: Vec<LogEvent> = session
        .log
        .told(one, 0, session.log.len())
        .into_iter()
        .map(|line| line.event)
        .collect();
    let start = lines
        .iter()
        .position(|event| matches!(event, LogEvent::TurnStarted { .. }))
        .expect("the first turn began");
    assert_eq!(
        lines[..start],
        [
            LogEvent::Mulliganed { player: zero },
            LogEvent::Mulliganed { player: zero },
            LogEvent::Kept {
                player: zero,
                cards: u8::try_from(7 - bottom).expect("a hand")
            },
            LogEvent::Kept {
                player: one,
                cards: 7
            },
        ]
    );
}

/// A decision clock's answer is logged as what the clock did.
#[test]
fn a_clock_answer_is_logged_as_what_the_clock_did() {
    let mut session = Session::new(&test_preset()).expect("session builds");
    let _ = session.pump();
    let me = PlayerId::new(0);
    clock_answers(&mut session, me)
        .expect("seat 0 owes its mulligan")
        .expect("the clock keeps");
    until_asked(&mut session, me);
    assert!(matches!(session.pending(), Pending::Priority { .. }));
    clock_answers(&mut session, me)
        .expect("seat 0 has priority")
        .expect("the clock passes");
    let timed_out: Vec<LogEvent> = session
        .log
        .told(me, 0, session.log.len())
        .into_iter()
        .map(|line| line.event)
        .filter(|event| matches!(event, LogEvent::TimedOut { .. }))
        .collect();
    assert_eq!(
        timed_out,
        [
            LogEvent::TimedOut {
                player: me,
                answer: ClockAnswer::Kept
            },
            LogEvent::TimedOut {
                player: me,
                answer: ClockAnswer::Passed
            },
        ]
    );
}

/// Every answer that does nothing is logged by its name, and anything
/// else the clock answered is the house's choice.
#[test]
fn every_answer_that_does_nothing_is_logged_by_name() {
    let me = PlayerId::new(0);
    let questions = [
        (
            Pending::Priority {
                player: me,
                legal: Box::default(),
            },
            ClockAnswer::Passed,
        ),
        (
            Pending::Mulligan {
                player: me,
                taken: 0,
                next_is_free: false,
                can_take: true,
            },
            ClockAnswer::Kept,
        ),
        (
            Pending::ChooseAttackers {
                player: me,
                attackers: Vec::new(),
                defenders: Vec::new(),
                required: Vec::new(),
                limits: Vec::new(),
            },
            ClockAnswer::NoAttackers,
        ),
        (
            Pending::ChooseBlockers {
                demands: Vec::new(),
                player: me,
                attacker: PlayerId::new(1),
                blockers: Vec::new(),
                capacity: Vec::new(),
                obeying: Vec::new(),
                bounds: Vec::new(),
            },
            ClockAnswer::NoBlockers,
        ),
        (
            Pending::YesNo {
                player: me,
                prompt: baylee_engine::choice::YesNoPrompt::Kicker,
                source: None,
            },
            ClockAnswer::Declined,
        ),
    ];
    for (pending, named) in questions {
        let quiet = baylee_engine::choice::timeout_answer(&pending).expect("a quiet answer");
        assert_eq!(clock_answer(Some(&pending), &quiet), named, "{pending:?}");
    }
    let priority = Pending::Priority {
        player: me,
        legal: Box::default(),
    };
    assert_eq!(
        clock_answer(Some(&priority), &PlayerAction::Concede),
        ClockAnswer::ChosenForThem
    );
}

/// A chair the house holds for an absent player is logged, and so is the
/// player's return.
#[test]
fn a_stand_in_and_the_return_are_logged() {
    let one = PlayerId::new(1);
    let mut session = Session::new(&two_humans()).expect("session builds");
    assert!(session.stand_in(one));
    assert!(session.hand_back(one));
    let lines: Vec<LogEvent> = session
        .log
        .told(one, 0, session.log.len())
        .into_iter()
        .map(|line| line.event)
        .collect();
    assert_eq!(
        lines,
        [
            LogEvent::StandIn { player: one },
            LogEvent::Returned { player: one }
        ]
    );
}

/// A printing a seat meets only in its log is earned the same way as one
/// met in its view: the entry arrives before the frame that points at it.
#[test]
fn a_printing_first_named_by_the_log_arrives_before_the_line() {
    let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
    let mut preset = split_preset();
    preset.seats[1].starting_battlefield = vec![];
    preset.seats[1].capabilities.dev_commands = true;
    let mut session = Session::new(&preset).expect("session");
    let _ = session.pump();
    assert!(session.game_static(zero).print(PrintRef::new(1)).is_none());

    let forest = session.state().zones.list(ZoneLocation::Library(one))[0];
    let state = session.engine.dev_state_mut(one).expect("dev commands");
    state
        .journal
        .record(baylee_engine::event::GameEvent::Revealed {
            player: one,
            cards: vec![forest],
        });
    session.log.consume(session.engine.state());
    let routed = session.pump();

    let to_zero: Vec<&Envelope> = routed
        .iter()
        .filter(|(seat, _)| *seat == zero)
        .map(|(_, env)| env)
        .collect();
    let statics = to_zero
        .iter()
        .position(|env| matches!(env.msg, Some(v1::envelope::Msg::GameStatic(_))));
    let line = to_zero.iter().position(|env| {
        matches!(&env.msg, Some(v1::envelope::Msg::StateDelta(delta)) if !delta.log_json.is_empty())
    });
    assert!(line.is_some(), "the reveal was sent");
    assert!(
        statics < line,
        "the print entry arrives before the line naming it"
    );
    assert!(session.game_static(zero).print(PrintRef::new(1)).is_some());
}

/// What the log measurement below adds up.
#[derive(Clone, Copy, Debug, Default)]
struct LogStats {
    games: u64,
    lines: u64,
    /// Pumps that sent the watching seat a view.
    sends: u64,
    /// Of those, the ones whose lines needed more than one frame.
    overflowing: u64,
    /// The most lines one pump sent it.
    largest: usize,
    naming: crate::log::Naming,
}

impl LogStats {
    fn add(&mut self, other: &Self) {
        self.games += other.games;
        self.lines += other.lines;
        self.sends += other.sends;
        self.overflowing += other.overflowing;
        self.largest = self.largest.max(other.largest);
        self.naming.references += other.naming.references;
        self.naming.unnamed += other.naming.unnamed;
        self.naming.dropped += other.naming.dropped;
        self.naming.abilities += other.naming.abilities;
        self.naming.by_source += other.naming.by_source;
    }
}

/// One game of `preset` with seat 0 a player the house answers for, as a
/// player would: sent a view after every pump, as a socket is.
fn play_logged(preset: &GamePreset, answers: usize) -> LogStats {
    let mut preset = preset.clone();
    preset.seats[0].controller = SeatController::Open;
    let mut stats = LogStats::default();
    let Some(mut session) = Session::new(&preset) else {
        return stats;
    };
    let me = PlayerId::new(0);
    let mut routed = session.pump();
    for _ in 0..answers {
        let frames = state_frames_to(&routed, me);
        let lines: usize = frames
            .iter()
            .filter_map(|(_, _, tail)| tail.as_ref())
            .map(|tail| tail.entries.len())
            .sum();
        if !frames.is_empty() {
            stats.sends += 1;
        }
        if lines > LOG_TAIL_CAP {
            stats.overflowing += 1;
        }
        stats.largest = stats.largest.max(lines);
        if matches!(session.pending(), Pending::GameOver(_)) {
            break;
        }
        let Some(action) = session.house_action(me) else {
            routed = session.pump();
            continue;
        };
        routed = match session.act(me, action) {
            Ok(routed) => routed,
            Err(_) => match session.timeout_action(me) {
                Some(action) => session.act(me, action).unwrap_or_default(),
                None => break,
            },
        };
    }
    stats.games = 1;
    stats.lines = session.log.len() as u64;
    stats.naming = session.log.naming();
    stats
}

/// How well the log names what it names, and how often a seat has more
/// lines waiting than one frame carries, over self-play: the acceptance
/// decks, and one game per implemented card. #262's handshake reports
/// these numbers; the assertion is only that games were played.
#[test]
#[ignore = "one game per implemented card; minutes, not seconds"]
fn the_log_measured_over_self_play() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/acceptance-decks.txt"
    ))
    .expect("acceptance deck file");
    let allytifact =
        baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact loads");
    let victory = baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory loads");
    let mut acceptance = LogStats::default();
    for seed in 1..=10 {
        acceptance.add(&play_logged(
            &baylee_cards::decks::preset_for(seed, &allytifact, &victory),
            2_000,
        ));
        acceptance.add(&play_logged(
            &baylee_cards::decks::preset_for(seed, &victory, &allytifact),
            2_000,
        ));
    }

    let cards: Vec<&'static baylee_cards_dsl::CardDef> =
        baylee_cards::all().filter(|d| d.is_implemented()).collect();
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    let pool = std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| {
                scope.spawn(move || {
                    let mut stats = LogStats::default();
                    for def in slice {
                        if let Some(preset) = baylee_cards::decks::probe_preset(9, def.index) {
                            stats.add(&play_logged(&preset, 400));
                        }
                    }
                    stats
                })
            })
            .collect();
        let mut total = LogStats::default();
        for handle in handles {
            total.add(&handle.join().expect("a probe chunk does not panic"));
        }
        total
    });
    eprintln!("acceptance: {acceptance:?}");
    eprintln!("pool probes: {pool:?}");
    assert!(acceptance.games > 0 && pool.games > 0);
}
