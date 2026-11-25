use super::*;

#[test]
fn seeds_are_a_function_of_the_root_and_differ_between_forks() {
    assert_eq!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 300, 0));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 300, 1));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 0, 300, 0));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(8, 1, 300, 0));
    assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 301, 0));
}

#[test]
fn only_a_reached_position_is_encoded_and_every_stop_has_its_code_and_name() {
    let mut names = BTreeSet::new();
    for (code, stop) in AfterStop4::ALL.into_iter().enumerate() {
        assert_eq!(usize::from(stop as u8), code, "{}", stop.key());
        assert!(names.insert(stop.key()), "{} twice", stop.key());
        assert_eq!(
            stop.reached(),
            matches!(stop, AfterStop4::Question | AfterStop4::GameOver),
            "{}",
            stop.key()
        );
    }
    // The names an archived sidecar's header gives its codes.
    assert_eq!(
        AfterStop4::ALL.map(AfterStop4::key)[..9],
        [
            "question",
            "game_over",
            "fork_refused",
            "answer_refused",
            "script_mismatch",
            "script_broken",
            "too_long",
            "root_mismatch",
            "forks_differ",
        ]
    );
}

/// The first fault in the module's order is the one the sidecar names.
#[test]
fn an_afterstate_is_judged_by_its_first_fault() {
    use AfterStop4::{
        AnswerRefused, Chance, DealtSeen, ForksDiffer, GameOver, Question, RootMismatch,
    };
    let after = |stop, dealt_seen, chance| Afterstate4 {
        stop,
        answers: 1,
        asked_seat: false,
        next_kind: u8::MAX,
        hidden: Hidden4 {
            event: true,
            dealt_seen,
            chance,
        },
        frames: [None, None],
    };
    for (stop, dealt_seen, chance, root_match, forks_match, judged) in [
        (Question, false, false, true, true, Question),
        (GameOver, false, false, true, true, GameOver),
        (Question, false, true, true, true, Chance),
        (GameOver, true, true, true, true, DealtSeen),
        (Question, true, true, true, false, ForksDiffer),
        (Question, true, true, false, false, RootMismatch),
        (AnswerRefused, true, true, false, false, AnswerRefused),
    ] {
        assert_eq!(
            after(stop, dealt_seen, chance).judged(root_match, forks_match),
            judged
        );
    }
}

#[test]
fn a_card_leaving_a_library_is_a_hidden_event_and_a_life_change_is_not() {
    let draw = GameEvent::ZoneChanged {
        object: baylee_core::ids::ObjectId::new(3, 0),
        from: Zone::Library,
        to: Zone::Hand,
        cause: baylee_engine::event::Cause::TurnBased,
        place: None,
    };
    assert!(hidden_event(&draw));
    let played = GameEvent::ZoneChanged {
        object: baylee_core::ids::ObjectId::new(3, 0),
        from: Zone::Hand,
        to: Zone::Battlefield,
        cause: baylee_engine::event::Cause::Spell,
        place: None,
    };
    assert!(!hidden_event(&played));
    assert!(hidden_event(&GameEvent::Shuffled {
        player: PlayerId::new(0),
        zone: Zone::Library,
    }));
}

fn plain(action: PlayerAction) -> AfterCandidate4 {
    AfterCandidate4 {
        action,
        script: None,
        target: None,
    }
}

/// A house self-play record and, for each of seat 0's inputs, the
/// candidates asked for: its recorded answer, a pass, a land that is no
/// card of the game, and a planned "cast" that is no cast.
fn game_and_roots() -> (Vec<u8>, super::super::search_roots::Roots4) {
    use crate::housedeck::HouseDeck;
    use crate::selfplay::{Caps, play_with_options, table};
    let a = HouseDeck::named("allytifact").unwrap();
    let b = HouseDeck::named("victory").unwrap();
    let preset = table(23, &a, &b, [baylee_core::preset::AIProfile::SHARP; 2]);
    let played = play_with_options(
        &preset,
        "t-afterstates-23",
        Caps {
            answers: 10_000,
            wall: std::time::Duration::from_secs(60),
        },
        true,
    );
    assert!(played.outcome.finished(), "{:?}", played.outcome);
    // The opening of the game: each root is forked sixteen times over.
    let mut afterstates = std::collections::BTreeMap::new();
    for line in played
        .record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .take(160)
    {
        if let Ok(baylee_gamehost::record::Line::Input {
            seat: 0, n, action, ..
        }) = serde_json::from_slice(line)
        {
            afterstates.insert(
                (0, n),
                vec![
                    plain(action),
                    plain(PlayerAction::PassPriority),
                    plain(PlayerAction::PlayLand {
                        card: baylee_core::ids::ObjectId::new(9_999, 0),
                    }),
                    AfterCandidate4 {
                        script: Some(PlayerAction::PassPriority),
                        ..plain(PlayerAction::PassPriority)
                    },
                ],
            );
        }
    }
    let roots = super::super::search_roots::Roots4 {
        roots: afterstates.keys().copied().collect(),
        menu: |_, _, own, _| (own.clone(), own.candidates.len()),
        afterstates,
    };
    (played.record, roots)
}

/// Every root the replay captured, with its frame: (root frame,
/// afterstates).
fn replay(
    record: &[u8],
    roots: &super::super::search_roots::Roots4,
) -> Vec<(Step4, RootAfterstates4)> {
    let mut out = Vec::new();
    super::super::stream_game4_asking(
        record,
        5,
        true,
        None,
        None,
        None,
        false,
        10,
        None,
        super::super::TeamHands::Private,
        super::super::Asks4 {
            roots: Some(roots),
            ..super::super::Asks4::default()
        },
        |mut step| {
            if let Some(root) = step.search_root.take() {
                let captured = root.afterstates.clone().expect("asked for afterstates");
                out.push((step, captured));
            }
            Ok(())
        },
    )
    .unwrap();
    out
}

use super::super::Step4;

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn afterstates_are_a_function_of_the_record_and_stop_at_the_next_question() {
    let (record, roots) = game_and_roots();
    let first = replay(&record, &roots);
    let again = replay(&record, &roots);
    assert!(first.len() > 10, "a game has priority roots of seat 0");
    assert_eq!(first.len(), again.len());
    let (mut passes_to_other, mut own_answers, mut unforked) = (0, 0, 0);
    for ((step, a), (_, b)) in first.iter().zip(&again) {
        assert_eq!(a.candidates.len(), 4);
        for (x, y) in a.candidates.iter().zip(&b.candidates) {
            assert_eq!(
                (x.stop, x.answers, x.asked_seat, x.next_kind),
                (y.stop, y.answers, y.asked_seat, y.next_kind)
            );
            for (p, q) in x.frames.iter().zip(&y.frames) {
                assert_eq!(
                    p.as_ref().map(|f| json(&f.state)),
                    q.as_ref().map(|f| json(&f.state))
                );
            }
        }
        // Mid-chain (mana floating, a cast under way) the engine will not
        // fork: nothing is played or encoded there.
        let Some((state, cands)) = a.root.as_ref() else {
            assert!(
                a.candidates
                    .iter()
                    .all(|c| c.stop == AfterStop4::ForkRefused
                        && c.frames.iter().all(Option::is_none))
            );
            unforked += 1;
            continue;
        };
        // The fork's frame of the root is the replay's own.
        assert_eq!(
            json(state),
            json(&step.state),
            "root at {}",
            step.step_cursor
        );
        assert_eq!(cands.candidates.len(), step.cand_feat.len());
        // The recorded answer is the table's: it reaches the next question
        // after one answer, as do both forks.
        let own = &a.candidates[0];
        assert!(own.stop.reached(), "{:?} at {}", own.stop, step.step_cursor);
        assert_eq!(own.answers, 1);
        assert!(own.frames.iter().all(Option::is_some));
        own_answers += 1;
        // A pass with priority moves the question on; where it goes to the
        // other seat, the seat is not asked.
        let pass = &a.candidates[1];
        if pass.stop.reached() && !pass.asked_seat {
            passes_to_other += 1;
        }
        // No such land: the fork refuses it and nothing is encoded.
        let land = &a.candidates[2];
        assert_eq!(land.stop, AfterStop4::AnswerRefused);
        assert!(land.frames.iter().all(Option::is_none));
        // A script that is no planned cast is never played.
        assert_eq!(a.candidates[3].stop, AfterStop4::ScriptMismatch);
        assert_eq!(a.candidates[3].answers, 0);
    }
    assert!(
        own_answers > 10 && passes_to_other > 0,
        "{own_answers} {passes_to_other} {unforked}"
    );
}

/// A table of two driven seats, each with what it starts with.
struct Table {
    preset: baylee_core::preset::GamePreset,
    session: Session,
}

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

impl Table {
    fn new(battlefields: [&[&str]; 2], their_hand: &[&str]) -> Self {
        use baylee_core::preset::{AIProfile, DeckEntry};
        let entry = |name: &&str| DeckEntry {
            card: baylee_cards::all()
                .find(|card| card.name() == *name)
                .unwrap_or_else(|| panic!("{name} is in the pool"))
                .index,
            print: baylee_core::ids::PrintRef::new(0),
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
        };
        let deck = crate::housedeck::HouseDeck::named("victory").unwrap();
        let mut preset =
            crate::selfplay::table_for(12, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        for (seat, cards) in preset.seats.iter_mut().zip(battlefields) {
            seat.starting_battlefield = cards.iter().map(entry).collect();
        }
        if !their_hand.is_empty() {
            preset.seats[1].starting_hand = Some(their_hand.iter().map(entry).collect());
        }
        let mut session = Session::new(&preset).unwrap();
        for seat in [ME, THEM] {
            session.take_over(seat);
        }
        Self { preset, session }
    }

    fn view(&self, seat: PlayerId) -> PlayerView {
        self.session.observe_view_with_context(seat)
    }

    fn permanent(&self, name: &str) -> ObjectId {
        self.view(ME)
            .battlefield
            .iter()
            .find(|object| object.name == name)
            .unwrap_or_else(|| panic!("{name} is on the battlefield"))
            .id
    }

    /// Whether `seat` has priority in `phase` of its own turn.
    fn main_of(&self, seat: PlayerId) -> bool {
        let view = self.view(seat);
        view.active == seat
            && view.phase == baylee_view::Phase::FirstMain
            && matches!(self.session.pending(), Pending::Priority { player, .. } if *player == seat)
    }

    fn act(&mut self, seat: PlayerId, action: PlayerAction) {
        self.session.act(seat, action).unwrap();
    }

    /// Answers every question with nothing done until `stop`.
    fn idle_until(&mut self, stop: impl Fn(&Self) -> bool) {
        for _ in 0..400 {
            if stop(self) {
                return;
            }
            let seat = self.session.pending().asked().unwrap();
            let action = match self.session.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::Priority { .. } => PlayerAction::PassPriority,
                _ => self.session.house_action(seat).unwrap(),
            };
            self.act(seat, action);
        }
        panic!("the table never got there");
    }

    /// The afterstates of `candidates`, answers of seat 0 at its question.
    fn capture(&self, candidates: &[AfterCandidate4]) -> RootAfterstates4 {
        let (pending, view) = self.session.view_for_with_context(ME).unwrap();
        let mut knowledge = SeatKnowledge4::from_preset(&self.preset, ME);
        knowledge.observe(&view);
        capture(
            &RootContext {
                session: &self.session,
                seat: ME,
                view: &view,
                pending: &pending,
                knowledge: &knowledge,
                teams: &[None, None],
                game: 1,
                cursor: 7,
            },
            candidates,
        )
    }

    /// Seat 0 passes at its question.
    fn pass(&self) -> Afterstate4 {
        self.capture(&[plain(PlayerAction::PassPriority)])
            .candidates
            .remove(0)
    }
}

/// An opponent's draw moves a hidden card and shows none: it is counted as a
/// hidden event and the afterstate stands.
#[test]
fn an_opponents_draw_shows_the_seat_no_dealt_card() {
    let mut table = Table::new([&[], &[]], &[]);
    table.idle_until(|table| {
        table.view(ME).active == THEM
            && table.view(ME).step == baylee_view::Step::Upkeep
            && matches!(table.session.pending(), Pending::Priority { player, .. } if *player == ME)
    });
    let after = table.pass();
    assert!(after.stop.reached(), "{:?}", after.stop);
    assert_eq!(
        after.hidden,
        Hidden4 {
            event: true,
            ..Hidden4::default()
        },
        "the opponent drew"
    );
    assert_eq!(after.judged(true, true), AfterStop4::Question);
}

/// A draw of the seat's own shows it a card the fork dealt, which another deal
/// would hold another card for: the afterstate is not kept, whatever the two
/// frames say.
#[test]
fn a_draw_of_the_seats_own_shows_it_a_dealt_card() {
    let mut table = Table::new([&[], &["Temple Bell"]], &[]);
    table.idle_until(|table| table.main_of(THEM));
    let bell = table.permanent("Temple Bell");
    table.act(
        THEM,
        PlayerAction::ActivateAbility {
            source: bell,
            ability_index: 0,
        },
    );
    table.act(THEM, PlayerAction::PassPriority);
    let after = table.pass();
    assert!(after.stop.reached(), "{:?}", after.stop);
    assert_eq!(
        after.hidden,
        Hidden4 {
            event: true,
            dealt_seen: true,
            chance: false,
        }
    );
    assert_eq!(after.judged(true, true), AfterStop4::DealtSeen);
}

/// Coins fall as the fork's seed has it: an afterstate past a flip is one
/// outcome of several and is not kept.
#[test]
fn a_coin_flip_is_chance() {
    let mut table = Table::new([&[], &["Mountain"]], &["Mana Clash"]);
    table.idle_until(|table| table.main_of(THEM));
    let mountain = table.permanent("Mountain");
    table.act(THEM, PlayerAction::ActivateManaAbility { source: mountain });
    let clash = table.view(THEM).hand[0].id;
    table.act(THEM, PlayerAction::CastSpell { card: clash });
    if matches!(table.session.pending(), Pending::ChooseTargets { .. }) {
        table.act(
            THEM,
            PlayerAction::ChooseTargets {
                objects: Vec::new(),
                players: vec![ME],
            },
        );
    }
    table.idle_until(|table| {
        !table.view(ME).stack.is_empty()
            && matches!(table.session.pending(), Pending::Priority { player, .. } if *player == ME)
    });
    let after = table.pass();
    assert!(after.stop.reached(), "{:?}", after.stop);
    assert!(after.hidden.chance && !after.hidden.dealt_seen);
    assert_eq!(after.judged(true, true), AfterStop4::Chance);
}

/// A candidate tried at a target is played on to it, and one whose target is
/// no answer to the question that follows is named so.
#[test]
fn a_target_variant_is_followed_to_its_target() {
    let mut table = Table::new([&["Prodigal Sorcerer"], &[]], &[]);
    table.idle_until(|table| table.main_of(ME));
    let ping = PlayerAction::ActivateAbility {
        source: table.permanent("Prodigal Sorcerer"),
        ability_index: 0,
    };
    let at = |players: Vec<PlayerId>| AfterCandidate4 {
        target: Some(PlayerAction::ChooseTargets {
            objects: Vec::new(),
            players,
        }),
        ..plain(ping.clone())
    };
    let captured = table.capture(&[plain(ping.clone()), at(vec![THEM]), at(vec![ME, THEM])]);
    let [untargeted, targeted, refused] = &captured.candidates[..] else {
        panic!("three candidates");
    };
    // Without its target the answer stops at the target question.
    assert_eq!(untargeted.stop, AfterStop4::Question);
    assert_eq!(untargeted.answers, 1);
    assert_eq!(untargeted.next_kind, PendingKind4::ChooseTargets as u8);
    // With it the ability is on the stack and the question is the next one.
    assert_eq!(targeted.stop, AfterStop4::Question);
    assert_eq!(targeted.answers, 2);
    assert_eq!(targeted.next_kind, PendingKind4::Priority as u8);
    assert!(targeted.frames.iter().all(Option::is_some));
    assert_eq!(refused.stop, AfterStop4::TargetRefused);
    assert!(refused.frames.iter().all(Option::is_none));
}

/// The two forks of a root deal its hidden cards differently, and each deals
/// them alike for every candidate: one frame in both would say nothing of the
/// hidden cards if it came of one deal.
#[test]
fn the_two_forks_of_a_root_are_dealt_differently() {
    let mut table = Table::new([&["Prodigal Sorcerer"], &[]], &[]);
    table.idle_until(|table| table.main_of(ME));
    let ping = PlayerAction::ActivateAbility {
        source: table.permanent("Prodigal Sorcerer"),
        ability_index: 0,
    };
    let captured = table.capture(&[plain(PlayerAction::PassPriority), plain(ping)]);
    let deals: Vec<[u64; 2]> = captured
        .candidates
        .iter()
        .map(|after| after.frames.each_ref().map(|f| f.as_ref().unwrap().deal))
        .collect();
    assert_eq!(deals.len(), 2);
    assert_eq!(deals[0], deals[1], "a fork's deal is its root's alone");
    assert_ne!(deals[0][0], deals[0][1], "one deal twice");
}
