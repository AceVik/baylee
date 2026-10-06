//! What one frame costs the engine process, transport included: building
//! the game, a seat's attach (the whole state), a resync, an answer that
//! moves the game, and the protobuf framing of what comes out.
//!
//! Run: `cargo bench -p baylee-engine-server --bench frames`
//! (numbers in `docs/perf-baseline.md`, server section).
#![allow(missing_docs)] // bench target: criterion's generated fns are self-describing

use baylee_core::preset::{GamePreset, SeatController};
use baylee_engine::choice::PlayerAction;
use baylee_engine_server::EngineRunner;
use baylee_protocol::v1::{self, Envelope};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use prost::Message as _;

/// The acceptance duel with both chairs human, as a room of two plays it.
fn duel() -> GamePreset {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/acceptance-decks.txt"),
    )
    .expect("acceptance deck file");
    let a = baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact");
    let b = baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory");
    let mut preset = baylee_cards::decks::preset_for(7, &a, &b);
    for seat in &mut preset.seats {
        seat.controller = SeatController::Open;
    }
    preset
}

fn setup_envelope(preset: &GamePreset) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
            game_id: "bench".to_string(),
            preset_json: serde_json::to_vec(preset).expect("preset serializes"),
            seat_names: vec!["A".to_string(), "B".to_string()],
        })),
    }
}

fn attached(seat: u32, resync: bool) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
            seat,
            resync,
        })),
    }
}

/// `inner`, as the seat's socket delivers it.
fn from_seat(seat: u32, inner: &Envelope) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
            seat,
            envelope: inner.encode_to_vec().into(),
        })),
    }
}

fn action(seat: u32, action: &PlayerAction) -> Envelope {
    from_seat(
        seat,
        &Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: "bench".to_string(),
                seat_token: String::new(),
                action_json: serde_json::to_vec(action).expect("action serializes"),
            })),
        },
    )
}

fn built(preset: &GamePreset) -> EngineRunner {
    let mut runner = EngineRunner::new();
    runner.handle(setup_envelope(preset), &[]);
    runner
}

/// Both seats attached and ready, the curtain up: a table at its first
/// question (both mulligans).
fn seated(preset: &GamePreset) -> EngineRunner {
    let mut runner = built(preset);
    for seat in 0..2 {
        runner.handle(attached(seat, false), &[]);
        let ready = Envelope {
            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
        };
        runner.handle(from_seat(seat, &ready), &[]);
    }
    if let Some(at) = runner.entrance_deadline() {
        runner.tell_time(at);
        runner.finish_entrance();
    }
    runner
}

fn bytes(out: &[Envelope]) -> usize {
    out.iter().map(prost::Message::encoded_len).sum()
}

fn benches(c: &mut Criterion) {
    let preset = duel();
    let setup = setup_envelope(&preset);

    // What the frames weigh, once, beside the times.
    let mut runner = built(&preset);
    let attach = runner.handle(attached(0, false), &[]);
    let mut runner = seated(&preset);
    let resync = runner.handle(attached(0, true), &[]);
    let keep = runner.handle(action(0, &PlayerAction::MulliganKeep), &[]);
    eprintln!(
        "frames: attach {} envelopes / {} B, resync {} / {} B, keep {} / {} B",
        attach.len(),
        bytes(&attach),
        resync.len(),
        bytes(&resync),
        keep.len(),
        bytes(&keep)
    );

    let mut group = c.benchmark_group("engine_server");
    group.bench_function("setup", |b| {
        b.iter_batched(
            || (EngineRunner::new(), setup.clone()),
            |(mut runner, setup)| runner.handle(setup, &[]),
            BatchSize::LargeInput,
        );
    });
    group.bench_function("attach", |b| {
        b.iter_batched(
            || built(&preset),
            |mut runner| runner.handle(attached(0, false), &[]),
            BatchSize::LargeInput,
        );
    });
    let mut runner = seated(&preset);
    group.bench_function("resync", |b| {
        b.iter(|| runner.handle(attached(0, true), &[]));
    });
    group.bench_function("answer_keep", |b| {
        b.iter_batched(
            || seated(&preset),
            |mut runner| runner.handle(action(0, &PlayerAction::MulliganKeep), &[]),
            BatchSize::LargeInput,
        );
    });
    group.finish();

    // The transport alone: what prost adds around the JSON the frames carry.
    let mut group = c.benchmark_group("protobuf");
    group.bench_function("encode_resync", |b| {
        b.iter(|| {
            resync
                .iter()
                .map(|e| e.encode_to_vec().len())
                .sum::<usize>()
        });
    });
    let wire: Vec<Vec<u8>> = resync.iter().map(prost::Message::encode_to_vec).collect();
    group.bench_function("decode_resync", |b| {
        b.iter(|| {
            wire.iter()
                .map(|w| Envelope::decode(&w[..]).expect("decodes").encoded_len())
                .sum::<usize>()
        });
    });
    // The gateway's half: a frame from the engine read off the socket as
    // `Bytes`, its seat frame's payload handed on.
    let wire: Vec<bytes::Bytes> = wire.into_iter().map(bytes::Bytes::from).collect();
    group.bench_function("gateway_unwrap_resync", |b| {
        b.iter(|| {
            wire.iter()
                .filter_map(|w| match Envelope::decode(w.clone()).ok()?.msg? {
                    v1::envelope::Msg::SeatFrame(f) => Some(f.envelope.len()),
                    _ => None,
                })
                .sum::<usize>()
        });
    });
    group.finish();
}

criterion_group!(frames, benches);
criterion_main!(frames);
