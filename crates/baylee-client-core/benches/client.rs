//! What the client spends outside the renderer, per call: one second of the
//! score, a busy table's view decoded from the wire, a felt's vein table.
//! `docs/perf-client.md` has the numbers.
//!
//! `cargo bench -p baylee-client-core --features test-support --bench client`
#![allow(missing_docs)]

use baylee_client_core::feltveins::VeinTable;
use baylee_client_core::music::{RATE, Tune};
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_view::PlayerView;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

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

criterion_group!(benches, music, view, felt);
criterion_main!(benches);
