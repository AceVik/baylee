use super::*;
use crate::social::OsEntropy;
use baylee_account::chat::{DeviceKey, HistorySnapshot};
use baylee_client_core::social::ChatPrefs;
use baylee_seal::{DeviceSecrets, RestKind, at_rest_seal};
use std::sync::{atomic::Ordering, mpsc};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir()
            .canonicalize()
            .unwrap()
            .join(format!("baylee-chat-bootstrap-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn session() -> Session {
    Session {
        gateway: "https://gateway.example".into(),
        tag: "af03".into(),
        handle: "Alice#af03".into(),
        token: "synthetic token".into(),
        guest: false,
    }
}

#[test]
fn first_import_retains_verified_peer_keys_and_never_overlays_a_remote_root() {
    let temp = Temp::new();
    let session = session();
    let keys = DeviceSecrets::generate(&mut OsEntropy);
    let peer = DeviceSecrets::generate(&mut OsEntropy);
    let mut device = ChatDevice {
        gateway: session.gateway.clone(),
        tag: session.tag.clone(),
        installation: uuid::Uuid::now_v7().to_string(),
        device: keys.device_id().into(),
        key: DeviceKey::new(keys.to_bytes()),
        retired: false,
        history: None,
    };
    let state = serde_json::json!({
        "v":1,"device_id":keys.device_id(),"registered":true,
        "keys":{"accounts":{"0012":{
            "handle":"Bob#0012","first_seen":1,
            "devices":[{"id":peer.device_id(),"ik":peer.ik(),"dk":peer.dk(),"label":"old verified device","added_at":1}],
            "verified_iks":[peer.ik()],"requires_confirmation":false
        }}}
    });
    let ciphertext = at_rest_seal(
        keys.vault_key(),
        RestKind::State,
        social::disk::STATE_FILE,
        &serde_json::to_vec(&state).unwrap(),
        &mut OsEntropy,
    )
    .unwrap();
    let path = temp.0.join(social::disk::STATE_FILE);
    std::fs::write(&path, &ciphertext).unwrap();
    let base = local_initial(&device, [temp.0.clone()]).unwrap().unwrap();
    let (machine, _, _) =
        super::super::start_shared(base, &device, &session, ChatPrefs::default(), 30).unwrap();
    let peer_keys = machine.contact_keys("0012").unwrap();
    assert!(
        peer_keys.verified(),
        "migration must not turn an existing verified peer back into TOFU"
    );
    assert_eq!(peer_keys.public_keys(), vec![(peer.ik(), peer.dk())]);
    assert_eq!(std::fs::read(&path).unwrap(), ciphertext);

    let original = device.key.clone();
    device.key = DeviceKey::new(peer.to_bytes());
    assert!(matches!(
        local_initial(&device, [temp.0.clone()]),
        Err(Error::Authentication)
    ));
    device.key = original;
    std::fs::write(&path, b"damaged authenticated state").unwrap();
    assert!(matches!(
        local_initial(&device, [temp.0.clone()]),
        Err(Error::Authentication)
    ));
    device.history = Some(HistorySnapshot {
        generation: 1,
        files: vec![],
    });
    assert!(
        local_initial(&device, [temp.0.clone()]).unwrap().is_none(),
        "an existing Central root prohibits any local state overlay"
    );
    assert!(
        matches!(
            super::super::start_shared(
                MemoryDisk::default(),
                &device,
                &session,
                ChatPrefs::default(),
                30
            ),
            Err(Error::Format)
        ),
        "a published archive without its state must not silently reset peer trust"
    );
}

#[test]
fn session_handoff_waits_for_the_actual_guard_release_and_is_bounded_and_cancellable() {
    let temp = Temp::new();
    let scope = Scope::new("af03", "https://gateway.example").unwrap();
    let first = ContextGuard::open(&temp.0, scope.clone()).unwrap();
    let cancelled = AtomicBool::new(false);
    assert!(matches!(
        wait_for_context(&temp.0, scope.clone(), &cancelled, Duration::ZERO),
        Err(Error::Busy)
    ));
    cancelled.store(true, Ordering::Release);
    assert!(wait_for_context(&temp.0, scope.clone(), &cancelled, Duration::from_secs(1)).is_err());
    let root = temp.0.clone();
    let (ready, started) = mpsc::channel();
    let (done, finished) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        ready.send(()).unwrap();
        let result = wait_for_context(
            &root,
            scope,
            &AtomicBool::new(false),
            Duration::from_secs(5),
        );
        let _ = done.send(result);
    });
    started.recv().unwrap();
    // The previous process still owns the OS lock: the new operation must not
    // finish or apply any state until the previous session releases it.
    let early = finished.recv_timeout(Duration::from_millis(150));
    let waiting = matches!(early, Err(mpsc::RecvTimeoutError::Timeout));
    drop(first);
    worker.join().unwrap();
    let second = early.unwrap_or_else(|_| finished.recv().unwrap());
    assert!(waiting);
    drop(second.unwrap());
}
