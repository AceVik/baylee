//! An account registered at Central, never a legacy account, plays on the
//! official gateway: a Central beta key, the registration with its recovery
//! wrapper as the website makes it (through the identity service's browser
//! entry, which the browser listener calls after its own checks), the native
//! sign-in, and the bridge gateway making the account a projection
//! (baylee-central `docs/web-accounts.md`).
use super::{Endpoint, common};
use baylee_account::{
    Account,
    wire::{Request, Response},
};
use central_api::beta_key::BetaKey;
use central_crypto::{
    Context,
    opaque::Enrollment,
    vault::{MasterKey, ObjectContext, RecoveryCode},
};
use central_identity::IdentityService;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// The plaintext the website seals as object 1, byte for byte.
const DOCUMENT: &[u8] = br#"{"version":1,"gateways":["https://baylee.acevik.de"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#;

/// `service` is the one the test serves: the registration reaches it through
/// its browser entry, the native sign-in over HTTP.
pub fn run(
    runtime: &Runtime,
    service: &Arc<IdentityService>,
    operator: &DatabaseConnection,
    endpoint: &Endpoint,
    gateway: &common::Gateway,
) {
    let origin = &format!("http://127.0.0.1:{}", gateway.port);
    let key = BetaKey::generate().unwrap();
    runtime.block_on(async {
        central_postgres::BetaKeyBatch::new(&[key.digest()], 1, None, "e2e fresh registration")
            .unwrap()
            .issue(operator)
            .await
            .unwrap();
    });
    let website = |request: &Request| {
        let request = serde_json::from_slice(&serde_json::to_vec(request).unwrap()).unwrap();
        runtime
            .block_on(service.handle_browser(request, None))
            .unwrap()
            .response
    };
    let username = "fresh_player";
    let password = "a fresh player's long password".to_string();
    let register = Request::Register {
        username: username.into(),
        beta_key: key.display().to_string(),
        display_name: Some("Fresh_Player".into()),
    };
    let Response::Enrollment { identity, ticket } = website(&register) else {
        panic!("a valid key registers a free name");
    };
    let context = Context::new(endpoint.issuer(), identity.account, identity.epoch).unwrap();
    let (enrollment, message) = Enrollment::start(password.clone(), context).unwrap();
    let start = Request::EnrollmentStart { ticket, message };
    let Response::EnrollmentChallenge { message } = website(&start) else {
        panic!("the enrollment starts");
    };
    let result = enrollment.finish(&message).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = result.wrapping_key.wrap(&master).unwrap();
    let recovery = RecoveryCode::generate()
        .unwrap()
        .wrap(context, &master)
        .unwrap();
    let object = ObjectContext::new(context, identity.account, 1).unwrap();
    let document = master.seal_object(object, DOCUMENT).unwrap();
    let Request::EnrollmentStart { ticket, .. } = start else {
        unreachable!()
    };
    let finish = Request::EnrollmentFinish {
        ticket,
        record: result.upload,
        wrapped_master: wrapped.as_bytes().to_vec(),
        document,
        recovery: Some(recovery.as_bytes().to_vec()),
    };
    assert!(matches!(website(&finish), Response::Done));

    // The native client's sign-in restores the website's document.
    let account = Account::login(endpoint.clone(), username.into(), password, Vec::new()).unwrap();
    assert_eq!(account.data().gateways, vec![central_api::LEGACY_ORIGIN]);
    assert!(account.data().legacy.is_none());

    // The bridge gateway has no legacy account for it, and makes one.
    let admission = account.admit(origin).unwrap();
    let (status, body) = common::http(gateway.port, "GET", "/me", Some(&admission.token), "");
    assert_eq!(status, 200, "{body}");
    // Signing in again finds the same projection.
    let again = account.admit(origin).unwrap();
    let (_, second) = common::http(gateway.port, "GET", "/me", Some(&again.token), "");
    let id = |body: &str| serde_json::from_str::<serde_json::Value>(body).unwrap()["id"].clone();
    assert_eq!(id(&body), id(&second), "one projection per Central account");
}
