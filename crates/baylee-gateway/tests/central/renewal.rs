//! A renewed Central session keeps its gateway session (baylee-central
//! `docs/operations.md` §"Sessions"): Central moves the lease's expiry with
//! the session, and the gateway takes that later expiry on its next check
//! instead of refusing a lease that no longer matches the one it stored. Before
//! renewal, every Central sign-in, and every game on it, ended 30 minutes in
//! (beta.8a, 22.10.2026).
use super::{
    Account, AccountData, CentralHttp, Credentials, Endpoint, LegacyData, LegacyHttp, common, sdk,
};
use std::time::Duration;

const USERNAME: &str = "renewing";
const PASSWORD: &str = "a-very-fine-password";

pub(super) fn run(gateway: &common::Gateway, endpoint: &Endpoint, origin: &str) {
    common::login(gateway.port, USERNAME, "Renewing");
    let legacy = LegacyHttp::development(origin).unwrap();
    let credentials = Credentials {
        username: USERNAME.into(),
        password: PASSWORD.into(),
    };
    let enrollment = legacy.enroll(&credentials).unwrap();
    let source_id = enrollment.snapshot.account_id.clone();
    let data = AccountData {
        version: 1,
        gateways: vec![origin.into()],
        preferences: enrollment.snapshot.preferences,
        chat: baylee_account::chat::ChatArchive::default(),
        legacy: Some(LegacyData {
            digest: enrollment.snapshot.digest,
            complete: false,
            gateway: origin.into(),
            account_id: source_id.clone(),
            username: enrollment.snapshot.username,
            decks: enrollment.snapshot.decks,
        }),
    };
    sdk::enroll(
        &CentralHttp::new(endpoint.clone()),
        enrollment.identity,
        enrollment.ticket,
        PASSWORD.into(),
        &data.write().unwrap(),
    )
    .unwrap();
    let account = Account::login(
        endpoint.clone(),
        USERNAME.into(),
        PASSWORD.into(),
        Vec::new(),
    )
    .unwrap();
    let admitted = account.admit(origin).unwrap();
    // The lease bounds a Central session here; the bearer's own expiry
    // renews itself on use, as every session's does.
    let leased = format!(
        "SELECT floor(extract(epoch FROM c.expires_at))::bigint FROM session_token s
         JOIN central_gateway_session c ON c.token_hash = s.token_hash
         WHERE s.account_id = '{source_id}'::uuid"
    );
    let first = gateway.scalar(&leased);
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&admitted.token), "").0,
        200
    );
    // Expiries are whole seconds: one later, the renewal is a later one.
    std::thread::sleep(Duration::from_millis(1_100));
    let renewed = account.renew().unwrap();
    assert!(renewed > first, "a renewal moves the session forward");
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&admitted.token), "").0,
        200,
        "the gateway takes the renewed lease"
    );
    assert_eq!(gateway.scalar(&leased), renewed, "and keeps its expiry");
    // Signing out still ends it at once, renewed or not.
    account.logout().unwrap();
    assert_eq!(
        common::http(gateway.port, "GET", "/me", Some(&admitted.token), "").0,
        401
    );
}
