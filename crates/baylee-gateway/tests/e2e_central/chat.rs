//! Complete encrypted archive roundtrip across two OPAQUE sessions; the first device is offline.
#![forbid(unsafe_code)]
use baylee_account::{
    Account, Error,
    chat::{ChatDevice, DeviceKey},
};
use baylee_client_core::social::{
    Body, ChatDisk, ChatPrefs, Effect, Me, MemoryDisk, Social, disk, history, vault,
};
use baylee_protocol::social::{Conv, Person};
use baylee_seal::{DeviceSecrets, RestKind, at_rest_open, at_rest_seal, testkit::Seeded};

const CONVERSATION: &str = "dm:0012:af03";
const NOW: u64 = 1_000_000;

pub(super) struct Fixture {
    device: ChatDevice,
    gateway: String,
}

pub(super) fn first_device(account: &mut Account, gateway: &str) -> Fixture {
    let mut rng = Seeded::new(921);
    let keys = DeviceSecrets::generate(&mut rng);
    let device = ChatDevice {
        gateway: gateway.into(),
        tag: "af03".into(),
        installation: uuid::Uuid::now_v7().to_string(),
        device: keys.device_id().into(),
        key: DeviceKey::new(keys.to_bytes()),
        retired: false,
        history: None,
    };
    let mut archive = MemoryDisk::default();
    let source = serde_json::json!({"v":1,"device_id":keys.device_id(),"registered":true,"convs":[{"id":CONVERSATION,"title":"Offline source device"}],"seen":{"foreign-seen-state":900},"acked":900});
    archive
        .write(
            disk::STATE_FILE,
            &at_rest_seal(
                keys.vault_key(),
                RestKind::State,
                disk::STATE_FILE,
                &serde_json::to_vec(&source).unwrap(),
                &mut rng,
            )
            .unwrap(),
        )
        .unwrap();
    let conversation: Conv = CONVERSATION.parse().unwrap();
    for number in 0..600u32 {
        let record = history::Record {
            id: format!("019a0000-0000-7000-8000-{number:012x}"),
            at: 1000 + u64::from(number),
            from: Person {
                tag: "0012".into(),
                handle: "Bob#0012".into(),
            },
            mine: false,
            body: Body {
                text: format!("synthetic private history {number}: {}", "x".repeat(600)),
                ..Body::default()
            },
            evidence: None,
        };
        archive
            .append(
                &history::file_path(keys.vault_key(), &conversation),
                &history::seal_record(keys.vault_key(), &conversation, &record, &mut rng).unwrap(),
            )
            .unwrap();
    }
    assert!(archive.files.values().all(|file| {
        !file
            .windows(b"synthetic private history".len())
            .any(|window| window == b"synthetic private history")
    }));
    account.archive_chat_device(&device).unwrap();
    account
        .remember_chat_installation(&device, device.installation.clone())
        .unwrap();
    account
        .upload_chat_history(&device, &archive.files)
        .unwrap();
    let before = account.chat_devices(gateway, "af03").unwrap().remove(0);
    account
        .upload_chat_history(&device, &archive.files)
        .unwrap();
    let after = account.chat_devices(gateway, "af03").unwrap().remove(0);
    assert!(
        before.history == after.history,
        "unchanged files reuse chunks and do not advance the manifest"
    );
    Fixture {
        device,
        gateway: gateway.into(),
    }
}

pub(super) fn second_device(first: &mut Account, second: &mut Account, fixture: Fixture) {
    assert!(
        matches!(first.check_active(), Err(Error::Rejected)),
        "the offline first device's session was revoked"
    );
    assert!(
        first
            .upload_chat_history(&fixture.device, &std::collections::BTreeMap::new())
            .is_err()
    );
    let records = second.chat_devices(&fixture.gateway, "af03").unwrap();
    let restored = records
        .iter()
        .find(|record| record.installation == fixture.device.installation)
        .unwrap();
    assert_eq!(restored.key.bytes(), fixture.device.key.bytes());
    let files = second.read_chat_history(restored).unwrap();
    let mut corrupt_manifest = restored.clone();
    corrupt_manifest.history.as_mut().unwrap().files[0].chunks[0].digest[0] ^= 1;
    assert!(
        second.read_chat_history(&corrupt_manifest).is_err(),
        "changed manifest hashes must fail before restore"
    );
    let mut stale = restored.history.clone().unwrap();
    stale.files.clear();
    assert!(matches!(
        second.save_chat_history(restored, 0, stale),
        Err(Error::Conflict)
    ));

    let mut rng = Seeded::new(922);
    let active = DeviceSecrets::from_bytes(restored.key.bytes());
    assert_eq!(
        active.device_id(),
        fixture.device.device,
        "the new installation resumes the one logical identity and its pending inbox"
    );
    second
        .remember_chat_installation(restored, uuid::Uuid::now_v7().to_string())
        .unwrap();
    let stored = files.get(disk::STATE_FILE).unwrap();
    Social::validate_stored_state(&active, stored).unwrap();
    let (mut social, initial) = Social::start(
        Me {
            tag: "af03".into(),
            handle: "Alice#af03".into(),
            guest: false,
            gateway: fixture.gateway,
        },
        Some(stored.clone()),
        Some(vault::encode_secrets(&active).to_string()),
        ChatPrefs::default(),
        NOW,
        &mut rng,
    );
    let mut local = MemoryDisk {
        files: files.clone(),
    };
    for effect in initial {
        if let Some(result) = disk::apply(&effect, &mut local) {
            result.unwrap();
        }
    }
    let archive = MemoryDisk { files };
    let old = DeviceSecrets::from_bytes(restored.key.bytes());
    let effects = social
        .restore_archive(&old, &archive, &local, NOW, &mut rng)
        .unwrap();
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, Effect::SaveState(_) | Effect::RewriteHistory { .. })),
        "history restore cannot send receipts or register the old identity"
    );
    for effect in effects {
        disk::apply(&effect, &mut local).unwrap().unwrap();
    }
    assert_complete_state(&active, &local);
    second.upload_chat_history(restored, &local.files).unwrap();
    assert_eq!(
        second
            .chat_devices(&restored.gateway, &restored.tag)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(second.data().chat.installations.len(), 2);
}

fn assert_complete_state(active: &DeviceSecrets, local: &MemoryDisk) {
    let conversation: Conv = CONVERSATION.parse().unwrap();
    let log = local
        .read(&history::file_path(active.vault_key(), &conversation))
        .unwrap();
    let loaded = history::read(
        active.vault_key(),
        &history::file_name(active.vault_key(), &conversation),
        &log,
    );
    assert_eq!(loaded.skipped, 0);
    assert_eq!(
        loaded.records.len(),
        600,
        "all lines, including those beyond the UI window, return without the old device"
    );
    let state: serde_json::Value = serde_json::from_slice(
        &at_rest_open(
            active.vault_key(),
            RestKind::State,
            disk::STATE_FILE,
            &local.read(disk::STATE_FILE).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(state["device_id"], active.device_id());
    assert_eq!(state["registered"], true);
    assert_eq!(
        state["seen"]["foreign-seen-state"], 900,
        "the shared identity retains its replay suppression state"
    );
}
