//! Actual replay helpers must preserve every perspective, check and log line.
#![cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
use baylee_client::host::{DuelHost, LocalHost, process::projection::Projection};
use baylee_client_core::replay::{Header, Perspective, Source, Verify, tape_perspective};
use baylee_core::{ids::PlayerId, preset::LoopPolicy};
use baylee_engine::{choice::PlayerAction, engine::NoLoopDetection};
use std::{
    process::Command,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

fn record(policy: LoopPolicy) -> Vec<u8> {
    let preset = super::local_process::preset(policy);
    let mut host: Box<dyn DuelHost> = match policy {
        LoopPolicy::Off => Box::new(
            LocalHost::<NoLoopDetection>::with_detection(
                &preset,
                PlayerId::new(0),
                &["One", "Two"],
            )
            .unwrap(),
        ),
        _ => Box::new(LocalHost::new(&preset, PlayerId::new(0), &["One", "Two"]).unwrap()),
    };
    host.poll();
    host.submit(PlayerAction::MulliganKeep);
    host.poll();
    host.submit(PlayerAction::Concede);
    host.poll();
    host.local_record().unwrap().to_vec()
}

fn header(perspective: Perspective) -> Header {
    Header {
        tape: baylee_client_core::replay::TAPE_VERSION,
        view: baylee_view::VIEW_VERSION,
        build: baylee_build::short().into(),
        source: Source::Local,
        perspective: tape_perspective(perspective),
        seat: perspective.seat().get(),
        game: "synthetic".into(),
        recorded_by: String::new(),
        own: true,
        hands: Vec::new(),
    }
}

#[test]
fn contained_replays_equal_native_projection_for_every_policy_and_perspective() {
    for policy in [
        LoopPolicy::Off,
        LoopPolicy::RunOnceThenBreak,
        LoopPolicy::CompRulesDraw,
    ] {
        let record = record(policy);
        for perspective in [
            Perspective::Seat(PlayerId::new(0)),
            Perspective::Public,
            Perspective::AllHands(PlayerId::new(1)),
        ] {
            let progress = AtomicU8::new(0);
            let cancel = AtomicBool::new(false);
            let names = vec!["One".into(), "Two".into()];
            let expected = baylee_client::replay::local::project(
                &record,
                names.clone(),
                header(perspective),
                perspective,
                Verify::Stop,
                &progress,
                &cancel,
            )
            .unwrap();
            let actual = Projection {
                record: record.clone(),
                names,
                header: header(perspective),
                verify: Verify::Stop,
            }
            .with_command(
                Command::new(env!("CARGO_BIN_EXE_baylee-client")),
                &progress,
                &cancel,
            )
            .unwrap();
            assert_eq!(actual, expected, "{policy:?}, {perspective:?}");
            assert_eq!(progress.load(Ordering::Relaxed), 100);
            assert!(actual.over().is_some());
        }
    }
}
