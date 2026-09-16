//! What a deck used to hold.
//!
//! One row per state a deck has **left behind**, written at the moment it was
//! replaced. The deck's current cards are on [`super::deck`] and are not
//! repeated here, which is the whole point: the two tables never describe the
//! same state, so there is no pair to keep in step and no way for them to
//! disagree about what the deck holds now.
//!
//! Reading the history is therefore the rows plus the deck itself — the rows
//! are versions 1 to `deck.version - 1`, and `deck.version` is the deck. And
//! reverting to version *k* is an ordinary save of version *k*'s lists, which
//! archives the present the way any other save does: a revert is a change
//! like every other change, and is itself revertible.

use sea_orm::entity::prelude::*;

/// One superseded state of one deck.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "deck_version")]
pub struct Model {
    /// Which deck.
    #[sea_orm(primary_key, auto_increment = false)]
    pub deck_id: Uuid,
    /// Which of its versions this was.
    ///
    /// Part of the key rather than a column beside one: there is exactly one
    /// state a deck had at version 4, and a second row claiming to be it is a
    /// forked history rather than a duplicate — so a racing save fails loudly
    /// instead of quietly writing a second past.
    #[sea_orm(primary_key, auto_increment = false)]
    pub version: i32,
    /// The main deck it held, as `"N Card Name"` lines.
    pub cards: Vec<String>,
    /// The sideboard it held.
    pub sideboard: Vec<String>,
    /// The commanders it had.
    pub commanders: Vec<String>,
    /// What the change was, in the words of whoever made it.
    ///
    /// The change, not the deck: `deck.description` says what the deck is
    /// for, and a card's own note rides in its row (`deckrow::NOTE_FENCE`).
    /// This is the third of the three and the only one about an *edit*.
    pub summary: Option<String>,
    /// When this state stopped being the current one.
    pub superseded_at: TimeDateTimeWithTimeZone,
}

/// The deck this state belonged to.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Cascades: a deleted deck takes its history with it.
    #[sea_orm(
        belongs_to = "super::deck::Entity",
        from = "Column::DeckId",
        to = "super::deck::Column::Id",
        on_delete = "Cascade"
    )]
    Deck,
}

impl Related<super::deck::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Deck.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
