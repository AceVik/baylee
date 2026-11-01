//! Who a gateway lets in, as the console chose it (migration 000027): read
//! at start, set by an admin. The mode is kept as its name; the gateway
//! reads it into its own type.
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

/// The stored choice.
pub struct StoredAuthMode {
    /// `guests`, `central` or `mixed`.
    pub mode: String,
    /// When it was set, Unix seconds.
    pub set_at: i64,
    /// Who set it: the console's admin name, or a Central account's id.
    pub set_by: String,
}

/// The stored choice, if an admin made one.
///
/// # Errors
/// Storage failure.
pub async fn auth_mode(db: &impl ConnectionTrait) -> Result<Option<StoredAuthMode>, DbErr> {
    let Some(row) = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT mode, floor(extract(epoch FROM set_at))::bigint AS set_at, set_by
             FROM auth_mode WHERE id = 1",
        ))
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(StoredAuthMode {
        mode: row.try_get("", "mode")?,
        set_at: row.try_get("", "set_at")?,
        set_by: row.try_get("", "set_by")?,
    }))
}

/// Store `mode` as the choice, replacing any earlier one.
///
/// # Errors
/// Storage failure, or a mode or name the schema refuses.
pub async fn set_auth_mode(
    db: &impl ConnectionTrait,
    mode: &str,
    set_by: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO auth_mode (id, mode, set_by) VALUES (1, $1, $2)
         ON CONFLICT (id) DO UPDATE SET mode = EXCLUDED.mode, set_at = clock_timestamp(),
             set_by = EXCLUDED.set_by",
        [mode.into(), set_by.into()],
    ))
    .await?;
    Ok(())
}
