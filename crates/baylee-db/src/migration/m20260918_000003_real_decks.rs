//! Four decks that exist on a table, and the two rows that never resolved.
//!
//! # The decks
//!
//! The four before these were written to exercise the engine. These four are
//! the opposite: decks people actually own, brought across with the printing
//! each card actually is, because a decklist is easy to retype and a hundred
//! deliberate printing choices are not.
//!
//! **Allytifact** and **Victory** are the owner's, read out of a Forge
//! Adventure save where every card carries its set and collector number.
//! **Schwarzrand** is Ossi's Sultai Highlander and **Weltenbaum** is
//! Dominik's five-colour lands Highlander — a hundred singleton cards each
//! and no commander, which is settled rather than assumed: `Ignoble Hierarch`
//! is `{B}{G}{R}` by identity and no Sultai commander could lead a deck
//! holding it.
//!
//! They are `house` and not `preconstructed`, so they belong to nobody and
//! are the thing a copy starts from. `scripts/forge/` is how they were read
//! and `cargo run -p xtask -- deck-check` is what says a row still parses and
//! still names a card this pool has.
//!
//! A commander is stored as a **bare card name** and its own row sits among
//! the deck's, which is not decoration: `decks::by_name` is an exact-spelling
//! lookup and `from_lines` drops a leader it cannot resolve, so a leader
//! written the way a deck row is written seats nobody and never says so. The
//! row beside it is where the printing lives, and `from_lines` moves it out of
//! the library instead of copying it, so the deck is still a hundred cards.
//!
//! The lists live in `data/decks/*.txt` and are pulled in with `include_str!`
//! rather than retyped as constants beside this file. A decklist is text that
//! a person edits and a tool checks; copying it into Rust would make two
//! truths out of one, and the compiler is what keeps this one honest.
//!
//! # The repair
//!
//! `Fire // Ice` and `Fatehold Chronologist // Peer Review` were seeded into
//! the Kess and Kenrith decks with both faces in the name. This pool names a
//! two-faced card by its **front face** — there is not one `//` among the
//! 1615 names it compiles — so neither row ever resolved, and each of those
//! decks has been quietly playing 98 cards instead of 99 since the day it was
//! seeded. Nothing said so: an unresolvable name is dropped, not refused.
//!
//! The seed those rows came from is deliberately **not** edited. A migration
//! that has run somewhere is never changed, and that holds for the data it
//! reads: a fresh database still gets the old rows and is repaired here, so
//! every database converges on the same state through the same steps rather
//! than on which day it happened to be created.

use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

use super::decklist::{self, Decklist, FRONT_FACE_REPAIRS};

/// The four decks, as `data/decks/` holds them.
///
/// `(file, format, description)`. The name is the file's own `[deck:…]`
/// header, so a deck cannot be called one thing here and another there.
const DECKS: [(&str, &str, &str); 4] = [
    (
        include_str!("../../../../data/decks/allytifact.txt"),
        "commander",
        "Verbündete und Artefakte unter General Tazri: viele kleine Kreaturen, \
         die einander verstärken, und ein Manasystem aus Artefakten. Prüfstein \
         für Betritt-das-Spielfeld-Trigger in Serie und für Fähigkeiten, die \
         eine ganze Kreaturenart zählen.",
    ),
    (
        include_str!("../../../../data/decks/victory.txt"),
        "commander",
        "Esper-Blink unter Aminatou: Kreaturen ins Exil und zurück, \
         Countermagie und Tutoren. Prüfstein für Zonenwechsel als \
         Ersatzeffekt — was beim Zurückkommen ein neues Objekt ist und was \
         eine Fähigkeit davon noch sieht.",
    ),
    (
        include_str!("../../../../data/decks/schwarzrand.txt"),
        "highlander",
        "Ossis Sultai-Highlander: hundert Karten, jede nur einmal, kein \
         Kommandeur. Fast jede davon ist der früheste deutsche Druck, den es \
         gibt — bei allem bis Revised also die deutsche Erstauflage, die als \
         einzige einen schwarzen Rand hatte, und daher der Name. Spielt billige \
         Interaktion, Tutoren und Kartenvorteil; Prüfstein für eine Manabasis \
         aus Fetch- und Dualländern, die fast jede Farbkombination bedienen muss.",
    ),
    (
        include_str!("../../../../data/decks/weltenbaum.txt"),
        "highlander",
        "Dominiks fünffarbiger Länder-Highlander: hundert Karten, jede nur \
         einmal, kein Kommandeur. Gaea's Cradle, Fastbond und Exploration \
         stellen das Mana, The World Tree und Yavimaya öffnen die Farben, und \
         Ramunap Excavator neben Crucible of Worlds macht aus jedem Fetchland \
         eine Dauerquelle. Prüfstein für Landfall, für Länder, die aus dem \
         Friedhof zurückkommen, und für Mana, das von der Anzahl der \
         Permanenten abhängt.",
    ),
];

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (both, front) in FRONT_FACE_REPAIRS {
            let statement = Statement::from_sql_and_values(
                manager.get_database_backend(),
                "UPDATE deck SET cards = array_replace(cards, $1, $2) \
                 WHERE kind = 'house' AND $1 = ANY(cards)",
                [both.into(), front.into()],
            );
            ConnectionTrait::execute_raw(manager.get_connection(), statement).await?;
        }

        for (text, format, description) in DECKS {
            decklist::seed(manager, &Decklist::parse(text), format, description).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (text, _, _) in DECKS {
            decklist::remove(manager, &Decklist::parse(text).name).await?;
        }
        // The two repaired rows are left repaired: `down` undoes what this
        // migration added, not what it corrected, and putting an unresolvable
        // card name back is not a state anybody wants to return to.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_deck_file_is_a_hundred_cards_under_one_name() {
        decklist::assert_seedable(&DECKS);
    }

    #[test]
    fn the_repair_names_a_front_face_and_not_both() {
        for (both, front) in FRONT_FACE_REPAIRS {
            assert!(both.contains(" // "), "{both} is not the broken spelling");
            assert!(!front.contains(" // "), "{front} still names both faces");
            assert!(
                both.starts_with(front),
                "{front} is not the front face of {both}"
            );
        }
    }
}
