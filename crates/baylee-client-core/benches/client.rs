//! What the client spends outside the renderer, per call: one second of the
//! score, a busy table's view decoded from the wire, a felt's vein table, a
//! table laid out for an arrangement (what every rebuild of the board pays).
//! `docs/perf-client.md` has the numbers.
//!
//! `cargo bench -p baylee-client-core --features test-support --bench client`
#![allow(missing_docs)]

use baylee_client_core::feltveins::VeinTable;
use baylee_client_core::music::{RATE, Scene, ScoreControl, ScoreRequest, Tune};
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_view::PlayerView;
use criterion::{Criterion, criterion_group, criterion_main};
use std::{hint::black_box, sync::Arc};

fn music(c: &mut Criterion) {
    // Each tune is settled into the score first: the opening seconds play
    // fewer voices.
    c.bench_function("music/one_second/frame_by_frame", |b| {
        let mut tune = Tune::new();
        for _ in 0..RATE * 4 {
            tune.frame();
        }
        b.iter(|| {
            for _ in 0..RATE {
                black_box(tune.frame());
            }
        });
    });
    c.bench_function("music/one_second/stream", |b| {
        let mut tune = Tune::new();
        for _ in 0..RATE * 4 {
            tune.frame();
        }
        b.iter(|| {
            for _ in 0..RATE * 2 {
                black_box(tune.next());
            }
        });
    });
    // One second of each scene as the audio thread pulls it, settled first.
    let table = |tension: f32| ScoreRequest {
        scene: Scene::Table,
        tension,
        own_turn: true,
        ..ScoreRequest::default()
    };
    for (name, request) in [
        (
            "lobby",
            ScoreRequest {
                scene: Scene::Lobby,
                ..ScoreRequest::default()
            },
        ),
        ("calm", table(0.05)),
        ("tension", table(0.6)),
        ("climax", table(0.95)),
    ] {
        c.bench_function(&format!("music/one_second/{name}"), |b| {
            let control = Arc::new(ScoreControl::default());
            control.set(request);
            let mut tune = Tune::with_control(control);
            for _ in 0..RATE * 12 {
                tune.next();
            }
            b.iter(|| {
                for _ in 0..RATE * 2 {
                    black_box(tune.next());
                }
            });
        });
    }
}

/// A busy four-seat view: twenty permanents a seat.
fn busy_view() -> PlayerView {
    let mut builder = ViewBuilder::new(4);
    for seat in 0..4u8 {
        let base = u32::from(seat) * 100;
        builder = builder.with_battlefield(
            seat,
            (base..base + 20).map(|slot| token(slot, seat, "Soldier", 2, 2)),
        );
    }
    builder.build()
}

fn view(c: &mut Criterion) {
    let json = serde_json::to_vec(&busy_view()).expect("a view serializes");
    eprintln!("a busy four-seat view is {} bytes of JSON", json.len());
    if let Ok(path) = std::env::var("BAYLEE_BENCH_DUMP") {
        std::fs::write(path, &json).expect("dump");
    }
    let view = busy_view();
    c.bench_function("view/encode/four_seats_busy", |b| {
        b.iter(|| black_box(serde_json::to_vec(black_box(&view)).expect("encodes")));
    });
    c.bench_function("view/encode_presized/four_seats_busy", |b| {
        b.iter(|| {
            let mut out = Vec::with_capacity(json.len() + json.len() / 8);
            serde_json::to_writer(&mut out, black_box(&view)).expect("encodes");
            black_box(out)
        });
    });
    c.bench_function("view/decode/four_seats_busy", |b| {
        b.iter(|| {
            black_box(serde_json::from_slice::<PlayerView>(black_box(&json)).expect("decodes"))
        });
    });
}

fn felt(c: &mut Criterion) {
    c.bench_function("felt/vein_table/duel", |b| {
        b.iter(|| {
            black_box(VeinTable::for_table(
                [38.0, 24.0],
                [131.5, 47.25, 64.0, 128.0],
            ))
        });
    });
    c.bench_function("felt/vein_table/eight_seats", |b| {
        b.iter(|| {
            black_box(VeinTable::for_table(
                [190.0, 76.0],
                [131.5, 47.25, 64.0, 128.0],
            ))
        });
    });
}

/// A table laid out as `rebuild_board` lays it, on a laptop's canvas: the
/// ellipse-or-frame `seated` and the arrangements that search a shape.
fn layout(c: &mut Criterion) {
    use baylee_client_core::layout::{Arrangement, Seat, TableLayout};
    use baylee_core::ids::PlayerId;
    for n in [4_u8, 6, 8] {
        let seats: Vec<Seat> = (0..n).map(|i| Seat::alone(PlayerId::new(i))).collect();
        for (name, arrangement) in [
            ("ring", Arrangement::Ring),
            ("upright_ring", Arrangement::UprightRing),
            ("spotlight", Arrangement::Spotlight),
        ] {
            c.bench_function(&format!("layout/{name}/{n}_seats"), |b| {
                b.iter(|| {
                    black_box(TableLayout::arranged(
                        black_box(&seats),
                        1708.0 / 860.0,
                        arrangement,
                        None,
                    ))
                });
            });
        }
    }
}

criterion_group!(benches, music, view, felt, layout);
criterion_main!(benches);
