use super::*;

mod clocks;
mod curtain;
mod record;
mod views;

/// The one clock a duel with one human seat can have running.
fn on_clock(runner: &EngineRunner) -> Option<Clock> {
    let clocks = runner.clocks();
    assert!(clocks.len() <= 1, "a duel with one human ran {clocks:?}");
    clocks.first().copied()
}

/// `ms` left on every clock running now, as the attach loop reads its
/// timers before handing a frame in.
fn reading(runner: &EngineRunner, ms: u32) -> Vec<(Clock, u32)> {
    runner
        .clocks()
        .into_iter()
        .map(|clock| (clock, ms))
        .collect()
}

/// The acceptance duel, seat 0 human and seat 1 the house.
fn duel(timeout_secs: u32) -> GamePreset {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/acceptance-decks.txt"),
    )
    .expect("acceptance deck file");
    let a = baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact");
    let b = baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory");
    let mut preset = baylee_cards::decks::preset_for(7, &a, &b);
    preset.seats[0].controller = baylee_core::preset::SeatController::Open;
    preset.house_rules.decision_timeout_secs = timeout_secs;
    preset
}

fn setup(runner: &mut EngineRunner, preset: &GamePreset) -> Vec<Envelope> {
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                game_id: "g1".to_string(),
                preset_json: serde_json::to_vec(preset).expect("preset serializes"),
                seat_names: vec!["You".to_string(), "House".to_string()],
            })),
        },
        &[],
    )
}

/// The same duel with a reconnect window a test can point at.
fn duel_window(decision_secs: u32, window_secs: u32) -> GamePreset {
    let mut preset = duel(decision_secs);
    preset.house_rules.reconnect_window_secs = window_secs;
    preset
}

fn detach(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatDetached(v1::SeatDetached { seat })),
        },
        &[],
    )
}

fn attach(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                seat,
                resync: false,
            })),
        },
        &[],
    )
}

/// A seat says it has drawn its table, the way its socket would.
fn ready(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
    };
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &[],
    )
}

/// A seat that attaches and has drawn its table: what every test about
/// an open table starts from (#256). The last human seat to sit raises
/// the curtain.
fn sit(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
    let mut out = attach(runner, seat);
    out.extend(ready(runner, seat));
    if let Some(at) = runner.entrance_deadline() {
        runner.tell_time(at);
        out.extend(runner.finish_entrance());
    }
    out
}

/// One seat's answer, wrapped the way its socket would deliver it.
fn act(runner: &mut EngineRunner, seat: u32, action: &PlayerAction) -> Vec<Envelope> {
    act_reading(runner, seat, action, &[])
}

/// [`act`], with what the attach loop read off its armed clocks.
fn act_reading(
    runner: &mut EngineRunner,
    seat: u32,
    action: &PlayerAction,
    remaining: &[(Clock, u32)],
) -> Vec<Envelope> {
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
            game_id: "g1".to_string(),
            seat_token: String::new(),
            action_json: serde_json::to_vec(action).expect("action serializes"),
        })),
    };
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        remaining,
    )
}

/// The `(seat, inner message)` of each frame, which is all a test cares
/// about — the gateway never looks inside one either.
fn frames(envelopes: &[Envelope]) -> Vec<(u32, v1::envelope::Msg)> {
    envelopes
        .iter()
        .filter_map(|env| match &env.msg {
            Some(v1::envelope::Msg::SeatFrame(frame)) => {
                let inner = <Envelope as prost::Message>::decode(&frame.envelope[..]).ok()?;
                Some((frame.seat, inner.msg?))
            }
            _ => None,
        })
        .collect()
}

// ------------------------------------------------- the decision clock

/// Both seats keep their opening hands, so that turn 1 has begun and one
/// seat is being asked.
fn keep_both(runner: &mut EngineRunner) {
    for seat in [0, 1] {
        act(runner, seat, &PlayerAction::MulliganKeep);
    }
    assert_eq!(
        runner.session().expect("a game").awaited().len(),
        1,
        "turn 1 has not begun"
    );
}

/// The acceptance duel with nobody automated, so the seat that is *not*
/// being asked is still able to say something.
fn two_humans(timeout_secs: u32) -> GamePreset {
    let mut preset = duel(timeout_secs);
    preset.seats[1].controller = baylee_core::preset::SeatController::Open;
    preset
}

/// Whose frames say what, in order, for the seats a test cares about.
fn said(out: &[Envelope]) -> Vec<(u32, &'static str)> {
    frames(out)
        .into_iter()
        .map(|(seat, msg)| {
            let what = match msg {
                v1::envelope::Msg::GameStatic(_) => "static",
                v1::envelope::Msg::StateDelta(_) => "view",
                v1::envelope::Msg::ChoiceRequest(_) => "question",
                v1::envelope::Msg::Curtain(_) => "curtain",
                v1::envelope::Msg::TableLoading(_) => "loading",
                v1::envelope::Msg::Error(_) => "error",
                _ => "other",
            };
            (seat, what)
        })
        .collect()
}

/// The log each frame of `seat`'s carries, in order: the index of its
/// first line and how many lines it holds.
fn logs(envelopes: &[Envelope], seat: u32) -> Vec<(u64, usize)> {
    frames(envelopes)
        .into_iter()
        .filter_map(|(s, msg)| match msg {
            v1::envelope::Msg::StateDelta(delta) if s == seat && !delta.log_json.is_empty() => {
                let tail: serde_json::Value = serde_json::from_slice(&delta.log_json).ok()?;
                Some((tail["from"].as_u64()?, tail["entries"].as_array()?.len()))
            }
            _ => None,
        })
        .collect()
}

/// The last view `seat` was sent.
///
/// The view rather than the field, so that "was sent no view at all" and
/// "was sent a view carrying no clock" stay two different answers: the
/// first is a missing `Option`, the second is a present view whose field
/// is `None`, and a test that conflated them would pass on silence.
fn last_view(envelopes: &[Envelope], seat: u32) -> Option<baylee_gamehost::PlayerView> {
    frames(envelopes)
        .into_iter()
        .filter_map(|(s, msg)| match msg {
            v1::envelope::Msg::StateDelta(delta) if s == seat => {
                serde_json::from_slice::<baylee_gamehost::PlayerView>(&delta.view_json).ok()
            }
            _ => None,
        })
        .next_back()
}

/// A seat's setting about itself, wrapped the way its socket sends it
/// (#265).
fn set(runner: &mut EngineRunner, seat: u32, setting: SeatSetting) -> Vec<Envelope> {
    let inner = Envelope {
        msg: Some(v1::envelope::Msg::SeatSetting(v1::SeatSettingMsg {
            setting_json: serde_json::to_vec(&setting).expect("setting serializes"),
        })),
    };
    runner.handle(
        Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat,
                envelope: prost::Message::encode_to_vec(&inner).into(),
            })),
        },
        &[],
    )
}
