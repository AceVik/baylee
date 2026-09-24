//! The standing answers go: nothing reads them and nothing writes them.
//!
//! `PUT /automation` stored them per account and the engine replayed them
//! into a seat as it attached. No client ever called the route: the client
//! keeps its per-ability answers in its own preferences (`ability_orders`,
//! stored through `/settings`) and hands them to the engine itself, as
//! `SetAbilityPolicy`. The route, the replay and the entity went with this
//! migration (#233).
//!
//! Rows are not carried anywhere. The development database held none when
//! this was written, and the only writer was that route.

use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer, boolean, uuid};

/// The fifth migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Named through `current_schema()` and not bare. A bare
        // `DROP … IF EXISTS` resolves along the whole search path: in a
        // schema that has no such table it walks on and drops the next one
        // it finds, which in a test sandbox is the developer's `public`.
        // The catalog learnt this by losing two live indexes.
        manager
            .get_connection()
            .execute_unprepared(
                "DO $drop$ BEGIN \
                   EXECUTE format('DROP TABLE IF EXISTS %I.standing_answer', current_schema()); \
                 END $drop$",
            )
            .await?;
        Ok(())
    }

    /// The table as the first migration made it, empty. The rows are not
    /// restored; the schema is, so the migrator can step back past here.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StandingAnswer::Table)
                    .if_not_exists()
                    .col(uuid(StandingAnswer::AccountId))
                    .col(big_integer(StandingAnswer::Card))
                    .col(big_integer(StandingAnswer::Ability))
                    .col(boolean(StandingAnswer::Yes))
                    .primary_key(
                        Index::create()
                            .col(StandingAnswer::AccountId)
                            .col(StandingAnswer::Card)
                            .col(StandingAnswer::Ability),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("standing_answer_account")
                            .from(StandingAnswer::Table, StandingAnswer::AccountId)
                            .to(Account::Table, Account::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum StandingAnswer {
    Table,
    AccountId,
    Card,
    Ability,
    Yes,
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Id,
}
