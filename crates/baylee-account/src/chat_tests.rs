//! What a restored chat archive may claim: where its files lie, how much it
//! is, and whose identity each entry is.

use crate::{
    Error,
    chat::{
        ChatArchive, ChatDevice, ChatInstallation, DeviceKey, HistoryChunk, HistoryFile,
        HistorySnapshot,
    },
};
use zeroize::Zeroizing;

const GATEWAY: &str = "https://gateway.example";

fn installation(n: usize) -> String {
    format!("019a0000-0000-7000-8000-{n:012x}")
}

fn device(n: usize) -> ChatDevice {
    ChatDevice {
        gateway: GATEWAY.into(),
        tag: "af03".into(),
        installation: installation(n),
        device: format!("device-{n}"),
        key: DeviceKey::new(Zeroizing::new([7; 96])),
        retired: false,
        history: None,
    }
}

fn known(n: usize, device: usize) -> ChatInstallation {
    ChatInstallation {
        gateway: GATEWAY.into(),
        tag: "af03".into(),
        id: installation(n),
        device: format!("device-{device}"),
    }
}

fn chunk(n: u32, bytes: u32) -> HistoryChunk {
    let mut id = [0; 16];
    id[..4].copy_from_slice(&n.to_be_bytes());
    id[6] = 0x70;
    id[8] = 0x80;
    HistoryChunk {
        id,
        revision: 1,
        bytes,
        digest: [0; 32],
    }
}

fn file(n: u64, chunks: Vec<HistoryChunk>) -> HistoryFile {
    HistoryFile {
        name: format!("history/{n:016x}.log"),
        chunks,
    }
}

fn history(files: Vec<HistoryFile>) -> HistorySnapshot {
    HistorySnapshot {
        generation: 1,
        files,
    }
}

const LARGEST: u32 = 256 * 1024;

// ------------------------------------------------------------- devices

#[test]
fn each_label_of_a_device_is_bounded_and_canonical() {
    assert!(device(1).validate().is_ok());
    let valid = |change: &dyn Fn(&mut ChatDevice)| {
        let mut device = device(1);
        change(&mut device);
        device.validate().is_ok()
    };
    for (gateway, taken) in [
        ("http://127.0.0.1:4000", true),
        ("http://gateway.example", false),
        ("https://gateway.example/path", false),
        ("https://u@gateway.example", false),
        ("", false),
    ] {
        assert_eq!(valid(&|d| d.gateway = gateway.into()), taken, "{gateway}");
    }
    for (tag, taken) in [
        ("x".repeat(128), true),
        ("x".repeat(129), false),
        (String::new(), false),
        ("../af03".into(), false),
        ("af 03".into(), false),
    ] {
        assert_eq!(valid(&|d| d.tag.clone_from(&tag)), taken, "{tag}");
    }
    for (name, taken) in [
        ("x".repeat(96), true),
        ("x".repeat(97), false),
        ("a-b_c".into(), true),
        (String::new(), false),
        ("a/b".into(), false),
        ("a.b".into(), false),
    ] {
        assert_eq!(valid(&|d| d.device.clone_from(&name)), taken, "{name}");
    }
}

#[test]
fn an_installation_is_a_lowercase_version_seven_identifier() {
    let good = installation(1);
    let device_takes = |id: &str| {
        let mut device = device(1);
        device.installation = id.into();
        device.validate().is_ok()
    };
    let archive_takes = |id: &str| {
        let mut installation = known(1, 1);
        installation.id = id.into();
        let archive = ChatArchive {
            devices: vec![device(1)],
            installations: vec![installation],
        };
        archive.validate().is_ok()
    };
    for variant in ["8", "9", "a", "b"] {
        let mut id = good.clone();
        id.replace_range(19..20, variant);
        assert!(device_takes(&id) && archive_takes(&id), "{id}");
    }
    let changed = |at: usize, to: &str| {
        let mut id = good.clone();
        id.replace_range(at..=at, to);
        id
    };
    for bad in [
        changed(14, "4"),
        changed(19, "c"),
        changed(8, "0"),
        changed(0, "-"),
        changed(0, "A"),
        changed(35, "g"),
        good[1..].to_owned(),
        format!("{good}0"),
        String::new(),
    ] {
        assert!(!device_takes(&bad), "{bad}");
        assert!(!archive_takes(&bad), "{bad}");
    }
}

// ------------------------------------------------------------- the archive

#[test]
fn no_two_entries_of_an_archive_share_a_slot_or_an_identity() {
    let archive = |devices| ChatArchive {
        devices,
        installations: Vec::new(),
    };
    assert!(archive(vec![device(1), device(2)]).validate().is_ok());
    let mut same_slot = device(2);
    same_slot.installation = installation(1);
    assert!(archive(vec![device(1), same_slot]).validate().is_err());
    let mut same_identity = device(2);
    same_identity.device = device(1).device;
    assert!(archive(vec![device(1), same_identity]).validate().is_err());
    // The same names for another account, or at another gateway, are others.
    let mut account = device(1);
    account.tag = "0012".into();
    let mut gateway = device(1);
    gateway.gateway = "https://other.example".into();
    let apart = archive(vec![device(1), account, gateway]);
    assert!(apart.validate().is_ok());
}

#[test]
fn an_archive_holds_at_most_512_devices() {
    let archive = |count| ChatArchive {
        devices: (0..count).map(device).collect(),
        installations: Vec::new(),
    };
    assert!(archive(512).validate().is_ok());
    assert!(archive(513).validate().is_err());
    let mut full = archive(512);
    assert_eq!(full.insert(device(512)), Err(Error::Conflict));
    assert_eq!(full.devices.len(), 512);
    assert_eq!(full.insert(device(3)), Ok(()), "what it holds is no news");
}

#[test]
fn an_archive_remembers_at_most_1024_installations() {
    let archive = |count| ChatArchive {
        devices: vec![device(1)],
        installations: (0..count).map(|n| known(n, 1)).collect(),
    };
    assert!(archive(1024).validate().is_ok());
    assert!(archive(1025).validate().is_err());
}

#[test]
fn an_installation_restored_an_identity_of_its_own_account() {
    let archive = |installation| ChatArchive {
        devices: vec![device(1), device(2)],
        installations: vec![installation],
    };
    assert!(archive(known(9, 2)).validate().is_ok());
    assert!(archive(known(9, 3)).validate().is_err());
    let mut elsewhere = known(9, 2);
    elsewhere.tag = "0012".into();
    assert!(archive(elsewhere).validate().is_err());
    let mut elsewhere = known(9, 2);
    elsewhere.gateway = "https://other.example".into();
    assert!(archive(elsewhere).validate().is_err());
    let twice = ChatArchive {
        devices: vec![device(1), device(2)],
        installations: vec![known(9, 1), known(9, 2)],
    };
    assert!(
        twice.validate().is_err(),
        "one installation, two identities"
    );
}

#[test]
fn an_identity_is_inserted_whole_or_not_at_all() {
    let mut archive = ChatArchive::default();
    let mut bad = device(1);
    bad.tag = String::new();
    assert_eq!(archive.insert(bad), Err(Error::Rejected));
    assert!(archive.devices.is_empty());
    archive.insert(device(1)).unwrap();
    let mut retired = device(1);
    retired.retired = true;
    assert_eq!(archive.insert(retired), Err(Error::Conflict));
    assert!(!archive.devices[0].retired);
}

#[cfg(feature = "client")]
#[test]
fn an_installation_that_restores_another_identity_is_remembered_once() {
    let mut archive = ChatArchive {
        devices: vec![device(1), device(2)],
        installations: Vec::new(),
    };
    archive.remember(&device(1), installation(9)).unwrap();
    archive.remember(&device(2), installation(9)).unwrap();
    assert_eq!(archive.installations.len(), 1);
    assert_eq!(archive.installations[0].device, "device-2");
    assert_eq!(
        archive.remember(&device(1), "not an identifier".into()),
        Err(Error::Rejected)
    );
    assert_eq!(archive.installations.len(), 1);
}

// ------------------------------------------------------------- the history

#[test]
fn a_history_belongs_to_the_device_and_is_checked_with_it() {
    let mut device = device(1);
    device.history = Some(history(vec![file(1, vec![chunk(1, 1)])]));
    assert!(device.validate().is_ok());
    device.history = Some(history(vec![file(1, vec![chunk(1, 0)])]));
    assert!(device.validate().is_err());
}

#[test]
fn a_history_has_a_generation_and_at_most_1025_files() {
    let files = |count| history((0..count).map(|n| file(n, Vec::new())).collect());
    assert!(files(1025).validate().is_ok());
    assert!(files(1026).validate().is_err());
    let mut none = files(1);
    none.generation = 0;
    assert!(none.validate().is_err());
}

#[test]
fn a_file_of_a_history_has_at_most_64_chunks() {
    let chunks = |count| history(vec![file(1, (0..count).map(|n| chunk(n, 1)).collect())]);
    assert!(chunks(64).validate().is_ok());
    assert!(chunks(65).validate().is_err());
}

#[test]
fn a_chunk_has_an_identifier_a_revision_and_a_size_of_its_own() {
    let valid = |change: &dyn Fn(&mut HistoryChunk)| {
        let mut chunk = chunk(1, LARGEST);
        change(&mut chunk);
        history(vec![file(1, vec![chunk])]).validate().is_ok()
    };
    assert!(valid(&|_| ()));
    assert!(!valid(&|c| c.bytes = LARGEST + 1));
    assert!(!valid(&|c| c.bytes = 0));
    assert!(!valid(&|c| c.revision = 0));
    assert!(!valid(&|c| c.id[6] = 0x40));
    assert!(!valid(&|c| c.id[8] = 0xC0));
}

#[test]
fn one_chunk_is_never_part_of_two_files() {
    let shared = history(vec![file(1, vec![chunk(1, 1)]), file(2, vec![chunk(1, 1)])]);
    assert!(shared.validate().is_err());
}

#[test]
fn a_history_is_at_most_256_mebibytes_in_all() {
    // Sixteen files of 64 full chunks are the bound exactly.
    let full = |extra: Option<u32>| {
        let mut files: Vec<_> = (0..16)
            .map(|f| {
                let chunks = (0..64).map(|c| chunk(f * 64 + c, LARGEST)).collect();
                file(u64::from(f), chunks)
            })
            .collect();
        files.extend(extra.map(|bytes| file(99, vec![chunk(5_000, bytes)])));
        history(files).validate().is_ok()
    };
    assert!(full(None));
    assert!(!full(Some(1)));
}

#[test]
fn only_the_state_and_a_log_of_sixteen_hex_digits_are_files_of_a_history() {
    let named = |name: &str| {
        let mut file = file(1, vec![chunk(1, 1)]);
        file.name = name.into();
        history(vec![file]).validate().is_ok()
    };
    assert!(named("state.bin"));
    assert!(named("history/0123456789abcdef.log"));
    for bad in [
        "",
        "state.bin/",
        "/state.bin",
        "history/state.bin",
        "history/0123456789abcde.log",
        "history/0123456789abcdef0.log",
        "history/0123456789abcdeg.log",
        "history/0123456789abcdef",
        "history/..............x.log",
        "0123456789abcdef.log",
    ] {
        assert!(!named(bad), "{bad}");
    }
}
