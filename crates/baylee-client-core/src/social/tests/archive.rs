//! Archive imports keep complete logs and never import another device's protocol state.
use crate::social::{
    Body, ChatDisk, ChatPrefs, Effect, Me, MemoryDisk, Social, disk, history, vault,
};
use baylee_protocol::social::{Conv, Person};
use baylee_seal::{DeviceSecrets, RestKind, at_rest_open, at_rest_seal, testkit::Seeded};

const CONV: &str = "dm:0012:af03";
const NOW: u64 = 1_000_000;

fn record(index: u32) -> history::Record {
    history::Record {
        id: format!("0194adcc-0000-7000-8000-{index:012x}"),
        at: 1000 + u64::from(index),
        from: Person {
            tag: "0012".into(),
            handle: "Bob#0012".into(),
        },
        mine: false,
        body: Body {
            text: format!("private line {index}"),
            ..Body::default()
        },
        evidence: None,
    }
}

fn archive(device: &DeviceSecrets, range: std::ops::Range<u32>, rng: &mut Seeded) -> MemoryDisk {
    let mut disk = MemoryDisk::default();
    let state = serde_json::json!({"v":1,"device_id":device.device_id(),"convs":[{"id":CONV,"title":"Archived Bob"}],
        "registered":true,"seen":{"do-not-import":999},"acked":999,"keys":"must never be deserialized"});
    disk.write(
        disk::STATE_FILE,
        &at_rest_seal(
            device.vault_key(),
            RestKind::State,
            disk::STATE_FILE,
            &serde_json::to_vec(&state).unwrap(),
            rng,
        )
        .unwrap(),
    )
    .unwrap();
    let id: Conv = CONV.parse().unwrap();
    for index in range {
        disk.append(
            &history::file_path(device.vault_key(), &id),
            &history::seal_record(device.vault_key(), &id, &record(index), rng).unwrap(),
        )
        .unwrap();
    }
    disk
}

fn start(device: &DeviceSecrets, days: u16, rng: &mut Seeded) -> (Social, MemoryDisk) {
    let (social, effects) = Social::start(
        Me {
            tag: "af03".into(),
            handle: "Alice#af03".into(),
            guest: false,
            gateway: "https://gw.test".into(),
        },
        None,
        Some(vault::encode_secrets(device).to_string()),
        ChatPrefs {
            history_days: days,
            ..ChatPrefs::default()
        },
        NOW,
        rng,
    );
    let mut disk = MemoryDisk::default();
    persist(&effects, &mut disk);
    (social, disk)
}

fn persist(effects: &[Effect], disk: &mut MemoryDisk) {
    for effect in effects {
        if let Some(result) = disk::apply(effect, disk) {
            result.unwrap();
        }
    }
}

fn state(device: &DeviceSecrets, disk: &MemoryDisk) -> serde_json::Value {
    serde_json::from_slice(
        &at_rest_open(
            device.vault_key(),
            RestKind::State,
            disk::STATE_FILE,
            &disk.read(disk::STATE_FILE).unwrap(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn imported_effects(effects: &[Effect]) {
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, Effect::SaveState(_) | Effect::RewriteHistory { .. }))
    );
}

#[test]
fn archives_merge_full_logs_across_devices_idempotently_without_protocol_state() {
    let mut rng = Seeded::new(51);
    let old = DeviceSecrets::generate(&mut rng);
    let active = DeviceSecrets::generate(&mut rng);
    let another = DeviceSecrets::generate(&mut rng);
    let (mut social, mut current) = start(&active, 30, &mut rng);
    let before = state(&active, &current);
    let local = archive(&active, 0..600, &mut rng);
    let id: Conv = CONV.parse().unwrap();
    let path = history::file_path(active.vault_key(), &id);
    current.write(&path, &local.read(&path).unwrap()).unwrap();
    let first = archive(&old, 300..950, &mut rng);
    let second = archive(&another, 900..1000, &mut rng);
    for (index, (device, archived)) in [(&old, &first), (&another, &second), (&old, &first)]
        .into_iter()
        .enumerate()
    {
        let effects = social
            .restore_archive(device, archived, &current, NOW, &mut rng)
            .unwrap();
        imported_effects(&effects);
        if index == 2 {
            assert!(
                effects.is_empty(),
                "an unchanged, authenticated archive must not reencrypt or upload history/state"
            );
        }
        persist(&effects, &mut current);
    }
    let loaded = history::read(
        active.vault_key(),
        &history::file_name(active.vault_key(), &id),
        &current.read(&path).unwrap(),
    );
    assert_eq!(loaded.skipped, 0);
    assert_eq!(
        loaded.records.len(),
        1000,
        "old records outside the UI window survive"
    );
    assert_eq!(loaded.records.first().unwrap().id, record(0).id);
    assert_eq!(loaded.records.last().unwrap().id, record(999).id);
    assert_eq!(social.conversation(&id).unwrap().lines.len(), 500);
    assert_eq!(social.device_id(), active.device_id());
    let after = state(&active, &current);
    for field in [
        "device_id",
        "registered",
        "seen",
        "keys",
        "own_devices",
        "status",
    ] {
        assert_eq!(
            before[field], after[field],
            "must preserve active device field {field}"
        );
    }
    assert!(current.files.values().all(|bytes| {
        !bytes
            .windows(b"private line".len())
            .any(|window| window == b"private line")
    }));
}

#[test]
fn damaged_or_conflicting_archive_never_changes_the_live_machine() {
    let mut rng = Seeded::new(52);
    let old = DeviceSecrets::generate(&mut rng);
    let active = DeviceSecrets::generate(&mut rng);
    let good = archive(&old, 0..3, &mut rng);
    let (mut social, current) = start(&active, 30, &mut rng);
    let path = history::file_path(old.vault_key(), &CONV.parse().unwrap());
    for corruption in 0..7 {
        let mut bad = good.clone();
        match corruption {
            0 => {
                bad.files.get_mut(&path).unwrap().pop();
            }
            1 => bad.files.get_mut(&path).unwrap().extend([0, 0, 0]),
            2 => bad.files.get_mut(&path).unwrap()[8] ^= 1,
            3 => {
                bad.files.remove(disk::STATE_FILE);
            }
            4 => {
                bad.files
                    .insert("history/0000000000000000.log".into(), vec![0; 8]);
            }
            5 => {
                bad.files.insert("unexpected.bin".into(), vec![0; 8]);
            }
            _ => {
                let mut conflicting = record(1);
                conflicting.body.text = "conflicting contents".into();
                bad.append(
                    &path,
                    &history::seal_record(
                        old.vault_key(),
                        &CONV.parse().unwrap(),
                        &conflicting,
                        &mut rng,
                    )
                    .unwrap(),
                )
                .unwrap();
            }
        }
        let revision = social.revision();
        let conversations = social.conversations().to_vec();
        assert!(
            social
                .restore_archive(&old, &bad, &current, NOW, &mut rng)
                .is_err(),
            "corruption {corruption}"
        );
        assert_eq!(social.revision(), revision);
        assert_eq!(social.conversations(), conversations);
    }
    assert!(
        social
            .restore_archive(&active, &good, &current, NOW, &mut rng)
            .is_err()
    );
}

#[test]
fn own_device_archive_restores_missing_files_and_respects_history_off() {
    let mut rng = Seeded::new(53);
    let active = DeviceSecrets::generate(&mut rng);
    let source = archive(&active, 0..2, &mut rng);
    let (mut social, mut current) = start(&active, 30, &mut rng);
    assert!(
        social
            .restore_archive(&active, &MemoryDisk::default(), &current, NOW, &mut rng)
            .unwrap()
            .is_empty()
    );
    let effects = social
        .restore_archive(&active, &source, &current, NOW, &mut rng)
        .unwrap();
    imported_effects(&effects);
    persist(&effects, &mut current);
    assert_eq!(
        social
            .conversation(&CONV.parse().unwrap())
            .unwrap()
            .lines
            .len(),
        2
    );
    let (mut disabled, disk) = start(&active, 0, &mut rng);
    assert!(
        disabled
            .restore_archive(&active, &source, &disk, NOW, &mut rng)
            .unwrap()
            .is_empty()
    );
    assert!(disabled.conversations().is_empty());
}

#[test]
fn local_state_validation_rejects_authenticated_parser_failures() {
    let mut rng = Seeded::new(54);
    let device = DeviceSecrets::generate(&mut rng);
    let (_, disk) = start(&device, 30, &mut rng);
    let original = disk.read(disk::STATE_FILE).unwrap();
    assert!(Social::validate_stored_state(&device, &original).is_ok());
    let valid = state(&device, &disk);
    for field in ["json", "v", "device_id", "keys", "convs"] {
        let mut bad = valid.clone();
        match field {
            "v" => bad["v"] = serde_json::json!(2),
            "device_id" => bad["device_id"] = serde_json::json!("another-device"),
            "keys" => bad["keys"] = serde_json::json!("invalid protocol state"),
            "convs" => bad["convs"] = serde_json::json!([{"id":"invalid","title":"test"}]),
            _ => {}
        }
        let plain = if field == "json" {
            b"not json".to_vec()
        } else {
            serde_json::to_vec(&bad).unwrap()
        };
        let bytes = at_rest_seal(
            device.vault_key(),
            RestKind::State,
            disk::STATE_FILE,
            &plain,
            &mut rng,
        )
        .unwrap();
        assert!(
            Social::validate_stored_state(&device, &bytes).is_err(),
            "{field}"
        );
    }
}
