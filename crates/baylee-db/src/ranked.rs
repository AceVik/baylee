//! Restart-safe bindings between a Central match and the single game its host may launch.
//! The existing private engine record supplies the durable terminal evidence.
use sea_orm::{ConnectionTrait, DbErr, Statement, Value};
use uuid::Uuid;

pub mod replica;

fn statement(db: &impl ConnectionTrait, sql: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(db.get_database_backend(), sql, values)
}

/// Commit a stable game ID before telling Central. Retries return the original ID.
/// # Errors
/// Database failure.
pub async fn reserve(db: &impl ConnectionTrait, match_id: Uuid) -> Result<Uuid, DbErr> {
    db.query_one_raw(statement(
        db,
        "INSERT INTO ranked_run(match_id,game) VALUES ($1,uuidv7())
         ON CONFLICT(match_id) DO UPDATE SET seen_at=CASE WHEN ranked_run.launched
             THEN ranked_run.seen_at ELSE clock_timestamp() END
         WHERE ranked_run.generation IS NULL
         RETURNING game",
        vec![match_id.into()],
    ))
    .await?
    .ok_or(DbErr::RecordNotFound("ranked binding".into()))?
    .try_get("", "game")
}

/// Burn the right to start before exposing a shuffled opening to either player.
/// # Errors
/// Database failure.
pub async fn launch_once(
    db: &impl ConnectionTrait,
    match_id: Uuid,
    game: Uuid,
) -> Result<bool, DbErr> {
    Ok(db
        .execute_raw(statement(
            db,
            "UPDATE ranked_run SET launched=true,seen_at=clock_timestamp()
         WHERE match_id=$1 AND game=$2 AND NOT launched AND generation IS NULL",
            vec![match_id.into(), game.into()],
        ))
        .await?
        .rows_affected()
        == 1)
}

/// One reserved game. Only a contiguous, completed game record with an authenticated
/// engine ending supplies a rated result.
pub struct Run {
    /// Assignment at Central.
    pub match_id: Uuid,
    /// Bound local game.
    pub game: Uuid,
    /// Age of the last observation of a living or starting game.
    pub absent_seconds: i64,
    /// A complete rules-engine result. Control-plane aborts carry no turn.
    pub outcome: Option<central_api::ranked::Outcome>,
}

/// Read the oldest reconciliation batch, with terminal evidence from the same database.
/// # Errors
/// Database failure.
pub async fn pending(db: &impl ConnectionTrait) -> Result<Vec<Run>, DbErr> {
    let rows = db.query_all_raw(statement(
        db,
        "SELECT b.match_id,b.game,extract(epoch FROM clock_timestamp()-b.seen_at)::bigint AS absent,
            CASE WHEN b.launched AND r.complete AND r.turn>0 THEN r.winners END AS winners
         FROM ranked_run b LEFT JOIN game_record r ON r.game_id=b.game::text
         WHERE b.generation IS NULL ORDER BY b.serviced_at,b.match_id LIMIT 32",
        vec![],
    )).await?;
    rows.into_iter()
        .map(|row| {
            let winners: Option<Vec<i16>> = row.try_get("", "winners")?;
            Ok(Run {
                match_id: row.try_get("", "match_id")?,
                game: row.try_get("", "game")?,
                absent_seconds: row.try_get("", "absent")?,
                outcome: outcome(winners.as_deref()),
            })
        })
        .collect()
}

fn outcome(winners: Option<&[i16]>) -> Option<central_api::ranked::Outcome> {
    use central_api::ranked::Outcome;
    match winners {
        Some([0]) => Some(Outcome::FirstWins),
        Some([1]) => Some(Outcome::SecondWins),
        Some([]) => Some(Outcome::Draw),
        _ => None,
    }
}

/// Rotate the batch, remembering when this game last existed in the host process.
/// # Errors
/// Database failure.
pub async fn touch(db: &impl ConnectionTrait, match_id: Uuid, living: bool) -> Result<(), DbErr> {
    db.execute_raw(statement(
        db,
        "UPDATE ranked_run SET serviced_at=clock_timestamp(),
        seen_at=CASE WHEN $2 THEN clock_timestamp() ELSE seen_at END WHERE match_id=$1 AND generation IS NULL",
        vec![match_id.into(), living.into()],
    ))
    .await?;
    Ok(())
}

/// Forget a binding after Central accepted the terminal state.
/// # Errors
/// Database failure.
pub async fn delivered(db: &impl ConnectionTrait, match_id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(statement(
        db,
        "DELETE FROM ranked_run WHERE match_id=$1 AND generation IS NULL",
        vec![match_id.into()],
    ))
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::ranked::Outcome;
    #[test]
    fn only_two_seat_terminal_evidence_can_score() {
        assert_eq!(outcome(Some(&[0])), Some(Outcome::FirstWins));
        assert_eq!(outcome(Some(&[1])), Some(Outcome::SecondWins));
        assert_eq!(outcome(Some(&[])), Some(Outcome::Draw));
        for invalid in [None, Some(&[0, 1][..]), Some(&[2]), Some(&[-1])] {
            assert_eq!(outcome(invalid), None);
        }
    }
}
