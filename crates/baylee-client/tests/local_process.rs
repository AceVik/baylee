//! Real headless client subprocesses: no graphics, services or user settings.
#![cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
use baylee_cards_pool as _;
use baylee_client::host::{DuelHost, HostMessage, process::ProcessHost};
use baylee_core::{
    card_metadata::{DeckChoices, RulesMetadata},
    ids::{PlayerId, PrintRef},
    preset::{
        DeckEntry, Finish, FormatId, GamePreset, HouseRules, LoopPolicy, PrintInfo,
        SeatCapabilities, SeatController, SeatSpec,
    },
};
use baylee_engine::choice::{Pending, PlayerAction};
use std::{
    ops::{Deref, DerefMut},
    process::Command,
    time::{Duration, Instant},
};

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_baylee-client"))
}

pub(super) fn preset(policy: LoopPolicy) -> GamePreset {
    let card = baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
        .unwrap()
        .index;
    let seat = SeatSpec {
        controller: SeatController::Open,
        capabilities: SeatCapabilities::default(),
        deck: (0..60)
            .map(|_| DeckEntry {
                card,
                print: PrintRef::new(0),
                rule_print: None,
                choices: DeckChoices::default(),
            })
            .collect(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        rules: RulesMetadata::default(),
        format: FormatId::Freeform,
        seed: 11,
        house_rules: HouseRules {
            loop_policy: policy,
            ..HouseRules::default()
        },
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![seat.clone(), seat],
    }
}

/// Cleanup runs on assertion failure too; only this test's owned process is observed.
struct Owned(Option<ProcessHost>);
impl Deref for Owned {
    type Target = ProcessHost;
    fn deref(&self) -> &ProcessHost {
        self.0.as_ref().unwrap()
    }
}
impl DerefMut for Owned {
    fn deref_mut(&mut self) -> &mut ProcessHost {
        self.0.as_mut().unwrap()
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        let pid = self.0.as_ref().and_then(ProcessHost::worker_id);
        drop(self.0.take());
        if let Some(pid) = pid {
            let mut system = sysinfo::System::new().unwrap();
            let pid = sysinfo::Pid::from_u32(pid);
            let until = Instant::now() + Duration::from_secs(10);
            loop {
                system.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::Some(&[pid]),
                    true,
                    sysinfo::ProcessRefreshKind::nothing(),
                );
                if system.process(pid).is_none() {
                    return;
                }
                if Instant::now() >= until {
                    assert!(
                        std::thread::panicking(),
                        "owned local worker was not reaped"
                    );
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn settled(host: &mut ProcessHost) -> Vec<HostMessage> {
    // A deadline and not a wait: the unoptimised worker starts in about
    // thirteen seconds beside the rest of the suite, which fifteen did not
    // always cover.
    let until = Instant::now() + Duration::from_secs(60);
    let mut messages = Vec::new();
    loop {
        messages.extend(host.poll());
        if !host.busy() {
            return messages;
        }
        assert!(
            Instant::now() < until,
            "worker did not settle: {messages:?}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn concrete_local_variants_play_resume_and_reap_without_opening_user_settings() {
    assert!(!baylee_client::settings::store_is_open());
    let seat = PlayerId::new(0);
    for policy in [
        LoopPolicy::Off,
        LoopPolicy::RunOnceThenBreak,
        LoopPolicy::CompRulesDraw,
    ] {
        let mut host = Owned(Some(
            ProcessHost::launch(command(), &preset(policy), seat, &["You", "Guest"]).unwrap(),
        ));
        let opening = settled(&mut host);
        assert!(
            opening.iter().any(|m| matches!(m, HostMessage::Choice(_))),
            "{opening:?}"
        );
        assert_eq!(host.own_deck().len(), 60);
        host.submit(PlayerAction::MulliganKeep);
        settled(&mut host);
        let point = host.resume_point().unwrap();
        let record = host.local_record().unwrap().to_vec();
        drop(host);

        // A Tower pack must never silently resume through this ordinary
        // desktop adapter just because both disable loop detection.
        if policy == LoopPolicy::Off {
            let end = record.iter().position(|byte| *byte == b'\n').unwrap();
            let mut header: serde_json::Value = serde_json::from_slice(&record[..end]).unwrap();
            let tower = baylee_protocol::execution::Identity::native(
                baylee_protocol::execution::Variant::Tower,
                baylee_build::short(),
                baylee_build::COMMIT,
                19,
            );
            header["execution"] = serde_json::to_value(tower).unwrap();
            let mut changed = serde_json::to_vec(&header).unwrap();
            changed.extend_from_slice(&record[end..]);
            assert!(
                ProcessHost::resume(
                    command(),
                    changed,
                    &point.record,
                    seat,
                    &point.names,
                    &point.hash
                )
                .is_err()
            );
        }

        let mut invalid = Owned(Some(
            ProcessHost::resume(
                command(),
                record.clone(),
                &point.record,
                seat,
                &point.names,
                "wrong hash",
            )
            .unwrap(),
        ));
        let refused = settled(&mut invalid);
        assert!(
            refused
                .iter()
                .any(|m| matches!(m, HostMessage::Interrupted)),
            "{refused:?}"
        );
        assert!(
            !refused
                .iter()
                .any(|m| matches!(m, HostMessage::View(..) | HostMessage::Static(_)))
        );
        assert!(invalid.resume_point().is_none());
        drop(invalid);

        let mut resumed = Owned(Some(
            ProcessHost::resume(
                command(),
                record.clone(),
                &point.record,
                seat,
                &point.names,
                &point.hash,
            )
            .unwrap(),
        ));
        let messages = settled(&mut resumed);
        assert!(
            !messages
                .iter()
                .any(|m| matches!(m, HostMessage::Interrupted | HostMessage::Failed(_))),
            "{messages:?}"
        );
        assert_eq!(resumed.resume_point().unwrap().hash, point.hash);
        assert_eq!(resumed.local_record().unwrap(), record);
        resumed.submit(PlayerAction::Concede);
        let ended = settled(&mut resumed);
        assert!(
            ended.iter().any(
                |m| matches!(m, HostMessage::Choice(p) if matches!(**p, Pending::GameOver(_)))
            ),
            "{ended:?}"
        );
        assert!(resumed.local_record().unwrap().starts_with(&record));
    }
}

#[test]
fn excess_local_commands_close_technically_and_never_create_a_winner() {
    let mut host = Owned(Some(
        ProcessHost::launch(
            command(),
            &preset(LoopPolicy::Off),
            PlayerId::new(0),
            &["You", "Guest"],
        )
        .unwrap(),
    ));
    settled(&mut host);
    // No renderer consumption: the bounded command/response queues must close
    // cleanly, without blocking this thread or quietly losing an action.
    for _ in 0..100 {
        host.submit(PlayerAction::PassPriority);
    }
    let messages = settled(&mut host);
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, HostMessage::Interrupted))
    );
    assert!(
        !messages
            .iter()
            .any(|m| matches!(m, HostMessage::Choice(p) if matches!(**p, Pending::GameOver(_))))
    );
    assert!(host.resume_point().is_none());
}
