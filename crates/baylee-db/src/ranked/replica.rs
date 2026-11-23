//! The durable match, game and last authorized generation on one gateway.
use super::{ConnectionTrait, DbErr, Uuid, statement};

/// Install or confirm the placement Central supplied. The game UUID is immutable,
/// and the role may change only in a strictly newer generation.
///
/// # Errors
/// Storage failure or an out-of-range generation.
pub async fn bind(
    db: &impl ConnectionTrait,
    match_id: Uuid,
    game: Uuid,
    generation: u64,
    primary: bool,
) -> Result<bool, DbErr> {
    let generation = valid_generation(generation)?;
    Ok(db.query_one_raw(statement(db,
        "INSERT INTO ranked_run (match_id,game,generation,primary_host)
         VALUES ($1,$2,$3,$4)
         ON CONFLICT (match_id) DO UPDATE SET generation=excluded.generation,primary_host=excluded.primary_host,seen_at=clock_timestamp()
         WHERE ranked_run.game=excluded.game AND
             (ranked_run.generation < excluded.generation OR
              (ranked_run.generation=excluded.generation AND ranked_run.primary_host=excluded.primary_host))
         RETURNING game",
        vec![match_id.into(), game.into(), generation.into(), primary.into()],
    )).await?.is_some())
}

/// A gateway can restart after Central settled the game but before deleting its
/// local binding. Remove at most 32 such rows per complete assigned-work list.
/// A full day since the last local placement also outlasts Central's 12h lease.
///
/// # Errors
/// Database failure.
pub async fn reap_absent(db: &impl ConnectionTrait, assigned: &[Uuid]) -> Result<(), DbErr> {
    if assigned.len() >= 64 {
        return Ok(());
    }
    db.execute_raw(statement(db,
        "WITH stale AS (
             SELECT match_id,generation FROM ranked_run WHERE generation IS NOT NULL
             AND seen_at < clock_timestamp()-interval '1 day' AND NOT(match_id = ANY($1::uuid[]))
             ORDER BY seen_at,match_id LIMIT 32 FOR UPDATE SKIP LOCKED)
         DELETE FROM ranked_run r USING stale s WHERE r.match_id=s.match_id AND r.generation=s.generation",
        vec![assigned.to_vec().into()],
    )).await?;
    Ok(())
}

/// Remove this exact binding once Central confirms the terminal disposition.
/// A delayed cleanup from a previous primary cannot remove its successor.
///
/// # Errors
/// Storage failure or an out-of-range generation.
pub async fn finish(
    db: &impl ConnectionTrait,
    match_id: Uuid,
    game: Uuid,
    generation: u64,
) -> Result<bool, DbErr> {
    Ok(db
        .execute_raw(statement(
            db,
            "DELETE FROM ranked_run WHERE match_id=$1 AND game=$2 AND generation=$3",
            vec![
                match_id.into(),
                game.into(),
                valid_generation(generation)?.into(),
            ],
        ))
        .await?
        .rows_affected()
        == 1)
}

fn valid_generation(generation: u64) -> Result<i64, DbErr> {
    i64::try_from(generation)
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| DbErr::Custom("invalid ranked generation".to_owned()))
}
