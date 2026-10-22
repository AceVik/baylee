//! A gateway account's move to Baylee Central while Central's own terms are
//! in force and not yet accepted (the owner's sign-in, 22.10.2026): every
//! sign-in failed, since the move asks for gateway tickets and Central gives
//! none before its terms are accepted. The sign-in now succeeds with the move
//! held, the client shows Central's terms, and their acceptance finishes the
//! move; the gateway admits the account after that, never before.
use super::{
    Account, AccountData, CentralHttp, Credentials, LegacyData, LegacyHttp, Served, common, sdk,
};
use baylee_account::terms::{self, TermsDocument, TermsSubject, TermsText, TermsVersion};
use central_postgres::AccountStore;
use tokio::runtime::Runtime;

const USERNAME: &str = "held-move";
const PASSWORD: &str = "a-very-fine-password";

/// Central's terms in force on this second service.
fn document() -> TermsDocument {
    let texts: Vec<TermsText> = ["de", "en"]
        .iter()
        .map(|lang| TermsText {
            lang: (*lang).into(),
            updated: Some("2026-10-22".into()),
            markdown: format!("# Central ({lang})\n\nSynthetic terms.\n"),
        })
        .collect();
    TermsDocument {
        version: TermsVersion {
            version: "2026-10-held".into(),
            digest: terms::digest(&texts),
        },
        texts,
    }
}

pub(super) fn run(runtime: &Runtime, store: &AccountStore) {
    let terms = document();
    let served = super::serve(runtime, store, "central-held", Some(terms.clone()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        held(&served, &terms);
    }));
    served.close(runtime);
    if let Err(failure) = result {
        std::panic::resume_unwind(failure);
    }
}

fn held(served: &Served, terms: &TermsDocument) {
    let (endpoint, origin, port) = (&served.endpoint, &served.origin, served.gateway.port);
    // Enrolled at Central, the move not linked yet: where the owner's first
    // sign-in stopped.
    common::login(port, USERNAME, "Held");
    let legacy = LegacyHttp::development(origin).unwrap();
    let credentials = Credentials {
        username: USERNAME.into(),
        password: PASSWORD.into(),
    };
    let enrollment = legacy.enroll(&credentials).unwrap();
    let source_id = enrollment.snapshot.account_id.clone();
    let data = AccountData {
        version: 1,
        gateways: vec![origin.clone()],
        preferences: enrollment.snapshot.preferences,
        chat: baylee_account::chat::ChatArchive::default(),
        legacy: Some(LegacyData {
            digest: enrollment.snapshot.digest,
            complete: false,
            gateway: origin.clone(),
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
        PASSWORD.into(),
        &data.write().unwrap(),
    )
    .unwrap();

    let mut account = Account::login(
        endpoint.clone(),
        USERNAME.into(),
        PASSWORD.into(),
        Vec::new(),
    )
    .expect("Central's open terms hold the move, not the sign-in");
    assert!(account.migration_held());
    assert!(account.central_terms_open());
    assert!(!account.data().legacy.as_ref().unwrap().complete);
    let status = account.central_terms_status().unwrap();
    assert!(!status.accepted);
    assert_eq!(status.current.as_ref(), Some(&terms.version));
    assert!(
        account.admit(origin).is_err(),
        "no gateway before Central's terms"
    );
    assert!(
        account
            .save(Some(serde_json::json!({"early": true})), None)
            .is_err(),
        "Central keeps no write before its terms"
    );
    let shown = account.central_terms().unwrap().expect("terms in force");
    assert_eq!(shown, *terms);
    account
        .accept_terms(TermsSubject::Central, &shown.version)
        .unwrap();
    assert!(!account.central_terms_open());
    account.finish_migration().unwrap();
    assert!(!account.migration_held());
    assert!(account.data().legacy.as_ref().unwrap().complete);
    assert_eq!(
        served.gateway.scalar(&format!(
            "SELECT count(*) FROM account WHERE id = '{source_id}'::uuid AND central_only AND password_hash IS NULL"
        )),
        1,
        "the gateway account moved"
    );
    let admitted = account.admit(origin).unwrap();
    assert_eq!(admitted.central_terms_stale, None);
    assert_eq!(
        common::http(port, "GET", "/me", Some(&admitted.token), "").0,
        200
    );
    account.logout().unwrap();

    // The next sign-in finds the terms accepted and the move done.
    let again = Account::login(
        endpoint.clone(),
        USERNAME.into(),
        PASSWORD.into(),
        Vec::new(),
    )
    .unwrap();
    assert!(!again.migration_held());
    assert!(!again.central_terms_open());
    assert!(again.data().legacy.as_ref().unwrap().complete);
    again.logout().unwrap();
}
