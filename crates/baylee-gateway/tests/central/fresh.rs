//! An account registered at Central, never a legacy account, plays on the
//! official gateway: a registration proof, the registration with its recovery
//! wrapper as the website makes it (through the identity service's browser
//! entry, which the browser listener calls after its own checks), the native
//! sign-in, and the bridge gateway making the account a projection
//! (baylee-central `docs/web-accounts.md`).
use super::{Endpoint, common};
use baylee_account::{
    Account,
    wire::{Request, Response},
};
use central_crypto::{
    Context,
    opaque::Enrollment,
    vault::{MasterKey, ObjectContext, RecoveryCode},
};
use central_identity::IdentityService;
use std::sync::{Arc, OnceLock};
use tokio::runtime::Runtime;

/// The plaintext the website seals as object 1, byte for byte.
const DOCUMENT: &[u8] = br#"{"version":1,"gateways":["https://baylee.acevik.de"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#;

/// Kept only in memory so later fixture checks can sign in to the same account.
pub(super) fn password() -> &'static str {
    static PASSWORD: OnceLock<String> = OnceLock::new();
    PASSWORD.get_or_init(|| {
        if std::env::var("BAYLEE_CENTRAL_DEMO").as_deref() == Ok("1") {
            std::env::var("BAYLEE_CENTRAL_DEMO_PASSWORD")
                .expect("the demo owner must supply a per-run password through its environment")
        } else {
            use base64::Engine as _;
            let mut bytes = [0_u8; 32];
            getrandom::fill(&mut bytes).expect("synthetic account password entropy");
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
        }
    })
}

/// `service` is the one the test serves: the registration reaches it through
/// its browser entry, the native sign-in over HTTP.
pub fn run(
    runtime: &Runtime,
    service: &Arc<IdentityService>,
    endpoint: &Endpoint,
    gateway: &common::Gateway,
    bridge: &baylee_account::Token,
    operator: &sea_orm::DatabaseConnection,
) {
    let origin = &format!("http://127.0.0.1:{}", gateway.port);
    let demo = std::env::var("BAYLEE_CENTRAL_DEMO").as_deref() == Ok("1");
    let document = if demo {
        super::demo::document(origin)
    } else {
        DOCUMENT.to_vec()
    };
    let account = register(runtime, service, endpoint, "fresh_player", &document);
    assert_eq!(
        account.data().gateways,
        vec![if demo {
            origin.as_str()
        } else {
            central_api::LEGACY_ORIGIN
        }]
    );
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
    super::challenges::run(runtime, endpoint, gateway, &account, bridge, operator);
}

/// Enrol a synthetic account through the real browser protocol, then sign in natively.
pub(super) fn register(
    runtime: &Runtime,
    service: &Arc<IdentityService>,
    endpoint: &Endpoint,
    username: &str,
    document: &[u8],
) -> Account {
    let website = |request: &Request| {
        let request = serde_json::from_slice(&serde_json::to_vec(request).unwrap()).unwrap();
        runtime
            .block_on(service.handle_browser(request, None))
            .unwrap()
            .response
    };
    let password = password().to_owned();
    let register = super::registration::request(website, username, username);
    let Response::Enrollment { identity, ticket } = website(&register) else {
        panic!("a valid proof registers a free name");
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
    let document = master.seal_object(object, document).unwrap();
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
    Account::login(endpoint.clone(), username.into(), password, Vec::new()).unwrap()
}
