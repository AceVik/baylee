//! The gateway's link to Baylee Central by a pasted token (migration
//! 000025): read at start, set and removed by the admin console. The token
//! leaves this module only as bytes, to the gateway's Central client.
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement, prelude::Uuid};

/// The stored link.
pub struct StoredLink {
    /// The token, 32 bytes. Never log it.
    pub token: [u8; 32],
    /// The gateway's id at Central.
    pub gateway: [u8; 16],
    /// Its origin, as Central named it.
    pub origin: String,
    /// When it was set, Unix seconds.
    pub set_at: i64,
    /// Who set it: the console's admin name, or a Central account's id.
    pub set_by: String,
}

/// The stored link, if there is one.
///
/// # Errors
/// Storage failure, or a row the schema would not hold.
pub async fn link(db: &impl ConnectionTrait) -> Result<Option<StoredLink>, DbErr> {
    let Some(row) = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT token, gateway, origin, floor(extract(epoch FROM set_at))::bigint AS set_at, set_by
             FROM central_link WHERE id = 1",
        ))
        .await?
    else {
        return Ok(None);
    };
    let token: Vec<u8> = row.try_get("", "token")?;
    let gateway: Uuid = row.try_get("", "gateway")?;
    Ok(Some(StoredLink {
        token: token
            .try_into()
            .map_err(|_| DbErr::Custom("a stored token of another length".into()))?,
        gateway: *gateway.as_bytes(),
        origin: row.try_get("", "origin")?,
        set_at: row.try_get("", "set_at")?,
        set_by: row.try_get("", "set_by")?,
    }))
}

/// Store `token` as the link, replacing any earlier one.
///
/// # Errors
/// Storage failure, or values the schema refuses.
pub async fn set_link(
    db: &impl ConnectionTrait,
    token: &[u8; 32],
    gateway: [u8; 16],
    origin: &str,
    set_by: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO central_link (id, token, gateway, origin, set_by) VALUES (1, $1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE SET token = EXCLUDED.token, gateway = EXCLUDED.gateway,
             origin = EXCLUDED.origin, set_at = clock_timestamp(), set_by = EXCLUDED.set_by",
        [
            token.to_vec().into(),
            Uuid::from_bytes(gateway).into(),
            origin.into(),
            set_by.into(),
        ],
    ))
    .await?;
    Ok(())
}

/// Remove the link; `true` when there was one.
///
/// # Errors
/// Storage failure.
pub async fn clear_link(db: &impl ConnectionTrait) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(Statement::from_string(
            DbBackend::Postgres,
            "DELETE FROM central_link WHERE id = 1",
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}
