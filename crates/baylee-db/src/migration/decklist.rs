//! Reading a `data/decks/*.txt` list, and seeding it as a house deck.
//!
//! Two migrations seed decks and a third will; each one pulls its lists in
//! with `include_str!` and puts them in the same table with the same
//! statement. Writing that twice is how the second copy comes to differ from
//! the first — so the parser, the insert and the removal live here once, and a
//! migration says only *which* decks and *why*.
//!
//! What is deliberately **not** here is the deck's meaning. This crate does
//! not read a row: a deck is stored as the lines it was written with, and
//! whoever takes it to a table is the one who resolves them against the pool.
//! `cargo run -p xtask -- deck-check` is what says a row still parses and
//! still names a card this repository has.

use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

/// A deck file, split into its sections.
///
/// The format is a `[deck:Name]` header, then rows, then optional
/// `[sideboard]` and `[commander]` sections. Blank lines and `#` comments are
/// skipped, so a list may carry its own provenance.
pub(super) struct Decklist {
    pub(super) name: String,
    pub(super) main: Vec<String>,
    pub(super) side: Vec<String>,
    pub(super) commanders: Vec<String>,
}

impl Decklist {
    /// Split one file into its sections.
    ///
    /// An unknown `[section]` falls back to the main deck rather than being
    /// dropped: a row that lands in the wrong list is visible at a table, and
    /// a row that silently disappears is the defect migration 3 had to repair.
    pub(super) fn parse(text: &str) -> Self {
        let mut list = Self {
            name: String::new(),
            main: Vec::new(),
            side: Vec::new(),
            commanders: Vec::new(),
        };
        let mut section = 0u8;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(rest) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                match rest.split_once(':') {
                    Some(("deck", deck)) => {
                        list.name = deck.to_string();
                        section = 0;
                    }
                    _ if rest == "sideboard" => section = 1,
                    _ if rest == "commander" => section = 2,
                    _ => section = 0,
                }
                continue;
            }
            match section {
                1 => list.side.push(line.to_string()),
                2 => list.commanders.push(line.to_string()),
                _ => list.main.push(line.to_string()),
            }
        }
        list
    }

    /// How many cards the deck plays, counting the leader once.
    ///
    /// A row is `<count> <name>…`, so the count is what is summed and never
    /// the number of rows — `cards.len()` is the number of *lines* and reads
    /// as 97 for a hundred-card deck with playsets in it.
    ///
    /// A leader is counted only when no row already names it. Both shapes
    /// exist in this table and both are correct at a table: the deck the
    /// schema shipped named its commander only in `commanders`, and a deck a
    /// player builds carries its commander among the rows — which is the
    /// better of the two, because a row is where the *printing* lives.
    /// `baylee_cards::decks::from_lines` moves a named row out of the library
    /// rather than copying it, so counting it twice here would be counting a
    /// card that is only ever on the table once.
    #[cfg(test)]
    pub(super) fn cards(&self) -> u32 {
        let rows: u32 = self
            .main
            .iter()
            .map(|row| {
                row.split_once(' ')
                    .expect("a deck row is `<count> <name>`")
                    .0
                    .parse::<u32>()
                    .expect("a deck row opens with a count")
            })
            .sum();
        let loose = self
            .commanders
            .iter()
            .filter(|leader| !self.names_a_row(leader))
            .count();
        // Fully qualified: the migration prelude brings a second `try_from`
        // into scope, which makes the short spelling ambiguous here.
        rows + <u32 as TryFrom<usize>>::try_from(loose).expect("few commanders")
    }

    /// Whether one of the deck's rows is this card.
    ///
    /// A leader is stored bare — `Kenrith, the Returned King` — because
    /// `decks::by_name` is an exact-spelling lookup and is what reads the
    /// column. Its row is that name behind a count of one, either alone or
    /// followed by the printing it names.
    #[cfg(test)]
    pub(super) fn names_a_row(&self, leader: &str) -> bool {
        let bare = format!("1 {leader}");
        let printed = format!("{bare} (");
        self.main
            .iter()
            .any(|row| row == &bare || row.starts_with(&printed))
    }
}

/// Seed one deck as a `house` deck, once.
///
/// House decks belong to nobody and are what a copy starts from, so a deck of
/// this project's own may also sit under a player's account — those are
/// different rows and both are correct. What must not happen twice is *this*
/// row, so the guard is on the pair that identifies it.
pub(super) async fn seed(
    manager: &SchemaManager<'_>,
    list: &Decklist,
    format: &str,
    description: &str,
) -> Result<(), DbErr> {
    let statement = Statement::from_sql_and_values(
        manager.get_database_backend(),
        "INSERT INTO deck \
         (account_id, kind, name, format, description, cards, sideboard, \
          commanders, version, updated_at) \
         SELECT NULL, 'house', $1, $2, $3, \
                string_to_array($4, chr(10)), \
                coalesce(string_to_array(nullif($5, ''), chr(10)), '{}'), \
                coalesce(string_to_array(nullif($6, ''), chr(10)), '{}'), \
                1, now() \
         WHERE NOT EXISTS ( \
             SELECT 1 FROM deck WHERE kind = 'house' AND name = $1)",
        [
            list.name.clone().into(),
            format.into(),
            description.into(),
            list.main.join("\n").into(),
            list.side.join("\n").into(),
            list.commanders.join("\n").into(),
        ],
    );
    ConnectionTrait::execute_raw(manager.get_connection(), statement).await?;
    Ok(())
}

/// Give one house deck the rows it should have had, keeping the deck.
///
/// The four engine decks were seeded as bare card names, because at the time
/// there was nothing to say about a printing. Replacing those rows is not
/// editing somebody's deck — a house deck belongs to nobody and nobody has
/// changed it — so `version` deliberately stays where it is: version 1 means
/// "as this schema ships it", and a migration telling the same deck in more
/// detail has not made it a second version of itself.
pub(super) async fn replace_rows(
    manager: &SchemaManager<'_>,
    name: &str,
    cards: &[String],
    commanders: &[String],
) -> Result<(), DbErr> {
    let statement = Statement::from_sql_and_values(
        manager.get_database_backend(),
        "UPDATE deck SET cards = string_to_array($2, chr(10)), \
                         commanders = coalesce(string_to_array(nullif($3, ''), chr(10)), '{}'), \
                         updated_at = now() \
         WHERE kind = 'house' AND name = $1",
        [
            name.into(),
            cards.join("\n").into(),
            commanders.join("\n").into(),
        ],
    );
    ConnectionTrait::execute_raw(manager.get_connection(), statement).await?;
    Ok(())
}

/// Rows seeded with both faces in the name, and the front face each should
/// have named.
///
/// Here rather than in the migration that repairs them, because the migration
/// that takes those decks *further* has to be able to put them back exactly as
/// the repair left them.
pub(super) const FRONT_FACE_REPAIRS: [(&str, &str); 2] = [
    ("1 Fire // Ice", "1 Fire"),
    (
        "1 Fatehold Chronologist // Peer Review",
        "1 Fatehold Chronologist",
    ),
];

/// One seeded row as the repair left it.
pub(super) fn repaired(row: &str) -> String {
    for (both, front) in FRONT_FACE_REPAIRS {
        if row == both {
            return front.to_string();
        }
    }
    row.to_string()
}

/// Take one house deck back out again.
///
/// Only the house copy: a player who copied it owns their copy, and a
/// migration rolling back is not a reason to reach into somebody's account.
pub(super) async fn remove(manager: &SchemaManager<'_>, name: &str) -> Result<(), DbErr> {
    let statement = Statement::from_sql_and_values(
        manager.get_database_backend(),
        "DELETE FROM deck WHERE kind = 'house' AND name = $1",
        [name.into()],
    );
    ConnectionTrait::execute_raw(manager.get_connection(), statement).await?;
    Ok(())
}

/// What every seeded list has to be true about itself.
///
/// Shared because both migrations seed the same shape from the same
/// directory, and a check that exists in one copy is a check the next
/// migration forgets.
#[cfg(test)]
pub(super) fn assert_seedable(decks: &[(&str, &str, &str)]) {
    for (text, format, description) in decks {
        let list = Decklist::parse(text);
        assert!(!list.name.is_empty(), "a deck file with no [deck:…] header");
        assert!(!description.is_empty(), "{} has no description", list.name);
        // A hundred cards either way, and the leader is one of them:
        // Commander is 99 plus the one in the command zone (CR 903.5a),
        // Highlander is 100 with nothing in front of it. A deck that is
        // neither is a deck somebody edited without saying so.
        let total = list.cards();
        assert_eq!(total, 100, "{} plays {total} cards, not 100", list.name);
        match *format {
            "commander" => assert_eq!(list.commanders.len(), 1, "{} leads with nobody", list.name),
            "highlander" => assert!(list.commanders.is_empty(), "{} has a leader", list.name),
            other => panic!("{} plays {other}, which no migration knows", list.name),
        }
        assert!(list.side.len() <= 250, "{} has a huge sideboard", list.name);
        // A leader is a bare card name and never a row: `decks::by_name` is
        // an exact-spelling lookup and `from_lines` drops what it cannot
        // resolve, so a leader written as `1 General Tazri (OGW) 19` seats
        // nobody and says nothing about it.
        for leader in &list.commanders {
            assert!(
                !leader.starts_with(|c: char| c.is_ascii_digit()),
                "{}: leader {leader} is written as a row",
                list.name
            );
            assert!(
                list.names_a_row(leader),
                "{}: leader {leader} has no row, so it has no printing",
                list.name
            );
        }
        // The pool names a two-faced card by its front face — there is not
        // one `//` among the names it compiles — so a row naming both faces
        // resolves to nothing and is dropped in silence. That is the defect
        // migration 3 repairs, kept from coming back through a new list.
        for row in list.main.iter().chain(&list.side).chain(&list.commanders) {
            assert!(
                !row.contains(" // "),
                "{}: {row} names both faces",
                list.name
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Decklist, FRONT_FACE_REPAIRS, repaired};

    /// **A section nobody knows puts its rows in the deck, not in the bin.**
    ///
    /// That is the fallback this parser is written around: a row in the
    /// wrong list is visible at a table, and a row that disappeared is the
    /// defect migration 3 had to repair. No deck file in the tree writes an
    /// unknown section, so nothing else reaches the branch — which is the
    /// whole reason it is worth a test rather than a comment.
    ///
    /// `[sideboard:in]` is the same branch wearing a known word: the match
    /// is on the header whole, so anything after a colon that is not `deck`
    /// is a section this parser has never heard of.
    #[test]
    fn a_section_nobody_knows_puts_its_rows_in_the_deck_and_not_in_the_bin() {
        let list = Decklist::parse(
            "[deck:Test]\n\
             4 Lightning Bolt\n\
             [maybeboard]\n\
             2 Forest\n\
             [sideboard:in]\n\
             1 Island\n",
        );

        assert_eq!(list.name, "Test");
        assert_eq!(list.main, ["4 Lightning Bolt", "2 Forest", "1 Island"]);
        assert!(
            list.side.is_empty() && list.commanders.is_empty(),
            "and nothing was routed by a word that merely looked familiar"
        );
    }

    /// The three sections the format has, and the one movement that is easy
    /// to leave out: a `[deck:…]` header **returns to the main list**, so a
    /// file that names its deck again after the sideboard does not keep
    /// writing into the sideboard.
    #[test]
    fn each_section_takes_the_rows_under_it_and_a_deck_header_returns_to_the_first() {
        let list = Decklist::parse(
            "[deck:Two Sections]\n\
             4 Lightning Bolt\n\
             [sideboard]\n\
             2 Forest\n\
             [commander]\n\
             Kenrith, the Returned King\n\
             [deck:Two Sections]\n\
             1 Island\n",
        );

        assert_eq!(list.name, "Two Sections");
        assert_eq!(list.main, ["4 Lightning Bolt", "1 Island"]);
        assert_eq!(list.side, ["2 Forest"]);
        assert_eq!(list.commanders, ["Kenrith, the Returned King"]);
    }

    /// A comment and a blank line are skipped **wherever they stand**, which
    /// is what lets a list carry its own provenance beside the rows it is
    /// about — and neither of them ends the section they are written in.
    #[test]
    fn a_comment_and_a_blank_line_are_skipped_inside_a_section_too() {
        let list = Decklist::parse(
            "# where this list came from\n\
             \n\
             [deck:Commented]\n\
             \n\
             4 Lightning Bolt\n\
             # and why that card is in it\n\
             \n\
             [sideboard]\n\
             # this one is for the mirror\n\
             2 Forest\n",
        );

        assert_eq!(list.name, "Commented");
        assert_eq!(list.main, ["4 Lightning Bolt"]);
        assert_eq!(list.side, ["2 Forest"]);
    }

    /// A file with no header keeps every row and has no name. Refusing it is
    /// `assert_seedable`'s job and not this one: what a parser owes is to
    /// lose nothing, and saying that a nameless deck may not be seeded is a
    /// sentence one layer up.
    #[test]
    fn a_file_with_no_header_keeps_its_rows_and_has_no_name() {
        let list = Decklist::parse("4 Lightning Bolt\n  2 Forest  \n");

        assert!(list.name.is_empty());
        assert_eq!(
            list.main,
            ["4 Lightning Bolt", "2 Forest"],
            "and the row is what was written, trimmed"
        );
    }

    /// `repaired` matches a **whole row**, so a row that merely opens with a
    /// repaired name is left alone.
    ///
    /// That is right only because the two rows it names are rows this
    /// repository seeded, and neither of them names a printing. A seeded row
    /// that did would need its own entry rather than a prefix match, which
    /// would reach rows nobody meant.
    #[test]
    fn a_repair_names_one_whole_row_and_never_a_prefix_of_it() {
        for (both, front) in FRONT_FACE_REPAIRS {
            assert_eq!(repaired(both), front, "the row the repair is about");
            assert_eq!(repaired(front), front, "and it is idempotent");
            assert_eq!(
                repaired(&format!("{both} (MH2) 290")),
                format!("{both} (MH2) 290"),
                "a printing of the same pair is a row this does not name"
            );
        }
        assert_eq!(
            repaired("4 Lightning Bolt"),
            "4 Lightning Bolt",
            "and every other row travels through unchanged"
        );
    }
}
