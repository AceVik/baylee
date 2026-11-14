//! Signing Baylee's client in from the signed-in website (baylee-central
//! `docs/web-accounts.md` §"Pairing"; Baylee `docs/central-login.md`
//! §"Signing in from the website"), through the client's own crates: an
//! account registered as the website makes it; the client's password
//! sign-in storing the key check the website's approval needs; the
//! website's browser session (through the identity service's browser entry,
//! which the browser listener calls after its own checks) starting a pairing;
//! the link it opens read by the client's parser; the claim, the page's
//! approval with the master key sealed under the link's secret and proven
//! under the key check, and the redeem through `baylee-account`: the account
//! restored without its password, the password's session replaced. A second
//! claim of one link is refused, a pairing the page cancels ends the
//! client's, and a no to the client's question leaves no native session.
#![forbid(unsafe_code)]
use super::{Endpoint, common};
use baylee_account::{
    Account, Error, Token,
    wire::{Identity, Request, Response},
};
use baylee_client_core::deeplink::{self, Link, Origins};
use central_crypto::{
    Context,
    opaque::{Enrollment, Login},
    pairing::{self, PairingSecret, SealBinding},
    vault::{KeyCheck, MasterKey, ObjectContext, RecoveryCode},
};
use central_identity::{BrowserReply, IdentityService};
use sea_orm::{ConnectionTrait as _, DatabaseConnection, DbBackend, Statement};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// The plaintext the website seals as object 1, byte for byte.
const DOCUMENT: &[u8] = br#"{"version":1,"gateways":["https://pairing.example.org"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#;
const USERNAME: &str = "pairing_player";
const PASSWORD: &str = "a paired player's long password";

/// The website, as its pages reach the identity service.
struct Website<'a> {
    runtime: &'a Runtime,
    service: &'a Arc<IdentityService>,
}

impl Website<'_> {
    fn call(&self, request: &Request, cookie: Option<&Token>) -> Result<BrowserReply, Error> {
        let request = serde_json::from_slice(&serde_json::to_vec(request).unwrap()).unwrap();
        self.runtime
            .block_on(self.service.handle_browser(request, cookie))
    }

    fn answer(&self, request: &Request, cookie: Option<&Token>) -> Result<Response, Error> {
        self.call(request, cookie).map(|reply| reply.response)
    }

    /// The website's registration: the account, its context and the master
    /// key the page made for it.
    fn register(&self, endpoint: &Endpoint) -> (Identity, Context, MasterKey) {
        let register = super::registration::request(
            |request| self.answer(request, None).unwrap(),
            USERNAME,
            "Pair_Player",
        );
        let Ok(Response::Enrollment { identity, ticket }) = self.answer(&register, None) else {
            panic!("a valid proof registers a free name");
        };
        let context = Context::new(endpoint.issuer(), identity.account, identity.epoch).unwrap();
        let (enrollment, message) = Enrollment::start(PASSWORD.into(), context).unwrap();
        let start = Request::EnrollmentStart {
            ticket: copy(&ticket),
            message,
        };
        let Ok(Response::EnrollmentChallenge { message }) = self.answer(&start, None) else {
            panic!("the enrollment starts");
        };
        let result = enrollment.finish(&message).unwrap();
        let master = MasterKey::generate().unwrap();
        let recovery = RecoveryCode::generate()
            .unwrap()
            .wrap(context, &master)
            .unwrap();
        let object = ObjectContext::new(context, identity.account, 1).unwrap();
        let finish = Request::EnrollmentFinish {
            ticket,
            record: result.upload,
            wrapped_master: result
                .wrapping_key
                .wrap(&master)
                .unwrap()
                .as_bytes()
                .to_vec(),
            document: master.seal_object(object, DOCUMENT).unwrap(),
            recovery: Some(recovery.as_bytes().to_vec()),
        };
        assert!(matches!(self.answer(&finish, None), Ok(Response::Done)));
        (identity, context, master)
    }

    /// The website's own sign-in: the browser session in its cookie.
    fn sign_in(&self, identity: Identity, context: Context) -> Token {
        let (login, message) = Login::start(PASSWORD.into(), context).unwrap();
        let Ok(Response::LoginChallenge { attempt, message }) =
            self.answer(&Request::LoginStart { identity, message }, None)
        else {
            panic!("a challenge");
        };
        let result = login.finish(&message).unwrap();
        let finish = Request::LoginFinish {
            attempt,
            message: result.finalization,
        };
        self.call(&finish, None)
            .unwrap()
            .session
            .expect("a session for the cookie")
    }

    /// "Sign in Baylee on this computer": a pairing, and the link the page
    /// opens with its own secret.
    fn start(&self, cookie: &Token) -> Page {
        let Ok(Response::PairStarted { handle, expires_at }) =
            self.answer(&Request::PairStart, Some(cookie))
        else {
            panic!("a pairing starts");
        };
        Page {
            handle,
            expires_at,
            secret: PairingSecret::generate().unwrap(),
        }
    }

    /// What the page reads of the client's claim, checked as it checks it.
    fn claimed(&self, cookie: &Token, page: &Page, endpoint: &Endpoint) -> Claimed {
        let Ok(Response::PairClaimed {
            public_key,
            challenge,
            mac,
        }) = self.answer(
            &Request::PairStatus {
                handle: copy(&page.handle),
            },
            Some(cookie),
        )
        else {
            panic!("the claim");
        };
        let claimed = Claimed {
            public_key: public_key.try_into().unwrap(),
            challenge: challenge.try_into().unwrap(),
        };
        assert!(
            pairing::claim_ok(
                &page.secret,
                &endpoint.issuer(),
                &page.bytes(),
                &claimed.public_key,
                &claimed.challenge,
                &mac,
            ),
            "the claim's MAC is under the link's secret"
        );
        claimed
    }

    /// Approve: the master key sealed to the claim's key under the link's
    /// secret, and the proof under the key check.
    fn approve(
        &self,
        cookie: &Token,
        page: &Page,
        claimed: &Claimed,
        master: &MasterKey,
        check: &KeyCheck,
        context: Context,
    ) {
        let binding = SealBinding {
            context,
            handle: page.bytes(),
            expires_at: page.expires_at,
        };
        let sealed = master
            .seal_for_pairing(&page.secret, &claimed.public_key, &binding)
            .unwrap();
        let proof = pairing::approval_proof(
            check,
            &page.bytes(),
            &claimed.public_key,
            &claimed.challenge,
            &sealed,
        );
        let approve = Request::PairApprove {
            handle: copy(&page.handle),
            sealed: sealed.to_vec(),
            proof: proof.to_vec(),
        };
        assert!(matches!(
            self.answer(&approve, Some(cookie)),
            Ok(Response::Done)
        ));
    }
}

/// A pairing as the page holds it.
struct Page {
    handle: Token,
    expires_at: i64,
    secret: PairingSecret,
}

impl Page {
    fn bytes(&self) -> [u8; 32] {
        *self.handle.to_secret_bytes()
    }

    /// `baylee://pair?h=<handle>&s=<S>`, as the page writes it.
    fn link(&self) -> String {
        let (h, s) = (
            baylee_seal::b64::url::encode(&self.bytes()),
            baylee_seal::b64::url::encode(self.secret.as_bytes()),
        );
        format!("baylee://pair?h={h}&s={s}")
    }
}

/// The client's claim, as the page reads it.
struct Claimed {
    public_key: [u8; 32],
    challenge: [u8; 32],
}

fn copy(token: &Token) -> Token {
    Token::from_secret_bytes(token.to_secret_bytes())
}

/// The account's live native sessions.
fn natives(runtime: &Runtime, operator: &DatabaseConnection) -> i64 {
    runtime
        .block_on(operator.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM central_identity.sessions s
             JOIN central_identity.accounts a ON a.id = s.account_id
             WHERE a.username_key = $1 AND s.kind = 'native'
             AND s.expires_at > pg_catalog.clock_timestamp()",
            [USERNAME.into()],
        )))
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

pub(super) fn run(
    runtime: &Runtime,
    service: &Arc<IdentityService>,
    operator: &DatabaseConnection,
    endpoint: &Endpoint,
    gateway: &common::Gateway,
) {
    let origin = &format!("http://127.0.0.1:{}", gateway.port);
    let website = Website { runtime, service };
    let (identity, context, master) = website.register(endpoint);
    let check = master.key_check(context).unwrap();

    // The client's password sign-in stores the key check the website's
    // approval needs: without it no pairing starts.
    let password_session = Account::login(
        endpoint.clone(),
        USERNAME.into(),
        PASSWORD.into(),
        Vec::new(),
    )
    .unwrap();
    let cookie = website.sign_in(identity, context);
    assert!(matches!(
        website.answer(&Request::KeyCheckStatus, Some(&cookie)),
        Ok(Response::KeyCheck { set: true })
    ));

    // The page starts a pairing and opens its link; the client reads it.
    let page = website.start(&cookie);
    let Some(Link::Pair(link)) = deeplink::read(&page.link(), Origins::Public) else {
        panic!("the page's link is one the client reads");
    };
    let claim = Account::claim_pairing(endpoint.clone(), link.handle(), link.secret()).unwrap();
    // Another program with the same link comes second.
    assert_eq!(
        Account::claim_pairing(endpoint.clone(), link.handle(), link.secret()).err(),
        Some(Error::Conflict)
    );
    let claimed = website.claimed(&cookie, &page, endpoint);
    assert_eq!(
        pairing::comparison_code(&page.bytes(), &claimed.public_key),
        claim.comparison_code(),
        "both screens show one code"
    );
    assert!(!claim.approved().unwrap(), "not approved yet");
    website.approve(&cookie, &page, &claimed, &master, &check, context);
    assert!(claim.approved().unwrap());

    // The redeem: the account named for the client's question, then the
    // account restored without its password.
    let paired = claim.redeem().unwrap();
    assert_eq!(paired.username(), USERNAME);
    let handle = paired.profile_handle();
    assert!(handle.starts_with("Pair_Player#"), "{handle}");
    assert_eq!(paired.who(), format!("{handle} ({USERNAME})"));
    assert_eq!(
        password_session.check_active().err(),
        Some(Error::Rejected),
        "the paired session replaced the password's"
    );
    drop(password_session);
    let account = paired.accept().unwrap();
    assert_eq!(account.data().gateways, ["https://pairing.example.org"]);
    account.check_active().unwrap();
    // A gateway lets the paired account in, as after a password sign-in.
    let admission = account.admit(origin).unwrap();
    let (status, body) = common::http(gateway.port, "GET", "/me", Some(&admission.token), "");
    assert_eq!(status, 200, "{body}");
    account.logout().unwrap();
    assert_eq!(natives(runtime, operator), 0);

    // The page cancels: the client's pairing has ended.
    let page = website.start(&cookie);
    let claim =
        Account::claim_pairing(endpoint.clone(), &page.bytes(), page.secret.as_bytes()).unwrap();
    assert!(matches!(
        website.answer(
            &Request::PairCancel {
                handle: copy(&page.handle)
            },
            Some(&cookie)
        ),
        Ok(Response::Done)
    ));
    assert_eq!(claim.approved().err(), Some(Error::Rejected));

    // No to the client's question: the session the redeem gave ends.
    let page = website.start(&cookie);
    let claim =
        Account::claim_pairing(endpoint.clone(), &page.bytes(), page.secret.as_bytes()).unwrap();
    let claimed = website.claimed(&cookie, &page, endpoint);
    website.approve(&cookie, &page, &claimed, &master, &check, context);
    assert!(claim.approved().unwrap());
    let paired = claim.redeem().unwrap();
    assert_eq!(natives(runtime, operator), 1);
    paired.decline().unwrap();
    assert_eq!(natives(runtime, operator), 0, "no native session left");

    // A link whose pairing was redeemed claims nothing.
    assert_eq!(
        Account::claim_pairing(endpoint.clone(), &page.bytes(), page.secret.as_bytes()).err(),
        Some(Error::Rejected)
    );
}
