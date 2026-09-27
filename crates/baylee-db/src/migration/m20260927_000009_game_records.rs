//! Game records (#315): every input a game's engine took, as its engine sent
//! them, kept for replaying the game behind a bug report.
//!
//! `game_record` is one row per game, `game_record_chunk` the pieces in the
//! order they came (`seq`), which stored one after another are one gzip
//! stream of the record. The gateway never reads inside them. A record is
//! `complete` once its last piece arrived; a record whose engine died stays
//! incomplete and is kept all the same, since that is the one a bug report
//! most wants.
//!
//! `game_record_seat` says which account sat where, and is the only link
//! between a record and a person: the record itself names nobody. The link
//! goes with the account (`ON DELETE SET NULL`), the record stays.
//!
//! No retention limit: the owner chose to keep every record.

use sea_orm_migration::prelude::*;

/// The ninth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GameRecord::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameRecord::GameId)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GameRecord::StartedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(GameRecord::EndedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GameRecord::Complete)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(GameRecord::Bytes)
                            .big_integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_table(
                Table::create()
                    .table(GameRecordChunk::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(GameRecordChunk::GameId).text().not_null())
                    .col(ColumnDef::new(GameRecordChunk::Seq).integer().not_null())
                    .col(ColumnDef::new(GameRecordChunk::Data).binary().not_null())
                    .primary_key(
                        Index::create()
                            .col(GameRecordChunk::GameId)
                            .col(GameRecordChunk::Seq),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("game_record_chunk_record")
                            .from(GameRecordChunk::Table, GameRecordChunk::GameId)
                            .to(GameRecord::Table, GameRecord::GameId)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .check(Expr::col(GameRecordChunk::Seq).gte(0))
                    .to_owned(),
            )
            .await?;
        seats(manager).await
    }

    /// Drops every record.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for table in [
            GameRecordSeat::Table.into_iden(),
            GameRecordChunk::Table.into_iden(),
            GameRecord::Table.into_iden(),
        ] {
            manager
                .drop_table(Table::drop().table(table).if_exists().to_owned())
                .await?;
        }
        Ok(())
    }
}

/// Which account sat where, the one link from a record to a person.
async fn seats(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(GameRecordSeat::Table)
                .if_not_exists()
                .col(ColumnDef::new(GameRecordSeat::GameId).text().not_null())
                .col(
                    ColumnDef::new(GameRecordSeat::Seat)
                        .small_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(GameRecordSeat::AccountId).uuid().null())
                .primary_key(
                    Index::create()
                        .col(GameRecordSeat::GameId)
                        .col(GameRecordSeat::Seat),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("game_record_seat_record")
                        .from(GameRecordSeat::Table, GameRecordSeat::GameId)
                        .to(GameRecord::Table, GameRecord::GameId)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("game_record_seat_account")
                        .from(GameRecordSeat::Table, GameRecordSeat::AccountId)
                        .to(Account::Table, Account::Id)
                        .on_delete(ForeignKeyAction::SetNull),
                )
                .to_owned(),
        )
        .await?;
    // An account's deletion sets its rows to NULL, and asks for them by
    // account to do it.
    manager
        .create_index(
            Index::create()
                .name("game_record_seat_by_account")
                .table(GameRecordSeat::Table)
                .col(GameRecordSeat::AccountId)
                .if_not_exists()
                .to_owned(),
        )
        .await
}

#[derive(DeriveIden)]
enum GameRecord {
    Table,
    GameId,
    StartedAt,
    EndedAt,
    Complete,
    Bytes,
}

#[derive(DeriveIden)]
enum GameRecordChunk {
    Table,
    GameId,
    Seq,
    Data,
}

#[derive(DeriveIden)]
enum GameRecordSeat {
    Table,
    GameId,
    Seat,
    AccountId,
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Id,
}
