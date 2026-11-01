//! A gateway account's move to Baylee Central while Central's own terms are
//! in force and not yet accepted (the owner's sign-in, 22.10.2026): the move's
//! close waits for them. The sign-in succeeds with the move held and holds no
//! password, the client shows Central's terms, and their acceptance finishes
//! the move; the gateway admits the account after that, never before.
use super::{Account, PASSWORD, Served, common, cutover};
use baylee_account::terms::{self, TermsDocument, TermsSubject, TermsText, TermsVersion};
use central_postgres::{AccountStore, LegacyStore};
use sea_orm::DatabaseConnection;
use tokio::runtime::Runtime;

const USERNAME: &str = "held-move";

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

pub(super) fn run(
    runtime: &Runtime,
    store: &AccountStore,
    legacy: &LegacyStore,
    operator: &DatabaseConnection,
) {
    let terms = document();
    let served = super::serve(runtime, store, legacy, "central-held", Some(terms.clone()));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        held(&served, &terms, runtime, operator);
    }));
    served.close(runtime);
    if let Err(failure) = result {
        std::panic::resume_unwind(failure);
    }
}

fn held(served: &Served, terms: &TermsDocument, runtime: &Runtime, operator: &DatabaseConnection) {
    let (endpoint, origin, port) = (&served.endpoint, &served.origin, served.gateway.port);
    // An account from before the cutover, in Central's legacy store: its
    // legacy sign-in needs no terms, its close does.
    common::login(port, USERNAME, "Held");
    let source_id = served.gateway.text(&format!(
        "SELECT id::text FROM account WHERE username_key = '{USERNAME}'"
    ));
    assert_eq!(cutover(&served.gateway, runtime, operator), 1);

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
    let admitted = account.admit(origin).unwrap();
    assert_eq!(
        served.gateway.scalar(&format!(
            "SELECT count(*) FROM account WHERE id = '{source_id}'::uuid AND central_only AND password_hash IS NULL"
        )),
        1,
        "the gateway account is linked at its first admission"
    );
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
