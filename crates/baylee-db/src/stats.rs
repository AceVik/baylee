//! What the admin console counts (`GET /admin/stats` on a gateway): numbers
//! over the whole store, never a row, an id, a name or an address.
//!
//! One hand-written statement, as in [`crate::invites`]: a handful of
//! `count(*) FILTER (…)` over `account`, `session_token`, `game_record` and
//! `invite`, so the console's every refresh is one round trip whatever it
//! asks. Every answer is a count or a sum; [`Stats`] has no field that could
//! hold anything else.

use sea_orm::{ConnectionTrait, DbErr, Statement};
use time::OffsetDateTime;

/// How many of something there were since three moments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Since {
    /// Since midnight UTC today.
    pub today_utc: i64,
    /// In the last 7 × 24 hours.
    pub last_7d: i64,
    /// In the last 30 × 24 hours.
    pub last_30d: i64,
}

/// Everything the store can say about how many, at one moment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Accounts that are not guests.
    pub registered: i64,
    /// Of those, the ones with an address.
    pub with_email: i64,
    /// Of those, the ones whose address was confirmed.
    pub confirmed_email: i64,
    /// Registered accounts made in each window.
    pub registered_since: Since,
    /// Registered accounts a closed-beta key let in, that still exist.
    pub admitted_by_key: i64,
    /// Guest accounts (each one a live session leads to, give or take the
    /// next sweep).
    pub guests: i64,
    /// Sessions that have not lapsed.
    pub sessions_live: i64,
    /// Accounts with at least one such session.
    pub accounts_signed_in: i64,
    /// Games with a record (#315): every hosted game whose engine sent a
    /// first piece of it.
    pub games_recorded: i64,
    /// Recorded games started in each window.
    pub games_started: Since,
    /// Recorded games whose record is complete: the game ended and its
    /// engine said so.
    pub games_finished: i64,
    /// Of those, the ones finished in each window.
    pub games_finished_since: Since,
    /// Closed-beta keys ever made.
    pub invites_total: i64,
    /// Keys that admit somebody now: not revoked, not expired, a use left.
    pub invites_active: i64,
    /// Keys not revoked and not expired, with no use left.
    pub invites_used_up: i64,
    /// Keys not revoked whose expiry has passed.
    pub invites_expired: i64,
    /// Keys revoked.
    pub invites_revoked: i64,
    /// The uses the active keys have left, together.
    pub invite_uses_left: i64,
}

/// Midnight UTC of the day `now` is in.
#[must_use]
pub fn start_of_day(now: OffsetDateTime) -> OffsetDateTime {
    now.to_offset(time::UtcOffset::UTC)
        .replace_time(time::Time::MIDNIGHT)
}

/// Counts everything [`Stats`] holds, as of `now`.
///
/// # Errors
///
/// When the query fails.
pub async fn read(db: &impl ConnectionTrait, now: OffsetDateTime) -> Result<Stats, DbErr> {
    // $1 midnight UTC, $2 a week ago, $3 thirty days ago, $4 now.
    const SQL: &str = "\
        SELECT a.*, s.*, g.*, i.* FROM \
        (SELECT \
            count(*) FILTER (WHERE NOT guest) AS registered, \
            count(*) FILTER (WHERE NOT guest AND email IS NOT NULL) AS with_email, \
            count(*) FILTER (WHERE NOT guest AND email IS NOT NULL \
                AND confirmed_at IS NOT NULL) AS confirmed_email, \
            count(*) FILTER (WHERE NOT guest AND created_at >= $1) AS registered_today, \
            count(*) FILTER (WHERE NOT guest AND created_at >= $2) AS registered_7d, \
            count(*) FILTER (WHERE NOT guest AND created_at >= $3) AS registered_30d, \
            count(*) FILTER (WHERE NOT guest AND invite_id IS NOT NULL) AS admitted_by_key, \
            count(*) FILTER (WHERE guest) AS guests \
         FROM account) a, \
        (SELECT count(*) AS sessions_live, count(DISTINCT account_id) AS accounts_signed_in \
         FROM session_token WHERE expires_at > $4) s, \
        (SELECT \
            count(*) AS games_recorded, \
            count(*) FILTER (WHERE started_at >= $1) AS started_today, \
            count(*) FILTER (WHERE started_at >= $2) AS started_7d, \
            count(*) FILTER (WHERE started_at >= $3) AS started_30d, \
            count(*) FILTER (WHERE complete) AS games_finished, \
            count(*) FILTER (WHERE complete AND ended_at >= $1) AS finished_today, \
            count(*) FILTER (WHERE complete AND ended_at >= $2) AS finished_7d, \
            count(*) FILTER (WHERE complete AND ended_at >= $3) AS finished_30d \
         FROM game_record) g, \
        (SELECT \
            count(*) AS invites_total, \
            count(*) FILTER (WHERE revoked_at IS NULL AND uses_left > 0 \
                AND (expires_at IS NULL OR expires_at > $4)) AS invites_active, \
            count(*) FILTER (WHERE revoked_at IS NULL AND uses_left = 0 \
                AND (expires_at IS NULL OR expires_at > $4)) AS invites_used_up, \
            count(*) FILTER (WHERE revoked_at IS NULL AND expires_at <= $4) AS invites_expired, \
            count(*) FILTER (WHERE revoked_at IS NOT NULL) AS invites_revoked, \
            coalesce(sum(uses_left) FILTER (WHERE revoked_at IS NULL AND uses_left > 0 \
                AND (expires_at IS NULL OR expires_at > $4)), 0)::bigint AS invite_uses_left \
         FROM invite) i";
    let week = now - time::Duration::days(7);
    let month = now - time::Duration::days(30);
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            SQL,
            [
                start_of_day(now).into(),
                week.into(),
                month.into(),
                now.into(),
            ],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("the counts answered no row".into()))?;
    let get = |column: &str| row.try_get::<i64>("", column);
    Ok(Stats {
        registered: get("registered")?,
        with_email: get("with_email")?,
        confirmed_email: get("confirmed_email")?,
        registered_since: Since {
            today_utc: get("registered_today")?,
            last_7d: get("registered_7d")?,
            last_30d: get("registered_30d")?,
        },
        admitted_by_key: get("admitted_by_key")?,
        guests: get("guests")?,
        sessions_live: get("sessions_live")?,
        accounts_signed_in: get("accounts_signed_in")?,
        games_recorded: get("games_recorded")?,
        games_started: Since {
            today_utc: get("started_today")?,
            last_7d: get("started_7d")?,
            last_30d: get("started_30d")?,
        },
        games_finished: get("games_finished")?,
        games_finished_since: Since {
            today_utc: get("finished_today")?,
            last_7d: get("finished_7d")?,
            last_30d: get("finished_30d")?,
        },
        invites_total: get("invites_total")?,
        invites_active: get("invites_active")?,
        invites_used_up: get("invites_used_up")?,
        invites_expired: get("invites_expired")?,
        invites_revoked: get("invites_revoked")?,
        invite_uses_left: get("invite_uses_left")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_starts_at_midnight_utc_whatever_the_offset() {
        let at = OffsetDateTime::from_unix_timestamp(1_790_000_000).unwrap(); // 2026-09-21 14:13 UTC
        let midnight = start_of_day(at);
        assert_eq!(midnight.unix_timestamp(), 1_789_948_800);
        let elsewhere = at.to_offset(time::UtcOffset::from_hms(-9, 0, 0).unwrap());
        assert_eq!(start_of_day(elsewhere), midnight);
    }
}
