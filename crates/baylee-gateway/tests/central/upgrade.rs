//! Reestablish an upgrade-era missing source proof without replacing the Central credential.
use super::{
    Account, AccountData, CentralHttp, Credentials, Endpoint, LegacyData, LegacyHttp,
    TicketPurpose, common, sdk,
};
use baylee_account::migration::LinkState;

pub(super) fn run(gateway: &common::Gateway, endpoint: &Endpoint, origin: &str) {
    common::login(gateway.port, "upgrade-pending", "Upgrade");
    let legacy = LegacyHttp::development(origin).unwrap();
    let credentials = Credentials {
        username: "upgrade-pending".into(),
        password: "a-very-fine-password".into(),
    };
    let enrollment = legacy.enroll(&credentials).unwrap();
    let source_id = enrollment.snapshot.account_id.clone();
    let digest = enrollment.snapshot.digest;
    let data = AccountData {
        version: 1,
        gateways: vec![origin.into()],
        preferences: enrollment.snapshot.preferences,
        chat: baylee_account::chat::ChatArchive::default(),
        legacy: Some(LegacyData {
            digest,
            complete: false,
            gateway: origin.into(),
            account_id: source_id.clone(),
            username: enrollment.snapshot.username,
            decks: enrollment.snapshot.decks,
        }),
    };
    let http = CentralHttp::new(endpoint.clone());
    sdk::enroll(
        &http,
        enrollment.identity,
        enrollment.ticket,
        credentials.password.clone(),
        &data.write().unwrap(),
    )
    .unwrap();
    let mut session = sdk::login(&http, enrollment.identity, credentials.password.clone()).unwrap();
    session.confirm_restored(&http).unwrap();
    // M23 cannot infer which old verifier authorized an existing preparation.
    gateway.sql(&format!(
        "UPDATE central_account_link SET source_credential_hash = NULL WHERE account_id = '{source_id}'::uuid"
    ));
    assert!(matches!(
        legacy
            .link_state(
                session
                    .gateway_capability(&http, origin.into(), TicketPurpose::Link)
                    .unwrap(),
                digest
            )
            .unwrap(),
        LinkState::FreshProofRequired
    ));
    assert!(
        legacy
            .prepare(
                session
                    .gateway_capability(&http, origin.into(), TicketPurpose::Export)
                    .unwrap(),
                Credentials {
                    username: credentials.username.clone(),
                    password: credentials.password.clone(),
                }
            )
            .is_err(),
        "an export capability cannot authorize fresh source proof"
    );
    assert!(
        legacy
            .prepare(
                session
                    .gateway_capability(&http, origin.into(), TicketPurpose::Link)
                    .unwrap(),
                Credentials {
                    username: credentials.username.clone(),
                    password: "wrong-synthetic-password".into(),
                }
            )
            .is_err()
    );
    let restored = Account::login(
        endpoint.clone(),
        credentials.username.clone(),
        credentials.password.clone(),
        Vec::new(),
    )
    .unwrap();
    assert!(restored.data().legacy.as_ref().unwrap().complete);
    assert_eq!(
        restored.data().legacy.as_ref().unwrap().account_id,
        source_id
    );
    assert_eq!(gateway.scalar(&format!("SELECT count(*) FROM account WHERE id = '{source_id}'::uuid AND central_only AND password_hash IS NULL")), 1);
    restored.logout().unwrap();
    closed_login_ignores_source(endpoint, &credentials, enrollment.identity);
}

fn closed_login_ignores_source(
    endpoint: &Endpoint,
    credentials: &Credentials,
    identity: baylee_account::wire::Identity,
) {
    // A complete/closed account must no longer need any source request to log in.
    let http = CentralHttp::new(endpoint.clone());
    let mut session = sdk::login(&http, identity, credentials.password.clone()).unwrap();
    session.confirm_restored(&http).unwrap();
    assert!(session.migration_closed(&http).unwrap());
    let mut completed = AccountData::read(session.document()).unwrap();
    completed.legacy.as_mut().unwrap().gateway = "http://127.0.0.1:1".into();
    session.save(&http, &completed.write().unwrap()).unwrap();
    session.logout(&http).unwrap();
    let independent = Account::login(
        endpoint.clone(),
        credentials.username.clone(),
        credentials.password.clone(),
        Vec::new(),
    )
    .unwrap();
    independent.logout().unwrap();
}
