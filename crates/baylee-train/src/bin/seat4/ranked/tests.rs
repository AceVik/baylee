//! The net's flight over a real socket, a table played by a script: to the
//! game's end, paced; handed over at a cut socket, a failed observation or
//! a first socket that opens mid-game; and the account key the pilot says.
use super::flight::{self, Flown, Pace, SOCKET_LOST};
use super::*;
use baylee_client_core::reconnect::Window;
use baylee_client_core::test_support::{ViewBuilder, statics};
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::win::{EndReason, GameResult, Victor};
use baylee_protocol::v1::{self, Envelope};
use baylee_seat::link::{Reporter, Socket, Table};
use baylee_train::features4::{SourceKind4, StepProvenance4};
use baylee_train::live4::LiveAnswer4;
use baylee_view::{PlayerView, SeatIdentity};
use futures_util::{SinkExt as _, StreamExt as _};
use prost::Message as _;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;

const ME: PlayerId = PlayerId::new(0);

/// A brain that keeps its hand, passes, names the first colour offered and
/// fails its observation number `fail_at` (counting from one).
#[derive(Default)]
struct Script {
    observed: usize,
    fail_at: Option<usize>,
}

impl Brain4 for Script {
    fn initialize(&mut self, _: PlayerId, _: &DeckList, _: &[Option<u8>]) -> anyhow::Result<()> {
        Ok(())
    }

    fn observe(&mut self, _: &PlayerView) -> anyhow::Result<()> {
        self.observed += 1;
        anyhow::ensure!(
            self.fail_at != Some(self.observed),
            "mock observation failed"
        );
        Ok(())
    }

    fn answer(
        &mut self,
        _: &PlayerView,
        pending: &Pending,
        _: &[Option<u8>],
    ) -> anyhow::Result<LiveAnswer4> {
        let action = match pending {
            Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
            Pending::Priority { .. } => PlayerAction::PassPriority,
            Pending::ChooseColor { options, .. } => PlayerAction::ChooseColor(options[0]),
            other => anyhow::bail!("the script has no answer to {other:?}"),
        };
        Ok(LiveAnswer4 {
            action,
            provenance: StepProvenance4 {
                source: SourceKind4::Net,
                ..StepProvenance4::default()
            },
            draw: None,
        })
    }
}

fn frame(msg: v1::envelope::Msg) -> Vec<u8> {
    Envelope { msg: Some(msg) }.encode_to_vec()
}

fn table() -> Vec<u8> {
    let mut game = statics(0);
    game.seats = (0..2)
        .map(|index| SeatIdentity {
            player: PlayerId::new(index),
            display_name: format!("Seat {index}"),
            is_ai: index == 0,
            guest: false,
            away: false,
            team: None,
        })
        .collect();
    frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
        game_id: game.game_id.clone(),
        view_version: game.view_version,
        statics: baylee_core::wire::encode(&game),
        more: Vec::new(),
    }))
}

fn view(seq: u64, turn: u32) -> Vec<u8> {
    let mut view = ViewBuilder::new(2).build();
    view.seq = seq;
    view.turn = turn;
    view.seats[0].life = 20 - i32::try_from(seq).unwrap();
    view.awaiting = Some(ME);
    frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
        game_id: "test-game".into(),
        seq,
        view_frame: baylee_view::ViewFrame::snapshot_of(&view),
        log: Vec::new(),
    }))
}

fn ask(seq: u64, pending: &Pending) -> Vec<u8> {
    frame(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
        game_id: "test-game".into(),
        seq,
        pending: baylee_core::wire::encode(pending),
        context: Vec::new(),
    }))
}

fn curtain() -> Vec<u8> {
    frame(v1::envelope::Msg::Curtain(v1::Curtain {}))
}

fn mulligan() -> Pending {
    Pending::Mulligan {
        player: ME,
        taken: 0,
        next_is_free: false,
        can_take: true,
    }
}

fn priority() -> Pending {
    Pending::Priority {
        player: ME,
        legal: Box::default(),
    }
}

fn colour() -> Pending {
    Pending::ChooseColor {
        player: ME,
        options: vec![ManaColor::Red, ManaColor::Green],
    }
}

fn game_over() -> Pending {
    Pending::GameOver(GameResult {
        winner: Some(Victor::Player(ME)),
        reason: EndReason::LastPlayerStanding,
    })
}

/// What the scripted table does next.
enum Step {
    /// Sends a frame.
    Send(Vec<u8>),
    /// Waits for the seat's next action and keeps it, with how long after
    /// the last frame sent it came.
    Answer,
    /// Drops the socket.
    Cut,
}

/// What the table heard: each action, and how long after the frame before
/// it; and every other envelope.
#[derive(Default)]
struct Heard {
    actions: Vec<(PlayerAction, Duration)>,
    others: Vec<Envelope>,
}

/// A table on a loopback socket that plays `steps` to the first seat that
/// dials, and then drops it.
async fn scripted(steps: Vec<Step>) -> (String, tokio::task::JoinHandle<Heard>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let served = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(tcp).await.unwrap();
        let mut heard = Heard::default();
        let mut sent = Instant::now();
        for step in steps {
            match step {
                Step::Send(bytes) => {
                    socket.send(Message::Binary(bytes.into())).await.unwrap();
                    sent = Instant::now();
                }
                Step::Answer => loop {
                    let Some(Ok(Message::Binary(bytes))) = socket.next().await else {
                        panic!("the seat left before it answered");
                    };
                    let envelope = Envelope::decode(&*bytes).unwrap();
                    if let Some(v1::envelope::Msg::PlayerAction(action)) = &envelope.msg {
                        let action = baylee_core::wire::decode(&action.action).unwrap();
                        heard.actions.push((action, sent.elapsed()));
                        break;
                    }
                    heard.others.push(envelope);
                },
                Step::Cut => return heard,
            }
        }
        // Whatever the seat still sends, until it closes.
        while let Some(Ok(message)) = socket.next().await {
            if let Message::Binary(bytes) = message {
                heard.others.push(Envelope::decode(&*bytes).unwrap());
            }
        }
        heard
    });
    (url, served)
}

/// A link to the scripted table, counting its dials.
struct Wire {
    url: String,
    dials: u32,
}

impl Table for Wire {
    fn hold_for(&mut self, _: Window) {}

    async fn connect(&mut self) -> anyhow::Result<Option<Socket>> {
        self.dials += 1;
        let (socket, _) = tokio_tungstenite::connect_async(self.url.as_str()).await?;
        Ok(Some(socket))
    }

    fn room_closed(&mut self) -> impl Future<Output = bool> + Send {
        std::future::ready(false)
    }

    fn selected_deck(&self) -> impl Future<Output = anyhow::Result<Option<DeckList>>> + Send {
        std::future::ready(Ok(None))
    }

    fn reporter(&self) -> Option<Reporter<'_>> {
        None
    }

    fn seat_secret(&self) -> &'static str {
        ""
    }

    fn dials(&self) -> u32 {
        self.dials
    }
}

const THINK: Duration = Duration::from_millis(400);

fn held(brain: Script) -> LiveSeat4<Script> {
    let mut core = LiveSeat4::new(brain, DeckList::default(), ME);
    core.require_opening();
    core
}

async fn fly(steps: Vec<Step>, brain: Script) -> (Flown, LiveSeat4<Script>, Heard, u32) {
    let (flown, core, heard, dials, _) = fly_kept(steps, brain).await;
    (flown, core, heard, dials)
}

/// [`fly`], with the lines the flight kept in its transcript.
async fn fly_kept(
    steps: Vec<Step>,
    brain: Script,
) -> (Flown, LiveSeat4<Script>, Heard, u32, Vec<serde_json::Value>) {
    let (url, served) = scripted(steps).await;
    let mut wire = Wire { url, dials: 0 };
    let mut core = held(brain);
    let pace = Pace {
        min_think: THINK,
        room_check: Duration::from_secs(5),
    };
    let mut transcript = baylee_seat::Transcript::memory();
    let flown = flight::fly(&mut wire, &mut core, pace, &mut transcript)
        .await
        .unwrap();
    let lines = transcript
        .lines()
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    (flown, core, served.await.unwrap(), wire.dials, lines)
}

/// The opening, as a table that just started shows it: the seat's table,
/// its first view, the curtain and the first question.
fn opening() -> Vec<Step> {
    vec![
        Step::Send(table()),
        Step::Send(view(1, 0)),
        Step::Send(curtain()),
        Step::Send(ask(1, &mulligan())),
        Step::Answer,
    ]
}

#[tokio::test]
async fn a_whole_game_ends_with_its_result_and_only_real_decisions_wait() {
    let mut steps = opening();
    steps.extend([
        Step::Send(view(2, 1)),
        Step::Send(ask(2, &priority())),
        Step::Answer,
        Step::Send(view(3, 1)),
        Step::Send(ask(3, &colour())),
        Step::Answer,
        Step::Send(view(4, 2)),
        Step::Send(ask(4, &game_over())),
    ]);
    let (flown, core, heard, dials, lines) = fly_kept(steps, Script::default()).await;
    assert_eq!(flown, Flown::Ended);
    assert_eq!(dials, 1);
    // The transcript keeps each answer as the bridge does, and the end.
    let answered: Vec<_> = lines.iter().filter(|l| l["event"] == "answered").collect();
    assert_eq!(answered.len(), 3, "{lines:?}");
    assert!(
        answered
            .iter()
            .all(|l| l["by"] == "mind" && l["model_ms"].is_u64())
    );
    assert_eq!(answered[1]["action"], serde_json::json!("PassPriority"));
    let over = lines
        .iter()
        .find(|l| l["event"] == "over")
        .expect("the end");
    assert!(over["result"].as_str().unwrap().contains("GameResult"));
    assert_eq!(over["turn"], 2);
    let actions: Vec<_> = heard.actions.iter().map(|(a, _)| a.clone()).collect();
    assert_eq!(
        actions,
        [
            PlayerAction::MulliganKeep,
            PlayerAction::PassPriority,
            PlayerAction::ChooseColor(ManaColor::Red)
        ]
    );
    let waited: Vec<_> = heard.actions.iter().map(|(_, after)| *after).collect();
    assert!(
        waited[0] >= THINK,
        "keeping a hand is a decision: {waited:?}"
    );
    assert!(waited[1] < THINK, "a pass goes at once: {waited:?}");
    assert!(
        waited[2] >= THINK,
        "naming a colour is a decision: {waited:?}"
    );
    assert!(core.game_result().is_some());
    assert_eq!(core.turn(), 2);
    assert_eq!(core.report().won, Some(true));
    let report = played(&core, ME, dials);
    assert_eq!(report.stats.outcome, Some(Outcome::Won));
    assert_eq!(report.stats.answered.mind, 3);
    assert_eq!(report.stats.turns, 2);
    assert!(
        heard
            .others
            .iter()
            .all(|e| !matches!(e.msg, Some(v1::envelope::Msg::SeatMind(_)))),
        "the net declares nothing to the table"
    );
}

#[tokio::test]
async fn a_cut_socket_hands_the_seat_over_without_dialling_again() {
    let mut steps = opening();
    steps.push(Step::Cut);
    let (flown, core, heard, dials) = fly(steps, Script::default()).await;
    assert_eq!(flown, Flown::HandOver(SOCKET_LOST.to_owned()));
    assert_eq!(dials, 1, "the net never resumes over missed views");
    assert_eq!(heard.actions.len(), 1);
    assert!(core.finished());
    assert!(core.report().failure.is_some());
}

#[tokio::test]
async fn a_failed_observation_hands_the_seat_over() {
    let mut steps = opening();
    steps.push(Step::Send(view(2, 1)));
    let brain = Script {
        fail_at: Some(2),
        ..Script::default()
    };
    let (flown, core, heard, _) = fly(steps, brain).await;
    let Flown::HandOver(why) = flown else {
        panic!("{flown:?}")
    };
    assert!(why.contains("observation failed"), "{why}");
    assert_eq!(heard.actions.len(), 1);
    assert!(core.finished());
}

#[tokio::test]
async fn a_first_socket_that_opens_mid_game_hands_over_before_any_answer() {
    // A restarted runner's first socket: the latest view, the curtain, the
    // question waiting.
    let steps = vec![
        Step::Send(table()),
        Step::Send(view(40, 6)),
        Step::Send(curtain()),
        Step::Send(ask(40, &priority())),
    ];
    let (flown, core, heard, _) = fly(steps, Script::default()).await;
    let Flown::HandOver(why) = flown else {
        panic!("{flown:?}")
    };
    assert!(why.contains("opening"), "{why}");
    assert!(heard.actions.is_empty(), "nothing is answered on a guess");
    assert!(core.finished());
}

/// A pilot as [`NetPilot::new`] makes one, without a model.
fn pilot(label: &str) -> NetPilot {
    NetPilot {
        model: PathBuf::from("/nowhere"),
        label: "r1-4eb".into(),
        sha256: "0".repeat(64),
        profile: profile_for(label).unwrap(),
        catalog: CatalogPolicy4::Drift,
        unread: compat::Unread::default(),
        broken: Mutex::new(None),
    }
}

#[test]
fn the_pilot_flies_a_net_account_under_its_stable_key_and_declares_nothing() {
    use baylee_account::ranked::ai::{AiBadge, Settings};
    let pilot = pilot("baylee");
    assert_eq!(pilot.kind(), AiKind::Net);
    assert_eq!(pilot.profile(), "net:baylee");
    assert_eq!(pilot.declared(), SeatMind::default(), "no borrowed kind");
    let settings = |kind, profile: &str| Settings {
        badge: AiBadge {
            kind,
            label: "Baylee AI".into(),
        },
        profile: profile.into(),
        enabled: true,
        max_games: 1,
    };
    assert!(baylee_seat::ranked::fits(&pilot, &settings(AiKind::Net, "net:baylee")).is_ok());
    assert!(baylee_seat::ranked::fits(&pilot, &settings(AiKind::House, "net:baylee")).is_err());
    assert!(baylee_seat::ranked::fits(&pilot, &settings(AiKind::Net, "net:r1-4eb")).is_err());
    for refused in ["", "Baylee", "v6s-league/r1-4eb", "baylee ai"] {
        assert!(profile_for(refused).is_err(), "{refused:?}");
    }
    assert!(pilot.told().contains("r1-4eb"));
    assert!(pilot.told().contains("drift"));
}

#[test]
fn a_pilot_whose_model_would_not_load_makes_no_offer() {
    let pilot = pilot("baylee");
    assert!(pilot.ready().is_ok());
    pilot.break_with("the net's model could not be loaded");
    assert!(pilot.ready().unwrap_err().contains("could not be loaded"));
}

#[test]
fn the_ranked_net_plays_the_measured_policy() {
    use clap::Parser as _;
    let default = super::super::Args::try_parse_from(["seat4", "--model", "/model"]).unwrap();
    assert_eq!(
        default.policy(),
        super::super::Policy::measured(),
        "the ranked pilot plays what a seat with no policy flags plays"
    );
}
