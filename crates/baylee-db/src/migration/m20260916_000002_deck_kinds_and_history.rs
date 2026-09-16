//! What a deck *is*, and what it *was*.
//!
//! The first migration stored a deck as one row: a name, two arrays of card
//! lines and a commander. That is a deck somebody owns and is playing right
//! now, and it answers none of the three questions this one adds.
//!
//! # Three kinds of deck, and only one of them belongs to anybody
//!
//! An **account** deck is a player's own. A **preconstructed** deck is a
//! retail product — bought, fixed, and the same for everyone. A **house**
//! deck is one this project publishes to be played with: the shipped decks
//! the table is exercised against. The last two belong to nobody, so
//! `account_id` becomes nullable and a `CHECK` ties the two columns
//! together, because "kind says house and there is an owner" is the state
//! that would quietly hand somebody else's deck to a player.
//!
//! It is `kind` and not `is_house`, because a copy has to know what it came
//! from: `copied_from`/`copied_version` say which deck and which of its
//! states were copied, and a copy is an ordinary account deck from the
//! moment it exists.
//!
//! # `deck` is the present and `deck_version` is the past
//!
//! The owner asked that every change to a deck's cards be traceable and
//! reversible. The obvious shape — move the cards into a version table and
//! leave `deck` pointing at the current one — stores the head *twice* the
//! moment anything caches it, and makes every reader of a deck do a join to
//! learn what is in it.
//!
//! So the cards stay where they are. `deck.cards` is what the deck holds
//! **now**, and `deck_version` holds the states it no longer has, each
//! stamped with the version number it was and the moment it stopped being
//! current. There is nothing to keep in step: the two tables never describe
//! the same state, so they cannot disagree about one.
//!
//! `deck.version` is the number the current cards carry, so a deck saved
//! four times is at version 5 with rows 1–4 behind it. Reverting to version
//! *k* is an ordinary save of version *k*'s lists — which archives the
//! present as usual — so a revert is a change like any other and is itself
//! revertible. History only ever grows, and no row is ever rewritten.
//!
//! An `int` and not a timestamp, and not the row's `uuid` either: `uuidv7`
//! is ordered by the clock of whoever minted it, and two gateways have two
//! clocks. A counter the database increments in the same statement has one.
//!
//! # A deck may have two commanders
//!
//! `commander` was one nullable column, which is the shape of a rule Magic
//! does not have: the partner mechanic (CR 702.124) seats two, and the
//! engine has taken a list since commanders existed —
//! `SeatSpec::commanders` is a `Vec` and `loaded_deck` was building a
//! one-element one out of the column. So the column becomes the list the
//! rest of the stack already speaks, and a deck with no commander is an
//! empty array rather than a null: "how many commanders" then has one
//! answer and not two.
//!
//! # The four house decks
//!
//! Seeded here, which is the only place that gives every install the same
//! decks without a data file beside the code. They are stored as card
//! *names*, exactly as `entity::deck` says a deck is, so a deck naming a
//! card this build has not implemented yet is a deck that fails when someone
//! takes it to a table rather than a deck that was quietly shortened.

use super::house_decks::{BREYA, KENRITH, KESS, TAYAM};
use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{
    array, integer, text, text_null, timestamp_with_time_zone, uuid, uuid_null,
};

/// Deck kinds, history, and the four house decks.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        widen_deck(manager).await?;
        versions(manager).await?;
        seed_house_decks(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // The house decks go with the columns that describe them: without
        // `kind` there is no such thing as a deck nobody owns, and a row with
        // a null `account_id` would fail the column this rollback restores.
        exec(
            manager,
            "DELETE FROM deck WHERE kind <> 'account' OR account_id IS NULL",
        )
        .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(DeckVersion::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        exec(
            manager,
            "ALTER TABLE deck DROP CONSTRAINT IF EXISTS deck_owner_matches_kind",
        )
        .await?;
        for column in [
            Deck::Kind.into_iden(),
            Deck::Format.into_iden(),
            Deck::Description.into_iden(),
            Deck::CopiedFrom.into_iden(),
            Deck::CopiedVersion.into_iden(),
            Deck::Version.into_iden(),
        ] {
            manager
                .alter_table(
                    Table::alter()
                        .table(Deck::Table)
                        .drop_column(column)
                        .to_owned(),
                )
                .await?;
        }
        exec(manager, "ALTER TABLE deck ADD COLUMN commander text").await?;
        exec(
            manager,
            "UPDATE deck SET commander = commanders[1] WHERE cardinality(commanders) > 0",
        )
        .await?;
        exec(manager, "ALTER TABLE deck DROP COLUMN commanders").await?;
        exec(
            manager,
            "ALTER TABLE deck ALTER COLUMN account_id SET NOT NULL",
        )
        .await?;
        Ok(())
    }
}

/// One statement, on the connection the migrator is already holding.
///
/// Unqualified on purpose: the gateway's end-to-end tests each run this
/// migrator in a schema of their own, and a statement naming `public.deck`
/// would reach out of that sandbox and alter the developer's table.
async fn exec(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute_unprepared(sql)
        .await
        .map(|_| ())
}

/// What a deck is: whose it is, what it plays, and where it came from.
async fn widen_deck(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .alter_table(
            Table::alter()
                .table(Deck::Table)
                // Every deck that exists today is somebody's own, which is
                // what the default says — and it is the right default for
                // the column afterwards too, because that is the kind a
                // player's `POST /decks` makes.
                .add_column(text(Deck::Kind).default("account"))
                .add_column(text(Deck::Format).default("freeform"))
                .add_column(text_null(Deck::Description))
                .add_column(uuid_null(Deck::CopiedFrom))
                .add_column(integer(Deck::CopiedVersion).null())
                .add_column(integer(Deck::Version).default(1))
                .to_owned(),
        )
        .await?;

    // A deck that named a commander is playing Commander — the same sentence
    // `baylee_cards::decks::format_of` already reads off a loaded deck, so
    // backfilling it here tells the stored deck what the code had been
    // deciding for it every time it was seated.
    exec(
        manager,
        "UPDATE deck SET format = 'commander' WHERE commander IS NOT NULL",
    )
    .await?;

    exec(
        manager,
        "ALTER TABLE deck ALTER COLUMN account_id DROP NOT NULL",
    )
    .await?;

    // One column becomes a list, carrying what it held. The order matters —
    // the format above is read off `commander` while it still exists.
    exec(
        manager,
        "ALTER TABLE deck ADD COLUMN commanders text[] NOT NULL DEFAULT '{}'",
    )
    .await?;
    exec(
        manager,
        "UPDATE deck SET commanders = ARRAY[commander] WHERE commander IS NOT NULL",
    )
    .await?;
    exec(manager, "ALTER TABLE deck DROP COLUMN commander").await?;

    // Both directions, deliberately. A house deck with an owner hands one
    // player a deck everybody is supposed to share; an account deck without
    // one is a deck no `DELETE FROM account` will ever reach.
    exec(
        manager,
        "ALTER TABLE deck ADD CONSTRAINT deck_owner_matches_kind \
         CHECK ((kind = 'account') = (account_id IS NOT NULL))",
    )
    .await?;

    exec(
        manager,
        "ALTER TABLE deck ADD CONSTRAINT deck_kind_is_known \
         CHECK (kind IN ('account', 'preconstructed', 'house'))",
    )
    .await?;

    // A copy points at the deck it came from, and that deck may be deleted
    // while the copy plays on. `SET NULL` is what "it came from a deck that
    // is gone" looks like; a cascade here would delete the player's own deck
    // because somebody retired the product it was built from.
    manager
        .create_foreign_key(
            ForeignKey::create()
                .name("deck_copied_from")
                .from(Deck::Table, Deck::CopiedFrom)
                .to(Deck::Table, Deck::Id)
                .on_delete(ForeignKeyAction::SetNull)
                .to_owned(),
        )
        .await?;

    // "The decks anyone may play" is its own question and its own index —
    // `deck_account_recent` is on `account_id`, which is null for every row
    // this one is about.
    manager
        .create_index(
            Index::create()
                .name("deck_shared_recent")
                .table(Deck::Table)
                .col(Deck::Kind)
                .col((Deck::UpdatedAt, IndexOrder::Desc))
                .to_owned(),
        )
        .await?;

    Ok(())
}

/// The states a deck no longer has.
async fn versions(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(DeckVersion::Table)
                .if_not_exists()
                .col(uuid(DeckVersion::DeckId))
                .col(integer(DeckVersion::Version))
                .col(array(DeckVersion::Cards, ColumnType::Text))
                .col(array(DeckVersion::Sideboard, ColumnType::Text))
                .col(array(DeckVersion::Commanders, ColumnType::Text))
                .col(text_null(DeckVersion::Summary))
                .col(timestamp_with_time_zone(DeckVersion::SupersededAt))
                // The pair *is* the identity: there is exactly one state a
                // deck had at version 4, and a second row claiming to be it
                // is a forked history rather than a duplicate. It is also
                // the index "this deck's history, newest first" reads.
                .primary_key(
                    Index::create()
                        .col(DeckVersion::DeckId)
                        .col(DeckVersion::Version),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("deck_version_deck")
                        .from(DeckVersion::Table, DeckVersion::DeckId)
                        .to(Deck::Table, Deck::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await?;
    Ok(())
}

/// The four decks this project ships, as the house's own.
///
/// Inserted by name and with `format = 'commander'`, each at version 1 with
/// no history behind it — which is true: nobody has changed them yet.
async fn seed_house_decks(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    for (name, commander, description, cards) in [
        (
            "Kenrith, the Returned King",
            "Kenrith, the Returned King",
            "Fünf Farben, alles erlaubt: Ramp, Tutoren, Reanimation und vier \
             Aktivierungen auf dem Kommandanten. Der breiteste der vier — \
             gedacht als Prüfstein dafür, dass Farbidentität und Mana über \
             alle fünf Farben halten.",
            KENRITH,
        ),
        (
            "Breya, Etherium Shaper",
            "Breya, Etherium Shaper",
            "Artefakt-Kombo in vier Farben: Sacrifice-Schleifen, \
             Artefakt-Recursion und die Kombos, die daraus entstehen. \
             Prüfstein für Ersatzeffekte, Opferkosten und Schleifen, die der \
             Engine auffallen müssen, bevor sie endlos werden.",
            BREYA,
        ),
        (
            "Kess, Dissident Mage",
            "Kess, Dissident Mage",
            "Grixis-Spellslinger: Zaubern aus dem Friedhof, Countermagie, \
             Storm und Extra-Züge. Prüfstein für den Stack — wer worauf \
             antwortet, was verrechnet wird und was beim Verrechnen passiert.",
            KESS,
        ),
        (
            "Tayam, Luminous Enigma",
            "Tayam, Luminous Enigma",
            "Abzan-Friedhof: Marken, Opfern und Zurückholen. Prüfstein für \
             +1/+1- und -1/-1-Marken, Todes-Trigger und Rekursion. Persist \
             und Undying sind im Pool noch Stubs — die Karten liegen im Deck, \
             die Regel fehlt der Engine noch.",
            TAYAM,
        ),
    ] {
        let lines = cards.join("\n");
        let statement = Statement::from_sql_and_values(
            manager.get_database_backend(),
            "INSERT INTO deck \
             (account_id, kind, name, format, description, cards, sideboard, \
              commanders, version, updated_at) \
             VALUES (NULL, 'house', $1, 'commander', $2, \
                     string_to_array($3, chr(10)), '{}', ARRAY[$4], 1, now())",
            [
                name.into(),
                description.into(),
                lines.into(),
                commander.into(),
            ],
        );
        ConnectionTrait::execute_raw(manager.get_connection(), statement).await?;
    }
    Ok(())
}

#[derive(DeriveIden)]
enum Deck {
    Table,
    Id,
    Kind,
    Format,
    Description,
    CopiedFrom,
    CopiedVersion,
    Version,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum DeckVersion {
    Table,
    DeckId,
    Version,
    Cards,
    Sideboard,
    Commanders,
    Summary,
    SupersededAt,
}
