use super::*;
use crate::recording::Transport as _;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_tower::{
    cards::{Card, Stack},
    v2::{
        execution::{MAX_CHUNK_BYTES, MAX_RECORD_BYTES},
        package::{Build, Cohort, Origin},
        run::{Binding, Outcome},
    },
};
use prost::Message as _;
use std::time::{Duration, Instant};

mod recovery;

fn fixture() -> (Package, Catalog) {
    let card = baylee_cards::decks::by_name("Forest").unwrap();
    let catalog = Catalog {
        cards: vec![Card {
            card,
            price: 0,
            basic_land: true,
        }],
    };
    let build = Build {
        deck: vec![Stack { card, copies: 40 }],
        active: Vec::new(),
        potions: [0; 2],
    };
    let package = Package {
        schema: 2,
        cohort: Cohort::new(crate::variant::identity(), &catalog).unwrap(),
        stage: 1,
        opponent: Origin::House { archetype: 1 },
        builds: [build.clone(), build],
        shuffle_seed: 17,
        policy_seed: 19,
    };
    (package, catalog)
}

fn commit(table: &mut Table, record: &mut Vec<u8>, package: [u8; 32]) -> Vec<Envelope> {
    let mut revealed = Vec::new();
    while let Some(batch) = table.batch() {
        let mut next = batch.expected;
        for chunk in batch.chunks() {
            next = next.extended(package, MAX_RECORD_BYTES, chunk).unwrap();
            baylee_worker::frame::write(record, chunk, MAX_CHUNK_BYTES).unwrap();
        }
        let fence = batch.fence;
        revealed.extend(table.acknowledge(fence, next).unwrap());
        assert!(
            table.acknowledge(fence, next).unwrap().is_empty(),
            "duplicate ACK never repeats a frame"
        );
    }
    revealed
}

fn seat(message: v1::envelope::Msg) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
            seat: 0,
            envelope: Envelope { msg: Some(message) }.encode_to_vec().into(),
        })),
    }
}

fn play(table: &mut Table, action: &PlayerAction) -> Vec<Envelope> {
    table
        .handle(
            seat(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: "durable-test".into(),
                seat_token: String::new(),
                action: baylee_core::wire::encode(action),
            })),
            &[],
        )
        .unwrap()
}

#[test]
fn real_native_output_waits_for_exact_commit_and_terminal_record_independently_verifies() {
    let (package, catalog) = fixture();
    let digest = package.digest(&catalog).unwrap();
    let mut table = Table::new(
        "durable-test".into(),
        &package,
        &catalog,
        4,
        MAX_RECORD_BYTES,
        ["Player".into(), "Echo".into()],
    )
    .unwrap();
    let attach = Envelope {
        msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
            seat: 0,
            resync: false,
        })),
    };
    assert_eq!(table.handle(attach.clone(), &[]), Err(Refusal::Unavailable));
    let initial: Vec<_> = table
        .batch()
        .unwrap()
        .chunks()
        .map(<[u8]>::to_vec)
        .collect();
    assert_eq!(table.acknowledged(), Prefix::default());
    assert!(table.acknowledge(3, Prefix::default()).is_err());
    assert_eq!(
        table
            .batch()
            .unwrap()
            .chunks()
            .map(<[u8]>::to_vec)
            .collect::<Vec<_>>(),
        initial
    );
    let mut record = Vec::new();
    commit(&mut table, &mut record, digest);
    table.handle(attach, &[]).unwrap();
    commit(&mut table, &mut record, digest);
    table
        .handle(seat(v1::envelope::Msg::SeatReady(v1::SeatReady {})), &[])
        .unwrap();
    commit(&mut table, &mut record, digest);
    table.tell_time(table.entrance_deadline().unwrap()).unwrap();
    table.finish_entrance().unwrap();
    commit(&mut table, &mut record, digest);
    let me = PlayerId::new(0);
    for _ in 0..32 {
        if table.needs_pump() {
            table.pump(&[]).unwrap();
        } else if matches!(table.driver.session().unwrap().pending(), Pending::Priority { player, .. } if *player == me)
        {
            break;
        } else {
            let action = table.driver.session().unwrap().house_action(me).unwrap();
            play(&mut table, &action);
        }
        commit(&mut table, &mut record, digest);
    }
    assert!(play(&mut table, &PlayerAction::Concede).is_empty());
    assert!(!table.finished(), "terminal view cannot outrun persistence");
    assert!(table.batch().is_some());
    let revealed = commit(&mut table, &mut record, digest);
    assert!(table.finished());
    assert!(
        revealed
            .iter()
            .any(|frame| matches!(frame.msg, Some(v1::envelope::Msg::GameEnded(_))))
    );
    assert!(
        !revealed
            .iter()
            .any(|frame| matches!(frame.msg, Some(v1::envelope::Msg::GameRecordChunk(_))))
    );
    let binding = Binding {
        package: digest,
        fence: 4,
        prefix: table.acknowledged(),
    };
    let settlement = super::super::verification::verify_record(
        &package,
        &catalog,
        binding,
        record.as_slice(),
        baylee_gamehost::session::tower::ReplayBudget::default(),
        Instant::now() + Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(settlement.outcome, Outcome::Defeat);
}

#[test]
fn partial_out_of_order_and_foreign_acks_never_release_a_batch() {
    let mut gate = Barrier::new([7; 32], 4, MAX_RECORD_BYTES).unwrap();
    let frames = vec![Envelope {
        msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded::default())),
    }];
    let mut piece = vec![9; MAX_CHUNK_BYTES];
    piece.push(b'\n');
    assert!(gate.stage(vec![piece], frames.clone()).unwrap().is_empty());
    let batch = gate.batch().unwrap();
    let first_head = batch
        .expected
        .extended([7; 32], MAX_RECORD_BYTES, batch.chunks().next().unwrap())
        .unwrap();
    let last = first_head
        .extended([7; 32], MAX_RECORD_BYTES, b"\n")
        .unwrap();
    assert_eq!(gate.ack(4, first_head), Err(Refusal::Stale));
    assert_eq!(gate.ack(5, first_head), Err(Refusal::Stale));
    assert!(gate.waiting());
    assert_eq!(gate.prefix(), Prefix::default());
    assert_eq!(gate.ack(4, last).unwrap(), frames);
    assert!(!gate.waiting());
    assert!(gate.ack(4, last).unwrap().is_empty());
}

#[test]
fn exhausted_native_record_budget_emits_no_hand_result_or_false_ack() {
    let (package, catalog) = fixture();
    assert!(matches!(
        Table::new(
            "bounded".into(),
            &package,
            &catalog,
            1,
            1,
            ["Player".into(), "Echo".into()]
        ),
        Err(Refusal::Capacity)
    ));
    let mut capture = Capture::default();
    assert!(
        capture
            .chunk("g", 0, vec![0; capture::MAX_BATCH_BYTES + 1], false)
            .is_none()
    );
    assert_eq!(capture.take(), Err(Refusal::Capacity));
    let mut gate = Barrier::new([7; 32], 1, MAX_RECORD_BYTES).unwrap();
    assert_eq!(
        gate.stage(Vec::new(), vec![Envelope::default(); 4097]),
        Err(Refusal::Capacity)
    );
    assert_eq!(gate.prefix(), Prefix::default());
    assert!(gate.batch().is_none());
}
