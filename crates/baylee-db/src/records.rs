//! Game records (#315): what a game's engine sent of its record, stored as it
//! came, and read back only for a player who sat at the game.
//!
//! Hand-written statements rather than entities: three tables the gateway
//! only appends to and reads whole, where the one thing worth having is that
//! a piece is added and counted in one statement.
//!
//! Nothing a seat or the lobby can ask reaches these tables. Three readers
//! a route reaches hand a record on: [`for_seated`], which the gateway calls
//! for a bug report from a player who sat at the game, and which answers
//! nothing for anyone else; [`for_delegate`], the same for a report a
//! host's seat bridge sends on its chair's seat token, answering only that
//! chair's game; and [`for_projection`], which only the replay
//! route calls, after its own check that the account sat there
//! ([`seat_of`]), and whose bytes go to an engine on the engine link, never
//! to a player (`docs/protocol.md` §"Replays"). The list of a player's
//! games ([`seated_games`]) reads the summary columns, never a record. The
//! operator's anonymised export for training and balancing
//! (`baylee-gateway records export`, no route) reads [`complete_ids`],
//! [`for_export`] and [`seated_identities`].

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, TransactionTrait, Value};
use uuid::Uuid;

/// Who sat in one seat of a recorded game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Seat {
    /// The account that played it; `None` for a chair the house or a
    /// host's seat bridge played.
    pub account: Option<Uuid>,
    /// The host whose seat bridge played it (a language model at its
    /// table), who answers for the seat without having played it.
    pub delegated_by: Option<Uuid>,
}

impl Seat {
    /// A seat an account played.
    #[must_use]
    pub const fn played_by(account: Uuid) -> Self {
        Self {
            account: Some(account),
            delegated_by: None,
        }
    }
}

/// Who a chair was, kept beside the account link so a seat whose link was
/// cut still reads as a former player rather than as the house.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Seated {
    /// Not said (a record from before replays).
    #[default]
    Unknown,
    /// A person's account.
    Person,
    /// The house AI.
    House,
    /// A host's seat bridge (a language model).
    Model,
}

impl Seated {
    const fn column(self) -> Option<&'static str> {
        match self {
            Self::Unknown => None,
            Self::Person => Some("person"),
            Self::House => Some("house"),
            Self::Model => Some("model"),
        }
    }

    fn of_column(text: Option<&str>) -> Self {
        match text {
            Some("person") => Self::Person,
            Some("house") => Self::House,
            Some("model") => Self::Model,
            _ => Self::Unknown,
        }
    }
}

/// What the replay list shows of one seat, beside [`Seat`]: card data, not
/// personal, kept when the account link is cut.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeatFacts {
    /// Who the chair was.
    pub seated: Seated,
    /// The deck's colour identity as `WUBRG` letters (`""`: colourless).
    pub colours: Option<String>,
    /// The deck's first commander, a `CardIndex`.
    pub commander: Option<u32>,
}

/// What the replay list shows of a game, known when it starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Opening {
    /// The engine's build, as `baylee_build::short` says it.
    pub build: Option<String>,
    /// Whether the engine writes `seen` into the record.
    pub checked: bool,
    /// Whether the table allowed spectators.
    pub allow_spectators: Option<bool>,
}

/// Starts a game's record: its row, and who sat in which seat (nobody, for
/// a chair the house plays). Starting one that exists changes nothing, so an
/// engine that attaches twice cannot rewrite who sat where.
///
/// # Errors
///
/// When a statement fails.
pub async fn open(db: &DatabaseConnection, game_id: &str, seats: &[Seat]) -> Result<(), DbErr> {
    let seats: Vec<(Seat, SeatFacts)> = seats
        .iter()
        .map(|seat| (*seat, SeatFacts::default()))
        .collect();
    open_with(db, game_id, &Opening::default(), &seats).await
}

/// [`open`], with what the replay list shows of the game and of each seat.
///
/// # Errors
///
/// When a statement fails.
pub async fn open_with(
    db: &DatabaseConnection,
    game_id: &str,
    opening: &Opening,
    seats: &[(Seat, SeatFacts)],
) -> Result<(), DbErr> {
    let txn = db.begin().await?;
    let backend = txn.get_database_backend();
    let made = txn
        .execute_raw(Statement::from_sql_and_values(
            backend,
            "INSERT INTO game_record (game_id, build, checked, allow_spectators) \
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            [
                game_id.into(),
                Value::String(opening.build.clone()),
                opening.checked.into(),
                Value::Bool(opening.allow_spectators),
            ],
        ))
        .await?;
    if made.rows_affected() == 1 {
        for (seat, (who, facts)) in seats.iter().enumerate() {
            let seat =
                i16::try_from(seat).map_err(|_| DbErr::Custom("seat out of range".into()))?;
            let commander = facts
                .commander
                .map(i32::try_from)
                .transpose()
                .map_err(|_| DbErr::Custom("commander out of range".into()))?;
            txn.execute_raw(Statement::from_sql_and_values(
                backend,
                "INSERT INTO game_record_seat \
                     (game_id, seat, account_id, delegated_by, seated, colours, commander) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
                [
                    game_id.into(),
                    seat.into(),
                    Value::Uuid(who.account),
                    Value::Uuid(who.delegated_by),
                    Value::String(facts.seated.column().map(str::to_owned)),
                    Value::String(facts.colours.clone()),
                    Value::Int(commander),
                ],
            ))
            .await?;
        }
    }
    txn.commit().await
}

/// Notes how a game ended, from its engine's `GameEnded`: the winning seats
/// (empty for a draw), the turn, and the reason. A game with no record yet
/// (nothing was stored before it ended) is left alone.
///
/// # Errors
///
/// When the statement fails.
pub async fn finish(
    db: &impl ConnectionTrait,
    game_id: &str,
    winners: &[u8],
    turn: Option<u32>,
    reason: &str,
) -> Result<(), DbErr> {
    let winners: Vec<i16> = winners.iter().map(|&w| i16::from(w)).collect();
    let turn = turn
        .map(i32::try_from)
        .transpose()
        .map_err(|_| DbErr::Custom("turn out of range".into()))?;
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE game_record SET winners = $2, turn = $3, end_reason = $4 WHERE game_id = $1",
        [
            game_id.into(),
            winners.into(),
            Value::Int(turn),
            reason.into(),
        ],
    ))
    .await?;
    Ok(())
}

/// Adds piece `seq` of a game's record; the last piece marks it complete.
///
/// Returns whether it was added: a piece already stored, or one for a game
/// whose record was never started, is not.
///
/// # Errors
///
/// When the statement fails.
pub async fn append(
    db: &impl ConnectionTrait,
    game_id: &str,
    seq: u32,
    data: Vec<u8>,
    last: bool,
) -> Result<bool, DbErr> {
    let seq = i32::try_from(seq).map_err(|_| DbErr::Custom("piece out of range".into()))?;
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "WITH piece AS ( \
                 INSERT INTO game_record_chunk (game_id, seq, data) \
                 SELECT game_id, $2, $3 FROM game_record WHERE game_id = $1 \
                 ON CONFLICT DO NOTHING \
                 RETURNING octet_length(data) AS n) \
             UPDATE game_record r SET \
                 bytes = r.bytes + piece.n, \
                 complete = r.complete OR $4, \
                 ended_at = CASE WHEN $4 THEN now() ELSE r.ended_at END \
             FROM piece WHERE r.game_id = $1",
            [game_id.into(), seq.into(), data.into(), last.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// A game's record as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The pieces in order, one gzip stream.
    pub data: Vec<u8>,
    /// Whether the last piece arrived.
    pub complete: bool,
}

/// The record of `game_id`, if `account` sat at that game; `None` for
/// anyone else, and for a game with no record.
///
/// # Errors
///
/// When the statement fails.
pub async fn for_seated(
    db: &impl ConnectionTrait,
    game_id: &str,
    account: Uuid,
) -> Result<Option<Record>, DbErr> {
    seated_record(
        db,
        "s.account_id = $2",
        vec![game_id.into(), account.into()],
    )
    .await
}

/// The record of `game_id`, if its chair `seat` is one `host`'s seat bridge
/// played (`delegated_by`, no account): what a report sent on that chair's
/// seat token carries. `None` for another game, another chair or another
/// host, a chair an account played, and a game with no record.
///
/// # Errors
///
/// When the statement fails.
pub async fn for_delegate(
    db: &impl ConnectionTrait,
    game_id: &str,
    seat: usize,
    host: Uuid,
) -> Result<Option<Record>, DbErr> {
    let Ok(seat) = i16::try_from(seat) else {
        return Ok(None);
    };
    seated_record(
        db,
        "s.seat = $2 AND s.delegated_by = $3 AND s.account_id IS NULL",
        vec![game_id.into(), seat.into(), host.into()],
    )
    .await
}

/// The record of the game `$1` names, when a seat of it matches `seated`: a
/// condition on `game_record_seat s`, its values from `$2` on. Never a
/// value spliced into the text.
async fn seated_record(
    db: &impl ConnectionTrait,
    seated: &'static str,
    values: Vec<Value>,
) -> Result<Option<Record>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            format!(
                "SELECT r.complete, \
                     coalesce((SELECT string_agg(c.data, ''::bytea ORDER BY c.seq) \
                               FROM game_record_chunk c WHERE c.game_id = r.game_id), ''::bytea) \
                       AS data \
                 FROM game_record r \
                 WHERE r.game_id = $1 AND EXISTS ( \
                     SELECT 1 FROM game_record_seat s \
                     WHERE s.game_id = r.game_id AND {seated})"
            ),
            values,
        ))
        .await?;
    row.map(|row| {
        Ok(Record {
            data: row.try_get("", "data")?,
            complete: row.try_get("", "complete")?,
        })
    })
    .transpose()
}

/// The games whose record is complete, oldest first, for the anonymised
/// export (`baylee-gateway records export`). Their ids never leave the
/// export: an id is the join to who sat there, and a time besides.
///
/// # Errors
///
/// When the statement fails.
pub async fn complete_ids(db: &impl ConnectionTrait) -> Result<Vec<String>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_string(
            db.get_database_backend(),
            "SELECT game_id FROM game_record WHERE complete ORDER BY started_at, game_id",
        ))
        .await?;
    rows.iter().map(|row| row.try_get("", "game_id")).collect()
}

/// The record of `game_id` whoever sat there, for the anonymised export
/// only; `None` for a game with no record. A seat's or a report's question
/// goes through [`for_seated`].
///
/// # Errors
///
/// When the statement fails.
pub async fn for_export(db: &impl ConnectionTrait, game_id: &str) -> Result<Option<Record>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT r.complete, \
                 coalesce((SELECT string_agg(c.data, ''::bytea ORDER BY c.seq) \
                           FROM game_record_chunk c WHERE c.game_id = r.game_id), ''::bytea) \
                   AS data \
             FROM game_record r WHERE r.game_id = $1",
            [game_id.into()],
        ))
        .await?;
    row.map(|row| {
        Ok(Record {
            data: row.try_get("", "data")?,
            complete: row.try_get("", "complete")?,
        })
    })
    .transpose()
}

/// Everything that names an account linked to a seat of `game_id` (one that
/// played it, or whose seat bridge did): its id, email, username, display
/// name and invite key's id, as text. The export refuses a record in which
/// any of them is found.
///
/// # Errors
///
/// When the statement fails.
pub async fn seated_identities(
    db: &impl ConnectionTrait,
    game_id: &str,
) -> Result<Vec<String>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT DISTINCT unnest(ARRAY[a.id::text, a.email, a.username, a.display_name, \
                                          a.invite_id::text]) AS said \
             FROM game_record_seat s \
             JOIN account a ON a.id = s.account_id OR a.id = s.delegated_by \
             WHERE s.game_id = $1",
            [game_id.into()],
        ))
        .await?;
    let mut said = Vec::with_capacity(rows.len());
    for row in &rows {
        if let Some(text) = row.try_get::<Option<String>>("", "said")? {
            said.push(text);
        }
    }
    Ok(said)
}

/// One seat of a game in [`seated_games`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListedSeat {
    /// The seat.
    pub seat: u8,
    /// The account that played it, while linked.
    pub account: Option<Uuid>,
    /// The host whose seat bridge played it, while linked.
    pub delegated_by: Option<Uuid>,
    /// Who the chair was ([`Seated::Unknown`] in a record from before it).
    pub seated: Seated,
    /// The account's display name and tag now, while linked.
    pub name: Option<(String, i32)>,
    /// The deck's colour identity, `WUBRG` letters.
    pub colours: Option<String>,
    /// The deck's first commander, a `CardIndex`.
    pub commander: Option<u32>,
    /// Whether the seat's player shares its hidden cards in this game's
    /// replays: what it said ([`set_share`]), else the default, which is yes
    /// for a game recorded from the `replay_share_cutoff` on and no for one
    /// recorded before (owner, 13.10.2026). An AI seat is shared whatever
    /// this says; the caller knows which seats are AI.
    pub shared: bool,
}

/// One game an account sat at, as the replay list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeatedGame {
    /// The game.
    pub game_id: String,
    /// When its record started, RFC 3339 in UTC.
    pub started_at: String,
    /// When its last piece arrived, RFC 3339 in UTC.
    pub ended_at: Option<String>,
    /// Whether the last piece arrived.
    pub complete: bool,
    /// The engine's build.
    pub build: Option<String>,
    /// Whether the record carries `seen`.
    pub checked: bool,
    /// Whether the table allowed spectators (`None`: not said).
    pub allow_spectators: Option<bool>,
    /// The winning seats (`None`: no end noted).
    pub winners: Option<Vec<u8>>,
    /// The turn it ended on.
    pub turn: Option<u32>,
    /// Why it ended.
    pub end_reason: Option<String>,
    /// The record's size in the store, compressed.
    pub bytes: u64,
    /// Every seat, in seat order.
    pub seats: Vec<ListedSeat>,
}

/// The games `account` sat at (played a seat; a seat its bridge played
/// does not count), newest first (`started_at desc, game_id`), `limit` of
/// them after `offset`, and how many there are in all. Reads the summary
/// columns and the seats' current handles, never a record.
///
/// # Errors
///
/// When a statement fails.
pub async fn seated_games(
    db: &impl ConnectionTrait,
    account: Uuid,
    archived: bool,
    offset: u64,
    limit: u64,
) -> Result<(Vec<SeatedGame>, u64), DbErr> {
    let backend = db.get_database_backend();
    let offset = i64::try_from(offset).unwrap_or(i64::MAX);
    let limit = i64::try_from(limit).unwrap_or(i64::MAX);
    let total: i64 = db
        .query_one_raw(Statement::from_sql_and_values(
            backend,
            "SELECT count(DISTINCT s.game_id) AS n FROM game_record_seat s \
             WHERE s.account_id = $1 AND $2 = EXISTS ( \
                 SELECT 1 FROM replay_archived h WHERE h.account_id = $1 AND h.game_id = s.game_id)",
            [account.into(), archived.into()],
        ))
        .await?
        .map_or(Ok(0), |row| row.try_get("", "n"))?;
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            backend,
            "SELECT r.game_id, \
                 to_char(r.started_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
                   AS started_at, \
                 to_char(r.ended_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') \
                   AS ended_at, \
                 r.complete, r.build, r.checked, r.allow_spectators, r.winners, r.turn, \
                 r.end_reason, r.bytes \
             FROM game_record r \
             WHERE EXISTS (SELECT 1 FROM game_record_seat s \
                           WHERE s.game_id = r.game_id AND s.account_id = $1) \
               AND $4 = EXISTS (SELECT 1 FROM replay_archived h \
                                WHERE h.account_id = $1 AND h.game_id = r.game_id) \
             ORDER BY r.started_at DESC, r.game_id \
             OFFSET $2 LIMIT $3",
            [account.into(), offset.into(), limit.into(), archived.into()],
        ))
        .await?;
    let mut games = Vec::with_capacity(rows.len());
    for row in &rows {
        let winners: Option<Vec<i16>> = row.try_get("", "winners")?;
        let turn: Option<i32> = row.try_get("", "turn")?;
        let bytes: i64 = row.try_get("", "bytes")?;
        games.push(SeatedGame {
            game_id: row.try_get("", "game_id")?,
            started_at: row.try_get("", "started_at")?,
            ended_at: row.try_get("", "ended_at")?,
            complete: row.try_get("", "complete")?,
            build: row.try_get("", "build")?,
            checked: row.try_get("", "checked")?,
            allow_spectators: row.try_get("", "allow_spectators")?,
            winners: winners.map(|w| w.into_iter().filter_map(|s| u8::try_from(s).ok()).collect()),
            turn: turn.and_then(|t| u32::try_from(t).ok()),
            end_reason: row.try_get("", "end_reason")?,
            bytes: u64::try_from(bytes).unwrap_or(0),
            seats: Vec::new(),
        });
    }
    if games.is_empty() {
        return Ok((games, u64::try_from(total).unwrap_or(0)));
    }
    let ids: Vec<String> = games.iter().map(|g| g.game_id.clone()).collect();
    for (game_id, seat) in listed_seats(db, ids).await? {
        if let Some(game) = games.iter_mut().find(|g| g.game_id == game_id) {
            game.seats.push(seat);
        }
    }
    Ok((games, u64::try_from(total).unwrap_or(0)))
}

/// Every seat of `game_id`, in seat order, as [`seated_games`] lists them:
/// the names a replay's payload shows.
///
/// # Errors
///
/// When the statement fails.
pub async fn seats_of(db: &impl ConnectionTrait, game_id: &str) -> Result<Vec<ListedSeat>, DbErr> {
    Ok(listed_seats(db, vec![game_id.to_owned()])
        .await?
        .into_iter()
        .map(|(_, seat)| seat)
        .collect())
}

/// The seats of `games`, each with its game, in game and seat order.
async fn listed_seats(
    db: &impl ConnectionTrait,
    games: Vec<String>,
) -> Result<Vec<(String, ListedSeat)>, DbErr> {
    let rows = db
        .query_all_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT s.game_id, s.seat, s.account_id, s.delegated_by, s.seated, s.colours, \
                    s.commander, a.display_name, a.tag, \
                    coalesce(g.shared, r.started_at >= \
                        coalesce((SELECT c.at FROM replay_share_cutoff c), '-infinity')) AS shared \
             FROM game_record_seat s LEFT JOIN account a ON a.id = s.account_id \
             LEFT JOIN game_record_share g ON g.game_id = s.game_id AND g.seat = s.seat \
             JOIN game_record r ON r.game_id = s.game_id \
             WHERE s.game_id = ANY($1) ORDER BY s.game_id, s.seat",
            [games.into()],
        ))
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let seat: i16 = row.try_get("", "seat")?;
        let seated: Option<String> = row.try_get("", "seated")?;
        let display_name: Option<String> = row.try_get("", "display_name")?;
        let tag: Option<i32> = row.try_get("", "tag")?;
        let commander: Option<i32> = row.try_get("", "commander")?;
        out.push((
            row.try_get("", "game_id")?,
            ListedSeat {
                seat: u8::try_from(seat).unwrap_or(u8::MAX),
                account: row.try_get("", "account_id")?,
                delegated_by: row.try_get("", "delegated_by")?,
                seated: Seated::of_column(seated.as_deref()),
                name: display_name.zip(tag),
                colours: row.try_get("", "colours")?,
                commander: commander.and_then(|c| u32::try_from(c).ok()),
                shared: row.try_get("", "shared")?,
            },
        ));
    }
    Ok(out)
}

/// Sets whether `seat` of `game_id` shares its hidden cards in that game's
/// replays. The caller has found the seat as the asking account's own
/// ([`seat_of`]). Returns whether the game has a record.
///
/// # Errors
///
/// When the statement fails.
pub async fn set_share(
    db: &impl ConnectionTrait,
    game_id: &str,
    seat: u8,
    shared: bool,
) -> Result<bool, DbErr> {
    let done = db
        .execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO game_record_share (game_id, seat, shared) \
             SELECT game_id, $2, $3 FROM game_record WHERE game_id = $1 \
             ON CONFLICT (game_id, seat) DO UPDATE SET shared = EXCLUDED.shared, at = now()",
            [game_id.into(), i16::from(seat).into(), shared.into()],
        ))
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Archives `game_id` for `account` (moves it to its archived list,
/// [`seated_games`] with `archived`), or takes it back out; the record and
/// the other accounts' lists are untouched. Saying the same again changes
/// nothing.
///
/// # Errors
///
/// When the statement fails.
pub async fn set_archived(
    db: &impl ConnectionTrait,
    account: Uuid,
    game_id: &str,
    archived: bool,
) -> Result<(), DbErr> {
    let sql = if archived {
        "INSERT INTO replay_archived (account_id, game_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
    } else {
        "DELETE FROM replay_archived WHERE account_id = $1 AND game_id = $2"
    };
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        [account.into(), game_id.into()],
    ))
    .await?;
    Ok(())
}

/// The seat `account` played at `game_id`, if it played one (the lowest,
/// should it hold two); `None` for anyone else and for a game with no
/// record. A seat its bridge played is not its seat.
///
/// # Errors
///
/// When the statement fails.
pub async fn seat_of(
    db: &impl ConnectionTrait,
    game_id: &str,
    account: Uuid,
) -> Result<Option<u8>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT min(seat) AS seat FROM game_record_seat \
             WHERE game_id = $1 AND account_id = $2",
            [game_id.into(), account.into()],
        ))
        .await?;
    let seat: Option<i16> = match row {
        Some(row) => row.try_get("", "seat")?,
        None => None,
    };
    Ok(seat.and_then(|s| u8::try_from(s).ok()))
}

/// A game's record as a projection job reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projectable {
    /// The pieces in order, one gzip stream.
    pub data: Vec<u8>,
    /// Whether the last piece arrived.
    pub complete: bool,
    /// The build that played it, where the engine said.
    pub build: Option<String>,
}

/// The size of `game_id`'s record in the store, compressed, and whether it
/// is complete; `None` for a game with no record. Read before
/// [`for_projection`], so an oversized record is refused without being
/// loaded.
///
/// # Errors
///
/// When the statement fails.
pub async fn stored_size(
    db: &impl ConnectionTrait,
    game_id: &str,
) -> Result<Option<(u64, bool)>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT bytes, complete FROM game_record WHERE game_id = $1",
            [game_id.into()],
        ))
        .await?;
    row.map(|row| {
        let bytes: i64 = row.try_get("", "bytes")?;
        Ok((
            u64::try_from(bytes).unwrap_or(0),
            row.try_get("", "complete")?,
        ))
    })
    .transpose()
}

/// The record of `game_id` whoever sat there, for a replay's projection job
/// only: the caller has checked that the account asking sat at the game
/// ([`seat_of`]), and hands the bytes to an engine on the engine link, never
/// to a player. `None` for a game with no record. Its one caller is the
/// gateway's replay route (a test holds it so).
///
/// # Errors
///
/// When the statement fails.
pub async fn for_projection(
    db: &impl ConnectionTrait,
    game_id: &str,
) -> Result<Option<Projectable>, DbErr> {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT r.complete, r.build, \
                 coalesce((SELECT string_agg(c.data, ''::bytea ORDER BY c.seq) \
                           FROM game_record_chunk c WHERE c.game_id = r.game_id), ''::bytea) \
                   AS data \
             FROM game_record r WHERE r.game_id = $1",
            [game_id.into()],
        ))
        .await?;
    row.map(|row| {
        Ok(Projectable {
            data: row.try_get("", "data")?,
            complete: row.try_get("", "complete")?,
            build: row.try_get("", "build")?,
        })
    })
    .transpose()
}
