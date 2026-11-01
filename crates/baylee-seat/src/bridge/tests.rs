use super::*;
use crate::mind::{DeckList, Disclosure, Request, Thinking};
use crate::seat::BridgeConfig;
use crate::transcript::Event;
use baylee_client_core::test_support::{ViewBuilder, statics};
use baylee_core::ids::PlayerId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1;
use baylee_view::{PlayerView, SeatIdentity};
use std::sync::{Mutex, PoisonError};

const ME: PlayerId = PlayerId::new(0);

fn frame(msg: v1::envelope::Msg) -> Vec<u8> {
    Envelope { msg: Some(msg) }.encode_to_vec()
}

/// A seat at a two-chair table, its own called as its mind must be.
fn seated() -> SeatCore {
    let mut game = statics(0);
    game.seats = [(ME, "LLM-seat"), (PlayerId::new(1), "Alice")]
        .into_iter()
        .map(|(player, name)| SeatIdentity {
            player,
            display_name: name.into(),
            is_ai: false,
            away: false,
            team: None,
        })
        .collect();
    let mut core = SeatCore::new(
        BridgeConfig::default(),
        DeckList::default(),
        Disclosure::Llm,
    );
    core.hear(&frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
        game_id: game.game_id.clone(),
        view_version: game.view_version,
        static_json: serde_json::to_vec(&game).unwrap(),
    })));
    core
}

/// The steps of the seat being asked to choose a colour at `seq`: a
/// question its standing orders never answer, so the mind is asked.
fn asked(core: &mut SeatCore, seq: u64) -> Vec<Step> {
    asked_with(core, seq, |_| {})
}

/// [`asked`], in turn `turn`.
fn asked_in_turn(core: &mut SeatCore, seq: u64, turn: u32) -> Vec<Step> {
    asked_with(core, seq, |view| view.turn = turn)
}

/// [`asked`], the view as `tweak` leaves it.
fn asked_with(core: &mut SeatCore, seq: u64, tweak: impl FnOnce(&mut PlayerView)) -> Vec<Step> {
    let steps = put_with(core, seq, tweak);
    assert!(
        steps.iter().any(|s| matches!(s, Step::Ask(_))),
        "the mind is asked: {steps:?}"
    );
    steps
}

/// The question of [`asked_in_turn`], whoever answers it.
fn put_in_turn(core: &mut SeatCore, seq: u64, turn: u32) -> Vec<Step> {
    put_with(core, seq, |view| view.turn = turn)
}

/// The question of [`asked_with`], whoever answers it.
fn put_with(core: &mut SeatCore, seq: u64, tweak: impl FnOnce(&mut PlayerView)) -> Vec<Step> {
    let mut view = ViewBuilder::new(2).build();
    view.seq = seq;
    tweak(&mut view);
    core.hear(&frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq,
        view_json: serde_json::to_vec(&view).unwrap(),
        log_json: Vec::new(),
    })));
    let pending = Pending::ChooseColor {
        player: ME,
        options: vec![ManaColor::Red, ManaColor::Green],
    };
    core.hear(&frame(v1::envelope::Msg::ChoiceRequest(
        v1::ChoiceRequest {
            game_id: "test-game".into(),
            seq,
            pending_json: serde_json::to_vec(&pending).unwrap(),
        },
    )))
}

/// A mind that answers `colour`, and writes down that it was asked.
struct Named {
    colour: ManaColor,
    asked: Arc<Mutex<Vec<ManaColor>>>,
}

impl Mind for Named {
    fn decide(&self, _request: Request) -> Thinking<'_> {
        self.asked.lock().unwrap().push(self.colour);
        let colour = self.colour;
        Box::pin(async move { Ok(Answer::new(PlayerAction::ChooseColor(colour))) })
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::Llm
    }
}

/// A mind handed over during the game answers from the next question
/// on, never the question already being thought about; and once it
/// does, the swap says so (where the old mind's spend is settled).
#[tokio::test]
async fn a_swapped_mind_answers_from_the_next_question_on() {
    let asked_of = Arc::new(Mutex::new(Vec::new()));
    let mind = |colour| -> Arc<dyn Mind> {
        Arc::new(Named {
            colour,
            asked: Arc::clone(&asked_of),
        })
    };
    let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
    let mut bridge = Bridge::new(
        mind(ManaColor::Red),
        PlayOptions::default(),
        Some(swaps),
        Vec::new(),
    );
    let mut core = seated();
    let first = asked(&mut core, 2);
    // Handed over before the question reaches the mind: it is the new
    // mind's, at the moment it is asked.
    let taken = Arc::new(Mutex::new(0));
    let count = Arc::clone(&taken);
    send.send(Swap {
        mind: mind(ManaColor::Green),
        think: None,
        taken: Some(Box::new(move || *count.lock().unwrap() += 1)),
        declared: None,
    })
    .unwrap_or_else(|_| panic!("the bridge listens"));
    bridge.carry_out(first, None).await;
    assert_eq!(*taken.lock().unwrap(), 1);
    let thinking = bridge.thinking.take().unwrap();
    core.answered(thinking.question, thinking.task.await.unwrap());
    assert_eq!(*asked_of.lock().unwrap(), [ManaColor::Green]);
    // One handed over while a question is being thought about waits
    // for the next question.
    let second = asked(&mut core, 3);
    bridge.carry_out(second, None).await;
    send.send(Swap {
        mind: mind(ManaColor::Red),
        think: None,
        taken: None,
        declared: None,
    })
    .unwrap_or_else(|_| panic!("the bridge listens"));
    let thinking = bridge.thinking.take().unwrap();
    core.answered(thinking.question, thinking.task.await.unwrap());
    assert_eq!(
        *asked_of.lock().unwrap(),
        [ManaColor::Green, ManaColor::Green]
    );
    let third = asked(&mut core, 4);
    bridge.carry_out(third, None).await;
    let thinking = bridge.thinking.take().unwrap();
    core.answered(thinking.question, thinking.task.await.unwrap());
    assert_eq!(
        *asked_of.lock().unwrap(),
        [ManaColor::Green, ManaColor::Green, ManaColor::Red]
    );
    assert_eq!(*taken.lock().unwrap(), 1, "called once");
}

/// The budget of the question the core asks in `steps`.
fn budget(steps: &[Step]) -> Option<Duration> {
    steps.iter().find_map(|s| match s {
        Step::Ask(request) => Some(request.budget),
        _ => None,
    })
}

/// A mind handed over brings its profile's think time: every question
/// after it is bounded by that, and the one it is first asked as soon
/// as it is taken, where that is shorter.
#[tokio::test]
async fn a_swapped_mind_thinks_for_as_long_as_its_profile_says() {
    let mind = |colour| -> Arc<dyn Mind> {
        Arc::new(Named {
            colour,
            asked: Arc::new(Mutex::new(Vec::new())),
        })
    };
    let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
    let mut bridge = Bridge::new(
        mind(ManaColor::Red),
        PlayOptions::default(),
        Some(swaps),
        Vec::new(),
    );
    let mut core = seated();
    let swap = |think| Swap {
        mind: mind(ManaColor::Green),
        think: Some(Duration::from_secs(think)),
        taken: None,
        declared: None,
    };
    send.send(swap(7))
        .unwrap_or_else(|_| panic!("the bridge listens"));
    // Taken before the core hears the question: it asks under 7 s, not
    // the bridge's 60.
    bridge.settle(&mut core);
    let first = asked(&mut core, 2);
    assert_eq!(budget(&first), Some(Duration::from_secs(7)));
    // Taken only as the question reaches the mind: held to 2 s at once.
    send.send(swap(2))
        .unwrap_or_else(|_| panic!("the bridge listens"));
    let before = Instant::now();
    bridge.carry_out(first, None).await;
    let thinking = bridge.thinking.take().unwrap();
    assert!(thinking.deadline <= before + Duration::from_secs(3));
    core.answered(thinking.question, thinking.task.await.unwrap());
    bridge.settle(&mut core);
    let second = asked(&mut core, 3);
    assert_eq!(budget(&second), Some(Duration::from_secs(2)));
}

/// A mind swapped during the game says what it is before it answers:
/// the declaration is carried out ahead of the new mind's first
/// answer, so the record writes the change where it happened.
#[tokio::test]
async fn a_swapped_mind_is_declared_before_its_first_answer() {
    let mind = |colour| -> Arc<dyn Mind> {
        Arc::new(Named {
            colour,
            asked: Arc::new(Mutex::new(Vec::new())),
        })
    };
    let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
    let mut bridge = Bridge::new(
        mind(ManaColor::Red),
        PlayOptions::default(),
        Some(swaps),
        Vec::new(),
    );
    let mut core = seated();
    let first = asked(&mut core, 2);
    send.send(Swap {
        mind: mind(ManaColor::Green),
        think: None,
        taken: None,
        declared: Some(v1::SeatMind {
            kind: v1::seat_mind::Kind::LlmApi as i32,
            provider: "anthropic".into(),
            model: "claude-sonnet-5-5".into(),
            effort: "high".into(),
            level: String::new(),
        }),
    })
    .unwrap_or_else(|_| panic!("the bridge listens"));
    bridge.carry_out(first, None).await;
    let joined = (&mut bridge.thinking.as_mut().unwrap().task).await;
    let answer = bridge.answered(&mut core, joined).unwrap();
    let declared: Vec<_> = bridge
        .declaring
        .iter()
        .filter_map(|step| match step {
            Step::Send(Envelope {
                msg: Some(v1::envelope::Msg::SeatMind(mind)),
            }) => Some(mind.model.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(declared, ["claude-sonnet-5-5"]);
    assert!(
        answer
            .iter()
            .any(|step| matches!(step, Step::Answer { .. })),
        "the new mind's answer follows"
    );
    bridge.carry_out(answer, None).await;
    assert!(
        bridge.declaring.is_empty(),
        "sent once, ahead of the answer"
    );
}
struct Ending(Arc<Mutex<Vec<(String, u8)>>>);
impl Mind for Ending {
    fn decide(&self, _request: Request) -> Thinking<'_> {
        Box::pin(std::future::pending())
    }
    fn disclosure(&self) -> Disclosure {
        Disclosure::Llm
    }
    fn end_game(&self, game: &str, seat: u8) {
        self.0.lock().unwrap().push((game.into(), seat));
    }
}

#[tokio::test]
async fn swapping_and_dropping_a_bridge_release_its_owned_session() {
    let ended = Arc::new(Mutex::new(Vec::new()));
    let mind: Arc<dyn Mind> = Arc::new(Ending(ended.clone()));
    let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(std::future::pending());
    let aborted = task.abort_handle();
    let mut bridge = Bridge::new(
        mind.clone(),
        PlayOptions::default(),
        Some(swaps),
        Vec::new(),
    );
    bridge.game_seat = Some(("game-owned".into(), 2));
    bridge.thinking = Some(super::Thinking {
        question: 1,
        deadline: Instant::now(),
        task,
        request: None,
    });
    send.send(Swap {
        mind: Arc::new(Ending(ended.clone())),
        think: None,
        taken: None,
        declared: None,
    })
    .unwrap();
    bridge.take_swaps();
    assert_eq!(*ended.lock().unwrap(), [("game-owned".into(), 2)]);
    drop(bridge);
    tokio::task::yield_now().await;
    assert!(aborted.is_finished());
    assert_eq!(
        *ended.lock().unwrap(),
        [("game-owned".into(), 2), ("game-owned".into(), 2)]
    );
    // Other users may still own the old mind; teardown is tied to this
    // bridge's game/seat rather than waiting for its last Arc to disappear.
    assert_eq!(Arc::strong_count(&mind), 1);
}

/// The turn the fallback tests play in, so that a turn the fallback
/// reports cannot be a default.
const TURN: u32 = 7;

/// What a mind was asked, and which games it was told to end.
#[derive(Default)]
struct Seen {
    asked: Vec<Asked>,
    ended: Vec<(String, u8)>,
}

/// One question as a mind got it.
struct Asked {
    question: u64,
    retried: bool,
    budget: Duration,
}

/// What a [`Moving`] mind wrote down, shared with the test.
type Record = Arc<Mutex<Seen>>;

/// What a [`Moving`] mind does at a call.
#[derive(Clone, Copy)]
enum Move {
    /// Answers this colour.
    Choose(ManaColor),
    /// The provider says it is rate limited (429, `Retry-After` 30 s).
    Limited,
    /// The provider failed otherwise ([`FAILED`], no limit).
    Unavailable,
    /// The mind will not answer this kind of question.
    Declined,
}

/// What a [`Move::Unavailable`] mind says: the staging acceptance's
/// `502` (13.10.2026), the failure the house answered then.
const FAILED: &str = "502: server_error: The website reported an error for this message";

/// A mind that plays `moves` call by call, the last one from then on,
/// and writes down what it was asked.
struct Moving {
    moves: Vec<Move>,
    seen: Record,
}

impl Moving {
    fn mind(moves: Vec<Move>, seen: &Record) -> Arc<dyn Mind> {
        Arc::new(Self {
            moves,
            seen: Arc::clone(seen),
        })
    }
}

impl Mind for Moving {
    fn decide(&self, request: Request) -> Thinking<'_> {
        let call = {
            let mut seen = self.seen.lock().unwrap();
            seen.asked.push(Asked {
                question: request.question,
                retried: request.retry.is_some(),
                budget: request.budget,
            });
            seen.asked.len()
        };
        let step = self.moves[(call - 1).min(self.moves.len() - 1)];
        Box::pin(async move {
            match step {
                Move::Choose(colour) => Ok(Answer::new(PlayerAction::ChooseColor(colour))),
                Move::Limited => Err(MindError::Limited {
                    why: "429: slow down".into(),
                    lifts_in: Some(Duration::from_secs(30)),
                }),
                Move::Unavailable => Err(MindError::Unavailable(FAILED.into())),
                Move::Declined => Err(MindError::Declined("not this question".into())),
            }
        })
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::Llm
    }

    fn end_game(&self, game: &str, seat: u8) {
        // Called as the bridge is dropped, even by a test that failed.
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        seen.ended.push((game.into(), seat));
    }
}

/// The limits a standby's arm was called with.
type Armed = Arc<Mutex<Vec<Limit>>>;

/// What a model behind an API says it is, for the table.
fn api_mind(model: &str) -> v1::SeatMind {
    v1::SeatMind {
        kind: v1::seat_mind::Kind::LlmApi as i32,
        provider: "openai".into(),
        model: model.into(),
        effort: String::new(),
        level: String::new(),
    }
}

/// What stands by in a [`Scene`].
enum Reserve {
    /// A fallback `backup` that takes over with a mind playing these
    /// moves, declared as `model-b`, thinking for 20 s.
    Takes(Vec<Move>),
    /// A fallback `backup` whose arm refuses, with this reason.
    Refuses(&'static str),
    /// No fallback.
    Absent,
}

impl Reserve {
    /// The chain it is.
    fn links(self) -> Vec<Link> {
        match self {
            Self::Takes(moves) => vec![Link::Takes {
                label: "backup",
                model: "model-b",
                moves,
            }],
            Self::Refuses(why) => vec![Link::Refuses {
                label: "backup",
                why,
            }],
            Self::Absent => Vec::new(),
        }
    }
}

/// One fallback of a chain in a [`Scene`], in the order they take over.
enum Link {
    /// The profile `label` takes over with a mind playing `moves`,
    /// declared as `model`, thinking for 20 s.
    Takes {
        label: &'static str,
        model: &'static str,
        moves: Vec<Move>,
    },
    /// The profile `label`'s arm refuses, with `why`.
    Refuses {
        label: &'static str,
        why: &'static str,
    },
}

/// A bridge whose primary mind plays `primary`, a core seated before it
/// (every question in turn [`TURN`]), and what the minds and the arms
/// wrote down. The bridge is driven as `run` drives it, a step at a
/// time, with no socket unless the test gives it one.
struct Scene {
    bridge: Bridge,
    core: SeatCore,
    primary: Record,
    /// What the first fallback was asked.
    fallback: Record,
    /// What each fallback of the chain was asked, in its order.
    chain: Vec<Record>,
    /// Every arm called, with what, in order.
    armed: Armed,
    /// The profiles whose arms were called, in order.
    armed_by: Arc<Mutex<Vec<&'static str>>>,
    wire: Option<Socket>,
}

impl Scene {
    fn new(primary: Vec<Move>, reserve: Reserve) -> Self {
        Self::chained(primary, reserve.links())
    }

    /// [`Self::new`], with a chain of fallbacks behind the primary.
    fn chained(primary: Vec<Move>, links: Vec<Link>) -> Self {
        let primary_seen = Record::default();
        let armed = Armed::default();
        let armed_by = Arc::new(Mutex::new(Vec::new()));
        let mut chain = Vec::new();
        let mut standbys = Vec::new();
        for link in links {
            let seen = Record::default();
            let (armed, armed_by) = (Arc::clone(&armed), Arc::clone(&armed_by));
            let standby = match link {
                Link::Takes {
                    label,
                    model,
                    moves,
                } => {
                    let (mind, declared) = (Moving::mind(moves, &seen), api_mind(model));
                    Standby {
                        label: label.into(),
                        arm: Box::new(move |limit| {
                            armed.lock().unwrap().push(limit.clone());
                            armed_by.lock().unwrap().push(label);
                            Ok(Swap {
                                mind,
                                think: Some(Duration::from_secs(20)),
                                taken: None,
                                declared: Some(declared),
                            })
                        }),
                    }
                }
                Link::Refuses { label, why } => Standby {
                    label: label.into(),
                    arm: Box::new(move |limit| {
                        armed.lock().unwrap().push(limit.clone());
                        armed_by.lock().unwrap().push(label);
                        Err(why.into())
                    }),
                },
            };
            chain.push(seen);
            standbys.push(standby);
        }
        let options = PlayOptions {
            min_think: Duration::ZERO,
            ..PlayOptions::default()
        };
        Self {
            bridge: Bridge::new(
                Moving::mind(primary, &primary_seen),
                options,
                None,
                standbys,
            ),
            core: seated(),
            primary: primary_seen,
            fallback: chain.first().cloned().unwrap_or_default(),
            chain,
            armed,
            armed_by,
            wire: None,
        }
    }

    /// Carries `steps` out, on the socket if there is one.
    async fn carry(&mut self, steps: Vec<Step>) {
        self.bridge.carry_out(steps, self.wire.as_mut()).await;
    }

    /// The seat is asked a question at `seq` and the bridge hands it to
    /// its mind; the budget the core gave it.
    async fn ask(&mut self, seq: u64) -> Duration {
        let steps = asked_in_turn(&mut self.core, seq, TURN);
        let budget = budget(&steps).expect("the mind is asked");
        self.carry(steps).await;
        budget
    }

    /// The mind that is thinking comes back, and the core's steps for
    /// what it said.
    async fn back(&mut self) -> Vec<Step> {
        let task = &mut self
            .bridge
            .thinking
            .as_mut()
            .expect("a question is being thought about")
            .task;
        let joined = task.await;
        self.bridge
            .answered(&mut self.core, joined)
            .expect("the question was being thought about")
    }

    /// [`Self::back`] and what comes of it carried out: who answered
    /// with what. Where the mind was limited and a fallback took the
    /// question over, the fallback is waited for too.
    async fn answer(&mut self) -> Vec<(By, PlayerAction)> {
        let mut steps = self.back().await;
        if steps.is_empty() && self.bridge.thinking.is_some() {
            self.carry(steps).await;
            steps = self.back().await;
        }
        let made = answers(&steps);
        self.carry(steps).await;
        made
    }
}

/// Who answered with what, in `steps`.
fn answers(steps: &[Step]) -> Vec<(By, PlayerAction)> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Answer { by, action, .. } => Some((*by, action.clone())),
            _ => None,
        })
        .collect()
}

/// The models a bridge is about to declare to the table.
fn declaring(bridge: &Bridge) -> Vec<String> {
    bridge
        .declaring
        .iter()
        .filter_map(|step| match step {
            Step::Send(Envelope {
                msg: Some(v1::envelope::Msg::SeatMind(mind)),
            }) => Some(mind.model.clone()),
            _ => None,
        })
        .collect()
}

/// The error of every `MindFailed` note the core wrote.
fn failures(core: &mut SeatCore) -> Vec<String> {
    core.take_notes()
        .into_iter()
        .filter_map(|note| match note.event {
            Event::MindFailed { error } => Some(error),
            _ => None,
        })
        .collect()
}

/// A limit where a fallback stands by never reaches the core: the same
/// question is asked of the fallback, which answers it and the ones
/// after it, and nothing is the house's. (Before, the limit was an
/// `Unavailable`: the house answered that question.)
#[tokio::test]
async fn a_limit_where_a_fallback_stands_by_is_answered_by_the_fallback() {
    let mut scene = Scene::new(
        vec![Move::Choose(ManaColor::Red), Move::Limited],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    // The primary answers while it can, and the fallback is not armed.
    scene.ask(2).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
    );
    assert!(scene.armed.lock().unwrap().is_empty());
    assert!(scene.bridge.fell_back.is_empty());

    // The second question is the one the primary cannot answer.
    let budget = scene.ask(3).await;
    let thinking = scene.bridge.thinking.as_ref().unwrap();
    let (question, deadline) = (thinking.question, thinking.deadline);
    let steps = scene.back().await;
    assert!(steps.is_empty(), "the core hears nothing of it: {steps:?}");
    let thinking = scene
        .bridge
        .thinking
        .as_ref()
        .expect("asked of the fallback");
    assert_eq!(
        (thinking.question, thinking.deadline),
        (question, deadline),
        "the same question, under the same deadline"
    );
    scene.carry(steps).await;
    let context = scene.core.context().unwrap().clone();
    assert_eq!(
        *scene.armed.lock().unwrap(),
        [Limit {
            turn: TURN,
            why: "429: slow down".into(),
            lifts_in: Some(Duration::from_secs(30)),
            failed: false,
            by: None,
        }],
        "armed once, on the limit"
    );
    assert_eq!(
        scene.primary.lock().unwrap().ended,
        [(context.game_id.clone(), context.seat.get())],
        "the primary plays no more of this game"
    );
    assert_eq!(
        scene.bridge.notes,
        [serde_json::json!({
            "fallback": {
                "profile": "backup",
                "turn": TURN,
                "cause": "limited",
                "why": "429: slow down",
            },
        })]
    );
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))],
        "the fallback answers the question the primary could not"
    );
    {
        let fallback = scene.fallback.lock().unwrap();
        assert_eq!(fallback.asked.len(), 1);
        assert_eq!(fallback.asked[0].question, question);
        assert!(!fallback.asked[0].retried);
        assert!(fallback.asked[0].budget <= budget, "within what was left");
        assert!(fallback.asked[0].budget + Duration::from_secs(5) > budget);
    }

    // And the questions after it.
    scene.ask(4).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    assert_eq!(scene.primary.lock().unwrap().asked.len(), 2);
    assert_eq!(scene.fallback.lock().unwrap().asked.len(), 2);
    assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
    assert_eq!(
        scene.bridge.fell_back,
        [FellBack {
            label: "backup".into(),
            turn: TURN,
            answered: 2,
        }],
        "the fallback's two answers, not the primary's"
    );
    let stats = scene.core.stats();
    assert_eq!(
        (stats.answered.mind, stats.answered.house, stats.asked),
        (3, 0, 3),
        "three questions, each asked of the core's mind once"
    );
    assert_eq!(stats.fallbacks.total(), 0, "no house answer, no failure");
    assert_eq!(stats.mind_down, 0);
    assert!(failures(&mut scene.core).is_empty());
}

/// The fallback is asked the question afresh: an answer the primary
/// gave and the referee refused was the primary's, so the retry is
/// cleared, and the time it has is what is left to the deadline the
/// primary was under, not a new budget.
#[tokio::test]
async fn a_fallback_is_asked_the_question_afresh_within_the_time_that_is_left() {
    // Blue is not among the colours on offer: the referee refuses it,
    // and the primary is asked again, told why, and is limited then.
    let mut scene = Scene::new(
        vec![Move::Choose(ManaColor::Blue), Move::Limited],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    scene.ask(2).await;
    let retry = scene.back().await;
    assert!(
        matches!(&retry[..], [Step::Ask(request)] if request.retry.is_some()),
        "asked again with the refusal: {retry:?}"
    );
    let retry_budget = budget(&retry).unwrap();
    scene.carry(retry).await;
    let deadline = scene.bridge.thinking.as_ref().unwrap().deadline;

    assert!(scene.back().await.is_empty(), "the limit is not the core's");
    assert_eq!(scene.bridge.thinking.as_ref().unwrap().deadline, deadline);
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    let (retried, question) = {
        let primary = scene.primary.lock().unwrap();
        assert_eq!(primary.asked.len(), 2);
        (primary.asked[1].retried, primary.asked[1].question)
    };
    assert!(retried, "the primary was asked with the refusal");
    let fallback = scene.fallback.lock().unwrap();
    assert_eq!(fallback.asked.len(), 1);
    assert!(
        !fallback.asked[0].retried,
        "a first asking for the fallback"
    );
    assert_eq!(fallback.asked[0].question, question);
    assert!(fallback.asked[0].budget <= retry_budget);
    assert!(fallback.asked[0].budget + Duration::from_secs(5) > retry_budget);
}

/// The fallback says what it is to the table before its first answer,
/// so the game's record writes the change where it happened: on the
/// wire the primary's answer comes first, then the declaration, then
/// the fallback's.
#[tokio::test]
async fn the_fallback_is_declared_before_its_first_answer() {
    let mut scene = Scene::new(
        vec![Move::Choose(ManaColor::Red), Move::Limited],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    let (socket, mut table) = wired().await;
    scene.wire = Some(socket);
    scene.ask(2).await;
    scene.answer().await;
    scene.ask(3).await;
    // The limit is taken over as `run` takes it: nothing to send but
    // the declaration, which waits to be carried out.
    let steps = scene.back().await;
    assert!(steps.is_empty());
    assert_eq!(declaring(&scene.bridge), ["model-b"]);
    scene.carry(steps).await;
    assert!(scene.bridge.declaring.is_empty(), "sent once, at once");
    scene.answer().await;
    let mut wire = Vec::new();
    for _ in 0..3 {
        let envelope = tokio::time::timeout(Duration::from_secs(5), table.recv())
            .await
            .expect("the table is sent three frames")
            .expect("the socket stays open");
        wire.push(match envelope.msg {
            Some(v1::envelope::Msg::PlayerAction(msg)) => {
                let action: PlayerAction = serde_json::from_slice(&msg.action_json).unwrap();
                format!("answer {action:?}")
            }
            Some(v1::envelope::Msg::SeatMind(mind)) => format!("mind {}", mind.model),
            other => format!("{other:?}"),
        });
    }
    assert_eq!(
        wire,
        [
            "answer ChooseColor(Red)",
            "mind model-b",
            "answer ChooseColor(Green)"
        ]
    );
}

/// The fallback brings its profile's think time: the questions after
/// the take-over are bounded by it, not by the primary's.
#[tokio::test]
async fn the_fallback_thinks_for_as_long_as_its_profile_says() {
    let mut scene = Scene::new(
        vec![Move::Limited],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    let before = scene.ask(2).await;
    assert_eq!(before, Duration::from_secs(60), "the primary's bound");
    scene.answer().await;
    assert_eq!(scene.ask(3).await, Duration::from_secs(20));
}

/// Where the fallback cannot take over (its own spend book refuses, say)
/// the limit is the core's as without one: the house answers and plays
/// the rest of the game, the transcript says why the fallback did not,
/// and the fallback stands by no longer.
#[tokio::test]
async fn a_fallback_that_cannot_take_over_leaves_the_limit_to_the_house() {
    let refusal = "the day's cap of $3.00 is spent";
    let mut scene = Scene::new(
        vec![Move::Choose(ManaColor::Red), Move::Limited],
        Reserve::Refuses(refusal),
    );
    scene.ask(2).await;
    scene.answer().await;
    scene.ask(3).await;
    let steps = scene.back().await;
    let made = answers(&steps);
    assert!(
        matches!(&made[..], [(By::House, _)]),
        "the house answers: {made:?}"
    );
    scene.carry(steps).await;

    let failed = failures(&mut scene.core);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].contains("rate limited"), "{}", failed[0]);
    assert!(
        failed[0].contains("the fallback backup could not take over"),
        "{}",
        failed[0]
    );
    assert!(failed[0].contains(refusal), "{}", failed[0]);
    assert_eq!(
        scene.bridge.notes,
        [serde_json::json!({
            "fallback_refused": { "profile": "backup", "why": refusal },
        })]
    );
    assert!(scene.bridge.fallbacks.is_empty(), "it stands by no longer");
    assert!(scene.bridge.fell_back.is_empty());
    assert!(
        scene.primary.lock().unwrap().ended.is_empty(),
        "the primary plays on"
    );
    let stats = scene.core.stats();
    assert_eq!(
        (stats.answered.house, stats.fallbacks.unavailable),
        (1, 1),
        "the same road as the mind unavailable"
    );

    // The house has the rest of the game (`SeatCore::limited`): neither
    // the mind is asked again nor the fallback armed again.
    scene.bridge.notes.clear();
    let steps = put_in_turn(&mut scene.core, 4, TURN);
    assert!(budget(&steps).is_none(), "the mind was asked again");
    assert!(matches!(&answers(&steps)[..], [(By::House, _)]));
    scene.carry(steps).await;
    assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
    assert!(scene.bridge.notes.is_empty(), "{:?}", scene.bridge.notes);
    assert_eq!(scene.primary.lock().unwrap().asked.len(), 2);
    assert!(scene.core.stats().housed);
}

/// With no fallback a limit hands the rest of the game to the house at
/// once (feedback of 28.10.2026): the house is declared to the table,
/// answers the question the mind could not, and every one after it; the
/// mind is asked nothing more and the seat stays at the table. Before,
/// a run of three limits took the mind off the table: the bridge left
/// its socket, the table held the chair as away for its whole reconnect
/// window before its stand-in played, and the mind came back once it
/// had cooled down, to be limited again.
#[tokio::test]
async fn without_a_fallback_a_limit_hands_the_rest_of_the_game_to_the_house() {
    let mut scene = Scene::new(vec![Move::Limited], Reserve::Absent);
    scene.ask(2).await;
    let steps = scene.back().await;
    let house = steps.iter().position(|step| {
        matches!(step, Step::Send(Envelope { msg: Some(v1::envelope::Msg::SeatMind(mind)) })
            if mind.kind == v1::seat_mind::Kind::House as i32)
    });
    let answered = steps
        .iter()
        .position(|step| matches!(step, Step::Answer { by: By::House, .. }));
    assert!(
        matches!((house, answered), (Some(h), Some(a)) if h < a),
        "the house is declared ahead of its first answer: {steps:?}"
    );
    assert!(!steps.iter().any(|step| matches!(step, Step::MindDown)));
    scene.carry(steps).await;
    for seq in [3, 4, 5] {
        let steps = put_in_turn(&mut scene.core, seq, TURN);
        assert!(budget(&steps).is_none(), "the mind was asked at {seq}");
        assert!(
            matches!(&answers(&steps)[..], [(By::House, _)]),
            "{steps:?}"
        );
        assert!(!steps.iter().any(|step| matches!(step, Step::MindDown)));
        scene.carry(steps).await;
    }
    assert_eq!(scene.primary.lock().unwrap().asked.len(), 1);
    let stats = scene.core.stats();
    assert_eq!(
        (stats.answered.house, stats.mind_down, stats.housed),
        (4, 0, true)
    );
    assert_eq!(
        failures(&mut scene.core)[0],
        "the mind is rate limited: 429: slow down (it lifts in 30 s)"
    );
    assert!(scene.bridge.notes.is_empty());
    assert!(scene.bridge.fell_back.is_empty());
    assert!(scene.armed.lock().unwrap().is_empty());
}

/// A fallback that is limited too has nothing behind it: the limit is
/// the core's, the house answers, and only what the fallback answered
/// is counted as its own.
#[tokio::test]
async fn a_fallback_that_is_limited_too_leaves_the_limit_to_the_house() {
    let mut scene = Scene::new(
        vec![Move::Limited],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green), Move::Limited]),
    );
    scene.ask(2).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    scene.ask(3).await;
    let made = scene.answer().await;
    assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
    assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
    assert_eq!(scene.fallback.lock().unwrap().asked.len(), 2);
    assert_eq!(
        scene.bridge.fell_back.last().map(|fell| fell.answered),
        Some(1),
        "the house's answer is not the fallback's"
    );
    let stats = scene.core.stats();
    assert_eq!((stats.answered.house, stats.fallbacks.unavailable), (1, 1));
}

/// A mind handed over during the game plays another profile, so the
/// fallback held for the old one is let go: a limit of the new mind
/// goes to the core, and the fallback is never armed.
#[tokio::test]
async fn a_swapped_mind_drops_the_fallback_held_for_the_old_one() {
    let mut scene = Scene::new(
        vec![Move::Choose(ManaColor::Red)],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
    scene.bridge.swaps = Some(swaps);
    let handed = Record::default();
    send.send(Swap {
        mind: Moving::mind(vec![Move::Limited], &handed),
        think: None,
        taken: None,
        declared: None,
    })
    .unwrap_or_else(|_| panic!("the bridge listens"));
    scene.bridge.settle(&mut scene.core);
    assert!(scene.bridge.fallbacks.is_empty());
    scene.ask(2).await;
    let made = scene.answer().await;
    assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
    assert_eq!(handed.lock().unwrap().asked.len(), 1);
    assert!(scene.armed.lock().unwrap().is_empty());
    assert!(scene.fallback.lock().unwrap().asked.is_empty());
    assert_eq!(scene.core.stats().fallbacks.unavailable, 1);
}

/// The fallback's answers are counted for the fallback only while it
/// plays: a mind handed over after it took over is another profile's,
/// and what it answers is not the fallback's.
#[tokio::test]
async fn a_mind_handed_over_after_the_fallback_is_not_counted_as_the_fallback() {
    let mut scene = Scene::new(
        vec![Move::Limited],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    scene.ask(2).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
    scene.bridge.swaps = Some(swaps);
    let handed = Record::default();
    send.send(Swap {
        mind: Moving::mind(vec![Move::Choose(ManaColor::Red)], &handed),
        think: None,
        taken: None,
        declared: None,
    })
    .unwrap_or_else(|_| panic!("the bridge listens"));
    scene.bridge.settle(&mut scene.core);
    scene.ask(3).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
    );
    assert_eq!(
        scene.bridge.fell_back.last().map(|fell| fell.answered),
        Some(1),
        "only the fallback's own answer"
    );
    assert_eq!(scene.core.stats().answered.mind, 2);
}

/// The staging acceptance (13.10.2026): the primary answered `502`
/// mid-game, an `Unavailable`, and the house took the question though a
/// fallback stood by. A failure is now taken over as a limit is: the
/// same question goes to the fallback under the same deadline, it plays
/// the rest of the game, and the house answers nothing.
#[tokio::test]
async fn a_primary_that_fails_where_a_fallback_stands_by_is_answered_by_the_fallback() {
    let mut scene = Scene::new(
        vec![
            Move::Choose(ManaColor::Red),
            Move::Choose(ManaColor::Red),
            Move::Unavailable,
        ],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    for seq in [2, 3] {
        scene.ask(seq).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
        );
    }
    assert!(scene.armed.lock().unwrap().is_empty(), "not armed yet");

    // The third question is the one the primary fails at.
    let budget = scene.ask(4).await;
    let thinking = scene.bridge.thinking.as_ref().unwrap();
    let (question, deadline) = (thinking.question, thinking.deadline);
    let steps = scene.back().await;
    assert!(steps.is_empty(), "the core hears nothing of it: {steps:?}");
    let thinking = scene
        .bridge
        .thinking
        .as_ref()
        .expect("asked of the fallback");
    assert_eq!((thinking.question, thinking.deadline), (question, deadline));
    assert_eq!(declaring(&scene.bridge), ["model-b"], "declared first");
    scene.carry(steps).await;
    let limit = Limit {
        turn: TURN,
        why: FAILED.into(),
        lifts_in: None,
        failed: true,
        by: None,
    };
    assert_eq!(*scene.armed.lock().unwrap(), std::slice::from_ref(&limit));
    assert_eq!(limit.said(), format!("the primary failed: {FAILED}"));
    assert!(
        !limit.said().contains(LIMITED),
        "a seat agent does not hear a limit"
    );
    assert_eq!(
        scene.bridge.notes,
        [serde_json::json!({
            "fallback": {
                "profile": "backup",
                "turn": TURN,
                "cause": "failed",
                "why": FAILED,
            },
        })]
    );
    let context = scene.core.context().unwrap().clone();
    assert_eq!(
        scene.primary.lock().unwrap().ended,
        [(context.game_id.clone(), context.seat.get())],
        "the primary plays no more of this game"
    );
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))],
        "the fallback answers the question the primary failed at"
    );
    {
        let fallback = scene.fallback.lock().unwrap();
        assert_eq!(fallback.asked.len(), 1);
        assert_eq!(fallback.asked[0].question, question);
        assert!(!fallback.asked[0].retried);
        assert!(fallback.asked[0].budget <= budget, "within what was left");
    }

    // And the questions after it, the primary never asked again.
    for seq in [5, 6] {
        scene.ask(seq).await;
        assert_eq!(
            scene.answer().await,
            [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
        );
    }
    assert_eq!(scene.primary.lock().unwrap().asked.len(), 3);
    assert_eq!(scene.fallback.lock().unwrap().asked.len(), 3);
    assert_eq!(scene.armed.lock().unwrap().len(), 1, "armed once");
    assert_eq!(
        scene.bridge.fell_back,
        [FellBack {
            label: "backup".into(),
            turn: TURN,
            answered: 3,
        }]
    );
    let stats = scene.core.stats();
    assert_eq!(
        (stats.answered.mind, stats.answered.house, stats.asked),
        (5, 0, 5)
    );
    assert_eq!(stats.fallbacks.total(), 0, "no house answer, no failure");
    assert_eq!(stats.mind_down, 0);
    assert!(failures(&mut scene.core).is_empty());
}

/// Without a fallback a failure of the mind is the house's, as it was:
/// it counts as the mind unavailable, a run of them takes the mind off
/// the table, and the transcript says what failed.
#[tokio::test]
async fn without_a_fallback_a_failure_is_the_house_s_to_answer() {
    let mut scene = Scene::new(vec![Move::Unavailable], Reserve::Absent);
    for (seq, down) in [(2, false), (3, false), (4, true)] {
        scene.ask(seq).await;
        let steps = scene.back().await;
        let made = answers(&steps);
        assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
        assert_eq!(
            steps.iter().any(|step| matches!(step, Step::MindDown)),
            down,
            "taken off the table on the third in a row"
        );
        scene.carry(steps).await;
    }
    let stats = scene.core.stats();
    assert_eq!(
        (
            stats.answered.house,
            stats.fallbacks.unavailable,
            stats.mind_down
        ),
        (3, 3, 1)
    );
    assert_eq!(
        failures(&mut scene.core)[0],
        format!("the mind is unavailable: {FAILED}")
    );
    assert!(scene.bridge.notes.is_empty());
    assert!(scene.bridge.fell_back.is_empty());
}

/// Where the fallback cannot take over a failure, the house answers
/// and the failure says both why the mind failed and why the fallback
/// did not; it is still the mind unavailable, never a limit.
#[tokio::test]
async fn a_fallback_that_cannot_take_over_a_failure_leaves_it_to_the_house() {
    let refusal = "the day's cap of $3.00 is spent";
    let mut scene = Scene::new(vec![Move::Unavailable], Reserve::Refuses(refusal));
    scene.ask(2).await;
    let steps = scene.back().await;
    let made = answers(&steps);
    assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
    scene.carry(steps).await;
    assert_eq!(
        failures(&mut scene.core),
        [format!(
            "the mind is unavailable: {FAILED}; the fallback backup could not take over: \
             {refusal}"
        )]
    );
    assert_eq!(
        *scene.armed.lock().unwrap(),
        [Limit {
            turn: TURN,
            why: FAILED.into(),
            lifts_in: None,
            failed: true,
            by: None,
        }]
    );
    assert_eq!(
        scene.bridge.notes,
        [serde_json::json!({
            "fallback_refused": { "profile": "backup", "why": refusal },
        })]
    );
    assert!(scene.bridge.fallbacks.is_empty(), "it stands by no longer");
    assert!(scene.bridge.fell_back.is_empty());
    let stats = scene.core.stats();
    assert_eq!((stats.answered.house, stats.fallbacks.unavailable), (1, 1));
}

/// A mind that declines a question is not down: the house answers that
/// one, the fallback is not armed and stands by still, and the mind
/// answers the next.
#[tokio::test]
async fn a_declined_question_is_the_house_s_and_the_fallback_stands_by() {
    let mut scene = Scene::new(
        vec![Move::Declined, Move::Choose(ManaColor::Red)],
        Reserve::Takes(vec![Move::Choose(ManaColor::Green)]),
    );
    scene.ask(2).await;
    let made = scene.answer().await;
    assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
    assert!(scene.armed.lock().unwrap().is_empty());
    assert!(!scene.bridge.fallbacks.is_empty(), "it stands by still");
    scene.ask(3).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
    );
    assert!(scene.fallback.lock().unwrap().asked.is_empty());
    assert!(scene.bridge.fell_back.is_empty());
}

/// What the operator reads of a limit is the words a seat agent holds
/// the profile back on, with when it lifts; never the provider's own.
#[test]
fn a_limit_is_said_in_the_words_a_seat_agent_reads() {
    let limit = Limit {
        turn: 3,
        why: "429: slow down".into(),
        lifts_in: Some(Duration::from_secs(120)),
        failed: false,
        by: None,
    };
    assert_eq!(
        limit.said(),
        "the primary is rate limited (it lifts in 120 s)"
    );
    let unsaid = Limit {
        lifts_in: None,
        ..limit
    };
    assert_eq!(unsaid.said(), LIMITED);
}

/// A stand-in table on this machine: a socket to it, and every frame it
/// is sent, decoded.
async fn wired() -> (Socket, tokio::sync::mpsc::UnboundedReceiver<Envelope>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (frames, table) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        while let Some(Ok(message)) = socket.next().await {
            if let Message::Binary(bytes) = message {
                let _ = frames.send(Envelope::decode(&bytes[..]).unwrap());
            }
        }
    });
    let (socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}"))
        .await
        .unwrap();
    (socket, table)
}

/// A fallback that took over and cannot answer either hands the game on
/// down the chain (owner, 31.10.2026: the same model elsewhere first, then
/// the next most suitable): the primary is limited and `fb1` takes over;
/// later `fb1` fails, and `fb2` is asked that very question under the same
/// deadline, declared to the table first, and plays on. Nobody hands the
/// game back, the house answers nothing, and each fallback's answers are
/// its own.
#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one game down a chain, read top to bottom"
)]
async fn a_chain_hands_the_game_on_from_fallback_to_fallback() {
    let mut scene = Scene::chained(
        vec![Move::Choose(ManaColor::Red), Move::Limited],
        vec![
            Link::Takes {
                label: "fb1",
                model: "model-fb1",
                moves: vec![Move::Choose(ManaColor::Green), Move::Unavailable],
            },
            Link::Takes {
                label: "fb2",
                model: "model-fb2",
                moves: vec![Move::Choose(ManaColor::Red)],
            },
        ],
    );
    scene.ask(2).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
    );
    // The primary is limited: `fb1` takes the question over.
    scene.ask(3).await;
    assert!(scene.back().await.is_empty());
    assert_eq!(declaring(&scene.bridge), ["model-fb1"]);
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    // `fb1` fails at the next: `fb2` is asked the same question.
    scene.ask(4).await;
    let thinking = scene.bridge.thinking.as_ref().unwrap();
    let (question, deadline) = (thinking.question, thinking.deadline);
    let steps = scene.back().await;
    assert!(steps.is_empty(), "the core hears nothing of it: {steps:?}");
    let thinking = scene.bridge.thinking.as_ref().expect("asked of fb2");
    assert_eq!((thinking.question, thinking.deadline), (question, deadline));
    assert_eq!(declaring(&scene.bridge), ["model-fb2"], "declared first");
    scene.carry(steps).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
    );
    scene.ask(5).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Red))]
    );

    assert_eq!(*scene.armed_by.lock().unwrap(), ["fb1", "fb2"]);
    let armed = scene.armed.lock().unwrap().clone();
    assert_eq!(
        armed,
        [
            Limit {
                turn: TURN,
                why: "429: slow down".into(),
                lifts_in: Some(Duration::from_secs(30)),
                failed: false,
                by: None,
            },
            Limit {
                turn: TURN,
                why: FAILED.into(),
                lifts_in: None,
                failed: true,
                by: Some("fb1".into()),
            },
        ]
    );
    assert_eq!(
        armed[1].said(),
        format!("the fallback fb1 failed: {FAILED}")
    );
    assert_eq!(
        scene.bridge.notes,
        [
            serde_json::json!({"fallback": {
                "profile": "fb1", "turn": TURN, "cause": "limited", "why": "429: slow down",
            }}),
            serde_json::json!({"fallback": {
                "profile": "fb2", "turn": TURN, "cause": "failed", "why": FAILED,
            }}),
        ]
    );
    assert_eq!(
        scene.bridge.fell_back,
        [
            FellBack {
                label: "fb1".into(),
                turn: TURN,
                answered: 1,
            },
            FellBack {
                label: "fb2".into(),
                turn: TURN,
                answered: 2,
            },
        ],
        "each fallback's own answers"
    );
    let context = scene.core.context().unwrap().clone();
    let ended = [(context.game_id.clone(), context.seat.get())];
    assert_eq!(scene.primary.lock().unwrap().ended, ended);
    assert_eq!(scene.chain[0].lock().unwrap().ended, ended, "fb1 is done");
    assert!(scene.chain[1].lock().unwrap().ended.is_empty(), "fb2 plays");
    assert_eq!(scene.primary.lock().unwrap().asked.len(), 2);
    assert_eq!(scene.chain[0].lock().unwrap().asked.len(), 2);
    assert_eq!(scene.chain[1].lock().unwrap().asked.len(), 2);
    assert!(scene.bridge.fallbacks.is_empty(), "the chain is used up");
    let stats = scene.core.stats();
    assert_eq!(
        (stats.answered.mind, stats.answered.house, stats.asked),
        (4, 0, 4)
    );
    assert_eq!(stats.fallbacks.total(), 0, "no house answer, no failure");
    assert!(failures(&mut scene.core).is_empty());
}

/// A fallback that cannot answer the very question it was handed passes
/// it on at once: the primary is limited, `fb1` fails at it, and `fb2`
/// answers it, each within what is left of the one deadline.
#[tokio::test]
async fn a_fallback_that_fails_at_once_hands_the_same_question_on() {
    let mut scene = Scene::chained(
        vec![Move::Limited],
        vec![
            Link::Takes {
                label: "fb1",
                model: "model-fb1",
                moves: vec![Move::Unavailable],
            },
            Link::Takes {
                label: "fb2",
                model: "model-fb2",
                moves: vec![Move::Choose(ManaColor::Green)],
            },
        ],
    );
    let budget = scene.ask(2).await;
    let deadline = scene.bridge.thinking.as_ref().unwrap().deadline;
    assert!(scene.back().await.is_empty(), "the primary's limit");
    assert_eq!(scene.bridge.thinking.as_ref().unwrap().deadline, deadline);
    assert!(scene.back().await.is_empty(), "fb1's failure");
    assert_eq!(scene.bridge.thinking.as_ref().unwrap().deadline, deadline);
    assert_eq!(
        declaring(&scene.bridge),
        ["model-fb1", "model-fb2"],
        "each declared before the answer"
    );
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    let question = scene.primary.lock().unwrap().asked[0].question;
    for seen in &scene.chain {
        let seen = seen.lock().unwrap();
        assert_eq!(seen.asked.len(), 1);
        assert_eq!(seen.asked[0].question, question, "the same question");
        assert!(!seen.asked[0].retried);
        assert!(seen.asked[0].budget <= budget, "within what was left");
    }
    assert_eq!(
        scene
            .bridge
            .fell_back
            .iter()
            .map(|fell| (fell.label.as_str(), fell.answered))
            .collect::<Vec<_>>(),
        [("fb1", 0), ("fb2", 1)]
    );
    assert_eq!(scene.core.stats().answered.house, 0);
}

/// A fallback that cannot take over (its book refuses, its key is gone) is
/// passed over, the transcript saying why, and the next in the chain takes
/// the question over as the first would have.
#[tokio::test]
async fn a_fallback_that_cannot_take_over_is_passed_over_for_the_next() {
    let refusal = "the day's cap of $3.00 is spent";
    let mut scene = Scene::chained(
        vec![Move::Limited],
        vec![
            Link::Refuses {
                label: "fb1",
                why: refusal,
            },
            Link::Takes {
                label: "fb2",
                model: "model-fb2",
                moves: vec![Move::Choose(ManaColor::Green)],
            },
        ],
    );
    scene.ask(2).await;
    assert!(scene.back().await.is_empty(), "taken over");
    assert_eq!(declaring(&scene.bridge), ["model-fb2"]);
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    assert_eq!(*scene.armed_by.lock().unwrap(), ["fb1", "fb2"]);
    let armed = scene.armed.lock().unwrap().clone();
    assert_eq!(armed[0], armed[1], "both asked on the primary's limit");
    assert_eq!(armed[0].by, None);
    assert_eq!(
        scene.bridge.notes,
        [
            serde_json::json!({"fallback_refused": {"profile": "fb1", "why": refusal}}),
            serde_json::json!({"fallback": {
                "profile": "fb2", "turn": TURN, "cause": "limited", "why": "429: slow down",
            }}),
        ]
    );
    assert_eq!(
        scene.bridge.fell_back,
        [FellBack {
            label: "fb2".into(),
            turn: TURN,
            answered: 1,
        }]
    );
    assert_eq!(scene.core.stats().answered.house, 0);
    assert!(failures(&mut scene.core).is_empty());
}

/// Past the end of the chain the house answers, as without a fallback: a
/// limit of the last fallback that played hands the rest of the game to
/// the house, and its failure says why each fallback after it could not
/// take over.
#[tokio::test]
async fn past_the_end_of_the_chain_the_house_answers() {
    let mut scene = Scene::chained(
        vec![Move::Limited],
        vec![
            Link::Takes {
                label: "fb1",
                model: "model-fb1",
                moves: vec![Move::Choose(ManaColor::Green), Move::Limited],
            },
            Link::Refuses {
                label: "fb2",
                why: "set TEST_FB_KEY",
            },
        ],
    );
    scene.ask(2).await;
    assert_eq!(
        scene.answer().await,
        [(By::Mind, PlayerAction::ChooseColor(ManaColor::Green))]
    );
    scene.ask(3).await;
    let steps = scene.back().await;
    let house = steps.iter().position(|step| {
        matches!(step, Step::Send(Envelope { msg: Some(v1::envelope::Msg::SeatMind(mind)) })
            if mind.kind == v1::seat_mind::Kind::House as i32)
    });
    assert!(house.is_some(), "the house is declared: {steps:?}");
    assert!(matches!(&answers(&steps)[..], [(By::House, _)]));
    scene.carry(steps).await;
    assert_eq!(
        failures(&mut scene.core),
        [
            "the mind is rate limited: 429: slow down; the fallback fb2 could not take over: \
             set TEST_FB_KEY (it lifts in 30 s)"
        ]
    );
    let armed = scene.armed.lock().unwrap().clone();
    assert_eq!(armed.len(), 2);
    assert_eq!(armed[1].by.as_deref(), Some("fb1"), "fb1 was limited");
    assert_eq!(
        armed[1].said(),
        format!("{} (it lifts in 30 s)", fallback_limited("fb1"))
    );
    assert_eq!(
        scene.bridge.notes,
        [
            serde_json::json!({"fallback": {
                "profile": "fb1", "turn": TURN, "cause": "limited", "why": "429: slow down",
            }}),
            serde_json::json!({"fallback_refused": {"profile": "fb2", "why": "set TEST_FB_KEY"}}),
        ]
    );
    // The house has the rest of the game; nothing is armed again.
    scene.bridge.notes.clear();
    let steps = put_in_turn(&mut scene.core, 4, TURN);
    assert!(budget(&steps).is_none(), "no mind is asked");
    assert!(matches!(&answers(&steps)[..], [(By::House, _)]));
    scene.carry(steps).await;
    assert_eq!(scene.armed.lock().unwrap().len(), 2);
    assert!(scene.bridge.notes.is_empty());
    assert_eq!(
        scene.bridge.fell_back,
        [FellBack {
            label: "fb1".into(),
            turn: TURN,
            answered: 1,
        }]
    );
    assert!(scene.core.stats().housed);
}

/// A declined question is not a mind down, whoever plays: the fallback
/// that took over declines one, the house answers it, and the next in the
/// chain still stands by.
#[tokio::test]
async fn a_fallback_that_declines_a_question_keeps_the_chain_standing() {
    let mut scene = Scene::chained(
        vec![Move::Limited],
        vec![
            Link::Takes {
                label: "fb1",
                model: "model-fb1",
                moves: vec![Move::Choose(ManaColor::Green), Move::Declined],
            },
            Link::Takes {
                label: "fb2",
                model: "model-fb2",
                moves: vec![Move::Choose(ManaColor::Red)],
            },
        ],
    );
    scene.ask(2).await;
    scene.answer().await;
    scene.ask(3).await;
    let made = scene.answer().await;
    assert!(matches!(&made[..], [(By::House, _)]), "{made:?}");
    assert_eq!(*scene.armed_by.lock().unwrap(), ["fb1"]);
    assert_eq!(scene.bridge.fallbacks.len(), 1, "fb2 stands by still");
    assert!(scene.chain[1].lock().unwrap().asked.is_empty());
}

/// What a seat agent reads of a fallback that took over and is limited in
/// its turn names that fallback, never the primary: the profile held back
/// is the one that is limited.
#[test]
fn a_fallback_s_limit_names_the_fallback() {
    let limit = Limit {
        turn: 3,
        why: "429: slow down".into(),
        lifts_in: Some(Duration::from_secs(60)),
        failed: false,
        by: Some("fb1".into()),
    };
    assert_eq!(
        limit.said(),
        "the fallback fb1 is rate limited (it lifts in 60 s)"
    );
    assert!(!limit.said().contains(LIMITED));
    let failed = Limit {
        failed: true,
        lifts_in: None,
        ..limit
    };
    assert_eq!(failed.said(), "the fallback fb1 failed: 429: slow down");
}
