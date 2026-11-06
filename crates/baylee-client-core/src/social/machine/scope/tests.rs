use super::*;
use crate::social::{ChatPrefs, Me, SocialLink, vault};
use baylee_protocol::social::{AccountKeys, DeviceKey, KeysAnswer};
use baylee_seal::{
    DeviceSecrets,
    b64::url,
    testkit::{Seeded, registered},
};

fn keys(tag: &str, secret: &DeviceSecrets) -> AccountKeys {
    let public = secret.public(tag).unwrap();
    AccountKeys {
        tag: tag.into(),
        handle: format!("Synthetic#{tag}"),
        devices: vec![DeviceKey {
            id: public.id,
            ik: url::encode(&public.ik),
            dk: url::encode(&public.dk),
            dsig: url::encode(&public.dsig),
            label: "test".into(),
            added_at: 1,
        }],
    }
}
fn answer(
    machine: &mut Social,
    game: bool,
    rows: Vec<AccountKeys>,
    rng: &mut Seeded,
) -> Vec<Effect> {
    let tags = rows.iter().map(|row| row.tag.clone()).collect();
    let tag = if game {
        HttpTag::GameKeys(tags)
    } else {
        HttpTag::Keys(tags)
    };
    machine.on_http(
        tag,
        200,
        &serde_json::to_vec(&KeysAnswer { keys: rows }).unwrap(),
        100,
        rng,
    )
}
fn start() -> (Social, DeviceSecrets, Seeded) {
    let own = DeviceSecrets::from_bytes(&[71; 96]);
    let mut rng = Seeded::new(91);
    let (mut machine, _) = Social::start(
        Me {
            tag: "0007".into(),
            handle: "A#0007".into(),
            guest: false,
            gateway: "https://example.test".into(),
        },
        None,
        Some(vault::encode_secrets(&own).to_string()),
        ChatPrefs::default(),
        100,
        &mut rng,
    );
    machine.use_account_transport();
    machine.on_link(SocialLink::Up, 100);
    (machine, own, rng)
}
fn open(machine: &mut Social, game: bool, rng: &mut Seeded) {
    let id = if game {
        "tbl:01931234-5678-7000-8000-000000000001"
    } else {
        "dm:0007:0009"
    };
    machine.act(Intent::OpenConv(id.parse().unwrap()), 100, rng);
}

#[test]
fn gateway_rotation_cannot_change_personal_safety_or_confirm_personal_keys() {
    let (mut machine, own, mut rng) = start();
    let (personal, _) = registered("0009", 81);
    let (gateway, _) = registered("0009", 82);
    let (replacement, _) = registered("0009", 83);
    answer(
        &mut machine,
        false,
        vec![keys("0007", &own), keys("0009", &personal)],
        &mut rng,
    );
    answer(
        &mut machine,
        true,
        vec![keys("0007", &own), keys("0009", &gateway)],
        &mut rng,
    );
    open(&mut machine, false, &mut rng);
    let personal_safety = machine.safety("0009").unwrap();
    assert_eq!(
        personal_safety.groups.join(""),
        baylee_seal::safety_number(("0007", &[own.ik()]), ("0009", &[personal.ik()]))
    );
    machine.act(
        Intent::Verify {
            tag: "0009".into(),
            expected: personal_safety.expected.clone(),
        },
        100,
        &mut rng,
    );
    open(&mut machine, true, &mut rng);
    let game_safety = machine.safety("0009").unwrap();
    assert_ne!(game_safety.groups, personal_safety.groups);
    machine.act(
        Intent::Verify {
            tag: "0009".into(),
            expected: game_safety.expected,
        },
        100,
        &mut rng,
    );
    let effects = answer(
        &mut machine,
        true,
        vec![keys("0009", &replacement)],
        &mut rng,
    );
    assert!(!machine.safety("0009").unwrap().verified);
    machine.act(
        Intent::Verify {
            tag: "0009".into(),
            expected: personal_safety.expected,
        },
        100,
        &mut rng,
    );
    assert!(
        !machine.safety("0009").unwrap().verified,
        "another context's confirmation never approves these keys"
    );
    open(&mut machine, false, &mut rng);
    assert!(machine.safety("0009").unwrap().verified);
    assert_eq!(
        machine.safety("0009").unwrap().groups,
        personal_safety.groups
    );
    let bytes = effects
        .iter()
        .find_map(|effect| {
            if let Effect::SaveState(bytes) = effect {
                Some(bytes.clone())
            } else {
                None
            }
        })
        .unwrap();
    let (mut restored, _) = Social::start(
        machine.me.clone(),
        Some(bytes),
        Some(vault::encode_secrets(&own).to_string()),
        ChatPrefs::default(),
        100,
        &mut rng,
    );
    restored.use_account_transport();
    open(&mut restored, false, &mut rng);
    assert_eq!(
        restored.contact_keys("0009").unwrap().devices[0].ik,
        personal.ik()
    );
    assert!(restored.safety("0009").unwrap().verified);
    open(&mut restored, true, &mut rng);
    assert_eq!(
        restored.contact_keys("0009").unwrap().devices[0].ik,
        replacement.ik()
    );
    assert!(restored.contact_keys("0009").unwrap().requires_confirmation);
}

#[test]
fn simultaneous_directories_do_not_coalesce_across_services() {
    let (mut machine, _, _) = start();
    let mut effects = Vec::new();
    machine.request_keys(&["0009".into()], &mut effects);
    machine.select_key_scope(true);
    machine.request_keys(&["0009".into()], &mut effects);
    machine.select_key_scope(false);
    machine.request_keys(&["0009".into()], &mut effects);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(
                effect,
                Effect::Http {
                    tag: HttpTag::Keys(_),
                    ..
                }
            ))
            .count(),
        1
    );
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(
                effect,
                Effect::Http {
                    tag: HttpTag::GameKeys(_),
                    ..
                }
            ))
            .count(),
        1
    );
}

#[test]
fn legacy_group_delivery_uses_gateway_pins_and_is_archived_before_ack_without_granting_send_access()
{
    use crate::social::{LineKind, RetainedMessage, sealing};
    use baylee_protocol::social::{Delivered, GroupInfo, Member, Person, Sender, Up};
    let (mut machine, own, mut rng) = start();
    let (personal, _) = registered("0009", 81);
    let (gateway, _) = registered("0009", 82);
    answer(
        &mut machine,
        false,
        vec![keys("0007", &own), keys("0009", &personal)],
        &mut rng,
    );
    answer(
        &mut machine,
        true,
        vec![keys("0007", &own), keys("0009", &gateway)],
        &mut rng,
    );
    let group = GroupInfo {
        id: "01931234-5678-7000-8000-000000000001".into(),
        name: "Legacy group".into(),
        members: vec![
            Member {
                person: Person {
                    tag: "0007".into(),
                    handle: "A#0007".into(),
                    guest: false,
                },
                owner: true,
            },
            Member {
                person: Person {
                    tag: "0009".into(),
                    handle: "B#0009".into(),
                    guest: false,
                },
                owner: false,
            },
        ],
        invited: vec![],
    };
    let conv = format!("grp:{}", group.id);
    let id = sealing::uuid_v7(100, &mut rng);
    let hdr = sealing::header_bytes(&id, &conv, "0009", gateway.device_id(), 100);
    let sealed = baylee_seal::seal(
        &gateway,
        &hdr,
        br#"{"text":"retained legacy message"}"#,
        &[baylee_seal::Recipient {
            device: own.device_id(),
            dk: own.dk(),
        }],
        &mut rng,
    )
    .unwrap();
    let delivery = RetainedMessage {
        seq: 44,
        at: 100,
        conv: conv.clone(),
        from: Sender {
            tag: "0009".into(),
            handle: "B#0009".into(),
            device: gateway.device_id().into(),
            guest: false,
        },
        sealed: Delivered {
            hdr: url::encode(&hdr),
            n: url::encode(&sealed.n),
            ct: url::encode(&sealed.ct),
            epk: url::encode(&sealed.epk),
            sig: url::encode(&sealed.sig),
            k: url::encode(&sealed.wraps[0].k),
        },
    };
    let rejected = machine.on_retained(delivery.clone(), 100, &mut rng);
    assert!(!rejected.iter().any(|effect| matches!(
        effect,
        Effect::AppendHistory { .. } | Effect::Up(Up::Ack { .. })
    )));
    let effects = machine.on_legacy_group(delivery, &group, 100, &mut rng);
    let stored = effects
        .iter()
        .position(|effect| matches!(effect, Effect::AppendHistory { .. }))
        .unwrap();
    let state = effects
        .iter()
        .position(|effect| matches!(effect, Effect::SaveState(_)))
        .unwrap();
    let ack = effects
        .iter()
        .position(|effect| matches!(effect, Effect::Up(Up::Ack { upto: 44 })))
        .unwrap();
    assert!(stored < ack && state < ack);
    let conversation = machine.conversation(&conv.parse().unwrap()).unwrap();
    assert!(conversation.lines.iter().any(|line|matches!(&line.kind,LineKind::Message {body,..} if body.text == "retained legacy message")));
    assert!(conversation.ended_at.is_some());
    assert!(machine.contacts().group(&group.id).is_none());
    assert!(machine.encrypted_to(&conv.parse().unwrap()).is_none());
}

#[test]
fn former_central_group_history_uses_personal_pins_without_granting_membership() {
    use crate::social::{RetainedMessage, sealing};
    use baylee_protocol::social::{Delivered, Sender, Up};
    let (mut machine, own, mut rng) = start();
    let (peer, _) = registered("0009", 81);
    answer(
        &mut machine,
        false,
        vec![keys("0007", &own), keys("0009", &peer)],
        &mut rng,
    );
    let conv = "grp:01931234-5678-7000-8000-000000000001";
    let id = sealing::uuid_v7(100, &mut rng);
    let hdr = sealing::header_bytes(&id, conv, "0009", peer.device_id(), 100);
    let sealed = baylee_seal::seal(
        &peer,
        &hdr,
        br#"{"text":"already delivered"}"#,
        &[baylee_seal::Recipient {
            device: own.device_id(),
            dk: own.dk(),
        }],
        &mut rng,
    )
    .unwrap();
    let effects = machine.on_retained(
        RetainedMessage {
            seq: 99,
            at: 100,
            conv: conv.into(),
            from: Sender {
                tag: "0009".into(),
                handle: "B#0009".into(),
                device: peer.device_id().into(),
                guest: false,
            },
            sealed: Delivered {
                hdr: url::encode(&hdr),
                n: url::encode(&sealed.n),
                ct: url::encode(&sealed.ct),
                epk: url::encode(&sealed.epk),
                sig: url::encode(&sealed.sig),
                k: url::encode(&sealed.wraps[0].k),
            },
        },
        100,
        &mut rng,
    );
    assert!(
        effects
            .iter()
            .any(|e| matches!(e, Effect::AppendHistory { .. }))
    );
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::Up(Up::Ack { .. })))
    );
    assert!(machine.contacts().groups.is_empty());
    assert!(machine.encrypted_to(&conv.parse().unwrap()).is_none());
}
