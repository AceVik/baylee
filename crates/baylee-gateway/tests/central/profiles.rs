//! The bridge's half of display names at Central: a move carries the legacy
//! account's name and tag in `LegacyReserve`; an account that moved before
//! Central kept them gets them with `LegacyProfile` at its next Central
//! sign-in here, once; a name that breaks the rule stays behind, its tag
//! does not.
use super::{Account, Credentials, Endpoint, LegacyHttp, common};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use tokio::runtime::Runtime;

const PASSWORD: &str = "a-very-fine-password";

/// The gateway's id, display name and tag of `username`.
fn here(gateway: &common::Gateway, username: &str) -> (String, String, i64) {
    (
        gateway.text(&format!(
            "SELECT id::text FROM account WHERE username = '{username}'"
        )),
        gateway.text(&format!(
            "SELECT display_name FROM account WHERE username = '{username}'"
        )),
        gateway.scalar(&format!(
            "SELECT tag::bigint FROM account WHERE username = '{username}'"
        )),
    )
}

/// Central's display name and tag of the legacy account `legacy`.
fn central(
    runtime: &Runtime,
    operator: &DatabaseConnection,
    legacy: &str,
) -> (Option<String>, Option<i32>) {
    let row = runtime
        .block_on(operator.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT display_name, tag FROM central_identity.accounts WHERE legacy_id = $1",
            [legacy.into()],
        )))
        .unwrap()
        .unwrap();
    (
        row.try_get("", "display_name").unwrap(),
        row.try_get("", "tag").unwrap(),
    )
}

/// Signs in at Central as the client does and is admitted here.
fn admitted(endpoint: &Endpoint, origin: &str, username: &str) {
    let account = Account::login(
        endpoint.clone(),
        username.into(),
        PASSWORD.into(),
        Vec::new(),
    )
    .unwrap();
    account.admit(origin).unwrap();
    account.logout().unwrap();
}

pub(super) fn run(
    runtime: &Runtime,
    operator: &DatabaseConnection,
    gateway: &common::Gateway,
    endpoint: &Endpoint,
    origin: &str,
) {
    // `synthetic` moved in `run` (e2e_central.rs) with this bridge.
    let (legacy, name, tag) = here(gateway, "synthetic");
    let tag = i32::try_from(tag).unwrap();
    assert_eq!(
        central(runtime, operator, &legacy),
        (Some(name.clone()), Some(tag)),
        "a move carries the name and the tag"
    );

    // As if it had moved before Central kept them.
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.accounts SET display_name = NULL, display_name_at = NULL,
             tag = NULL WHERE legacy_id = $1",
            [legacy.as_str().into()],
        )))
        .unwrap();
    admitted(endpoint, origin, "synthetic");
    assert_eq!(
        central(runtime, operator, &legacy),
        (Some(name.clone()), Some(tag)),
        "the next Central sign-in here carries them"
    );
    // Once only: a later sign-in changes nothing at Central, whatever this side holds.
    gateway.sql(&format!(
        "UPDATE account SET display_name = 'Renamed_Here' WHERE id = '{legacy}'::uuid"
    ));
    admitted(endpoint, origin, "synthetic");
    assert_eq!(
        central(runtime, operator, &legacy),
        (Some(name.clone()), Some(tag))
    );
    gateway.sql(&format!(
        "UPDATE account SET display_name = '{name}' WHERE id = '{legacy}'::uuid"
    ));

    // A name from before the rule of 15.09.2026: the tag moves, the name stays behind.
    common::login(gateway.port, "old-name", "OldName");
    gateway.sql("UPDATE account SET display_name = '_old' WHERE username = 'old-name'");
    let (old, _, old_tag) = here(gateway, "old-name");
    LegacyHttp::development(origin)
        .unwrap()
        .enroll(&Credentials {
            username: "old-name".into(),
            password: PASSWORD.into(),
        })
        .unwrap();
    assert_eq!(
        central(runtime, operator, &old),
        (None, Some(i32::try_from(old_tag).unwrap()))
    );
}
