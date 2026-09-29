//! Maik's European Highlander joins the house decks.
//!
//! Like the four migration 3 brought, it is a deck that exists on a table and
//! not one written for the engine's needs: a hundred singleton cards, no
//! commander, with the printing each card actually is. It came across as a
//! Moxfield export (`data/decks/raw/maik.txt`) and was rewritten in the house
//! spelling: a two-faced card by its front face, set, collector number and
//! foil kept. `cargo run -p xtask -- deck-check` is what says a row still
//! parses and still names a card this pool has.
//!
//! The deck is named for its format, not for the person who plays it: a house
//! deck is shown to every player, and the first name in the description is
//! all this table needs to say about whose it is.
//!
//! It is `house`, so it belongs to nobody and is what a copy starts from, and
//! it is seeded under the same guard as the others: a database that somehow
//! already holds a house deck of that name keeps it.

use sea_orm_migration::prelude::*;

use super::decklist::{self, Decklist};

/// The deck, as `data/decks/` holds it: `(file, format, description)`.
///
/// The name is the file's own `[deck:…]` header, so the deck cannot be called
/// one thing here and another there.
const DECKS: [(&str, &str, &str); 1] = [(
    include_str!("../../../../data/decks/maik.txt"),
    "highlander",
    "Maiks European Highlander: hundert Karten, jede nur einmal, kein \
     Kommandeur. Eine Kreaturen-Werkzeugkiste in fast allen Farben — Birthing \
     Pod, Natural Order und Eldritch Evolution holen genau die Kreatur, die \
     gerade gebraucht wird, Ephemerate und Restoration Angel lassen ihre \
     Betritt-Fähigkeiten zweimal auslösen, und Force of Will und Mana Drain \
     schützen den Plan. Prüfstein für Kopien (Phantasmal Image, Kiki-Jiki), \
     für Evoke und für Suchen, die an Manawerte gebunden sind.",
)];

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (text, format, description) in DECKS {
            decklist::seed(manager, &Decklist::parse(text), format, description).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (text, _, _) in DECKS {
            decklist::remove(manager, &Decklist::parse(text).name).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_deck_file_is_a_hundred_cards_under_one_name() {
        decklist::assert_seedable(&DECKS);
    }
}
