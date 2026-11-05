//! Actual native and browser crypto implementations, no mocked cipher or key.
use baylee_account::{chat_api as api, sdk::chat::Chat, social::Person};
use baylee_client_core::social::sealing;
use baylee_seal::{DeviceSecrets, Recipient, b64::url, testkit::Seeded};

fn directory(keys: &DeviceSecrets, tag: u32) -> api::AccountKeys {
    let public = keys.public(&format!("{tag:04x}")).unwrap();
    api::AccountKeys {
        tag: format!("{tag:04x}"),
        handle: format!("Synthetic#{tag:04x}"),
        devices: vec![api::DeviceKey {
            id: public.id,
            ik: url::encode(&public.ik),
            dk: url::encode(&public.dk),
            dsig: url::encode(&public.dsig),
            label: "Account".into(),
            added_at: 1,
        }],
    }
}

#[test]
fn browser_and_native_open_each_others_real_ciphertext_and_share_the_account_identity() {
    let alice = DeviceSecrets::from_bytes(&[71; 96]);
    let mut bob_bytes = [91; 96];
    let browser_keys = central_crypto::chat::DeviceSecrets::from_bytes(&bob_bytes);
    // A migrated native archive may retain its old at-rest key. This must not
    // change its wire identity or prevent the browser from reading its messages.
    bob_bytes[64..].fill(92);
    let bob = DeviceSecrets::from_bytes(&bob_bytes);
    let mut browser = Chat::new(browser_keys, 9).unwrap();
    assert_eq!(browser.device(), bob.device_id());
    let bob_directory = directory(&bob, 9);
    assert_eq!(
        browser.registration().unwrap().dsig,
        bob_directory.devices[0].dsig
    );
    browser.observe(directory(&alice, 7)).unwrap();
    let now = 1_800_000_000_000;
    let mut rng = Seeded::new(77);
    let id = sealing::uuid_v7(now, &mut rng);
    let hdr = sealing::header_bytes(&id, "dm:0007:0009", "0007", alice.device_id(), now);
    let body=serde_json::to_vec(&serde_json::json!({"text":"Native → browser: Grüße 🦊","links":[],"mentions":[],"emote":true})).unwrap();
    let sealed = baylee_seal::seal(
        &alice,
        &hdr,
        &body,
        &[Recipient {
            device: bob.device_id(),
            dk: bob.dk(),
        }],
        &mut rng,
    )
    .unwrap();
    let delivered = api::Delivery {
        seq: 1,
        at: now,
        sender: Person {
            tag: 7,
            display_name: "Alice".into(),
        },
        device: directory(&alice, 7).devices.remove(0),
        sealed: api::Sealed {
            hdr: url::encode(&hdr),
            n: url::encode(&sealed.n),
            ct: url::encode(&sealed.ct),
            epk: url::encode(&sealed.epk),
            sig: url::encode(&sealed.sig),
            wraps: vec![api::Wrap {
                device: bob.device_id().into(),
                k: url::encode(&sealed.wraps[0].k),
            }],
        },
    };
    let opened = browser.open(&delivered).unwrap();
    assert_eq!(opened.text, "Native → browser: Grüße 🦊");
    assert!(opened.emote);
    let reply = browser
        .seal(
            "dm:0007:0009",
            "Browser → native: 日本語".into(),
            false,
            &[7, 9],
            now,
        )
        .unwrap();
    let hdr = url::decode(&reply.hdr).unwrap();
    let n = url::decode_array::<24>(&reply.n).unwrap();
    let ct = url::decode(&reply.ct).unwrap();
    let epk = url::decode_array::<32>(&reply.epk).unwrap();
    let sig = url::decode_array::<64>(&reply.sig).unwrap();
    let wrap = url::decode_array::<48>(
        &reply
            .wraps
            .iter()
            .find(|wrap| wrap.device == alice.device_id())
            .unwrap()
            .k,
    )
    .unwrap();
    let envelope = baylee_seal::Envelope {
        hdr: &hdr,
        n: &n,
        ct: &ct,
        epk: &epk,
        sig: &sig,
    };
    let opened = baylee_seal::open(&alice, &bob.ik(), &envelope, &wrap).unwrap();
    let body: baylee_client_core::social::Body = serde_json::from_slice(&opened.body).unwrap();
    assert_eq!(body.text, "Browser → native: 日本語");
    let mut tampered = ct.clone();
    tampered[0] ^= 1;
    let envelope = baylee_seal::Envelope {
        ct: &tampered,
        ..envelope
    };
    assert!(baylee_seal::open(&alice, &bob.ik(), &envelope, &wrap).is_err());
}
