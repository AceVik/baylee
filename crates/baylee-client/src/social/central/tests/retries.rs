use super::*;
use baylee_client_core::social::{ComposerKey, Intent, LineState};

fn pending(host: &mut SocialHost, conv: &wire::Conv, req: u32) -> (String, wire::Sealed) {
    let machine = host.machine.as_mut().unwrap();
    let mut rng = Seeded::new(u64::from(req));
    machine.act(Intent::OpenConv(conv.clone()), 100, &mut rng);
    machine.composer_mut().typed("An uncertain send");
    machine.key(ComposerKey::Enter, 100, &mut rng);
    let line = machine.conversation(conv).unwrap().lines.last().unwrap();
    assert_eq!(line.state, LineState::Pending);
    let id = line.key.id.clone();
    let header = wire::Header {
        v: wire::HEADER_VERSION,
        id: id.clone(),
        conv: conv.to_string(),
        from: "0007".into(),
        dev: machine.device_id().into(),
        ts: 100,
        kind: "text".into(),
    };
    let sealed = wire::Sealed {
        hdr: baylee_seal::b64::url::encode(&serde_json::to_vec(&header).unwrap()),
        n: String::new(),
        ct: "original ciphertext".into(),
        epk: String::new(),
        sig: String::new(),
        wraps: vec![],
    };
    host.central
        .as_ref()
        .unwrap()
        .retries
        .lock()
        .unwrap()
        .insert(
            sealed.hdr.clone(),
            serde_json::from_value(serde_json::to_value(&sealed).unwrap()).unwrap(),
        );
    machine.on_frame(
        Down::Refused {
            req: Some(req),
            why: wire::Why::Rate,
            missing: vec![],
            extra: vec![],
            message: None,
        },
        100,
        &mut rng,
    );
    (id, sealed)
}

#[test]
fn losing_group_membership_releases_its_retry_slot_but_keeps_uncertain_sends() {
    let mut host = host();
    let mut rng = Seeded::new(43);
    let group: wire::Conv = "grp:019a0000-0000-7000-8000-000000000001".parse().unwrap();
    let dm: wire::Conv = "dm:0007:0009".parse().unwrap();
    let mut snapshot = wire::Snapshot::default();
    snapshot.groups.push(wire::GroupInfo {
        id: "019a0000-0000-7000-8000-000000000001".into(),
        name: "Temporary group".into(),
        members: vec![wire::Member {
            person: wire::Person {
                tag: "0007".into(),
                handle: "Alice#0007".into(),
                guest: false,
            },
            owner: true,
        }],
        invited: vec![],
    });
    host.machine
        .as_mut()
        .unwrap()
        .on_frame(Down::Snapshot(snapshot), 100, &mut rng);
    let (group_id, group_sealed) = pending(&mut host, &group, 1);
    let (dm_id, dm_sealed) = pending(&mut host, &dm, 2);
    let machine = host.machine.as_mut().unwrap();
    machine.on_frame(Down::Snapshot(wire::Snapshot::default()), 200, &mut rng);
    machine.tick(10_000, 0);
    machine.flush(10_000, &mut rng);
    assert_eq!(
        machine
            .conversation(&group)
            .unwrap()
            .line(&group_id)
            .unwrap()
            .state,
        LineState::Failed
    );
    assert_eq!(
        machine
            .conversation(&dm)
            .unwrap()
            .line(&dm_id)
            .unwrap()
            .state,
        LineState::Pending
    );

    assert!(up(
        &mut host,
        &Up::Send {
            req: 2,
            conv: dm.to_string(),
            sealed: dm_sealed.clone()
        }
    ));
    let retries = host.central.as_ref().unwrap().retries.lock().unwrap();
    assert!(
        !retries.contains_key(&group_sealed.hdr),
        "a locally abandoned send must not leak a retry slot"
    );
    assert_eq!(
        serde_json::to_value(retries.get(&dm_sealed.hdr).unwrap()).unwrap(),
        serde_json::to_value(&dm_sealed).unwrap()
    );
}
