//! A saved deck.

use sea_orm::entity::prelude::*;

/// The kind a player's own deck is.
pub const KIND_ACCOUNT: &str = "account";
/// The kind a retail product is: bought, fixed, the same for everyone.
pub const KIND_PRECONSTRUCTED: &str = "preconstructed";
/// The kind this project publishes to be played with.
pub const KIND_HOUSE: &str = "house";

/// Every kind there is, which is also the order a listing shows them in.
pub const KINDS: [&str; 3] = [KIND_ACCOUNT, KIND_PRECONSTRUCTED, KIND_HOUSE];

/// Whether a string is a deck kind.
///
/// The database has the same list as a `CHECK`, and that is the enforcing
/// half; this is so a route can refuse a request with a message about the
/// kind rather than handing the caller a constraint violation.
#[must_use]
pub fn is_kind(kind: &str) -> bool {
    KINDS.contains(&kind)
}

/// One deck, as the player built it.
///
/// Card *names*, not registry indices: a deck outlives the pool it was built
/// against, and a name still resolves when an index has been re-issued. The
/// registry lookup happens when the deck is taken to a table.
///
/// This row is the deck **now**. What it used to hold is
/// [`super::deck_version`], and the two never describe the same state — see
/// that module for why the head is not stored twice.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "deck")]
pub struct Model {
    /// `UUIDv7`.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// Whose deck it is — `None` for every kind but [`KIND_ACCOUNT`], which
    /// the database holds both ways round with a `CHECK`.
    pub account_id: Option<Uuid>,
    /// Which of [`KINDS`] this is.
    pub kind: String,
    /// What the player called it.
    pub name: String,
    /// What it plays: `commander`, `freeform`, … Stored rather than derived,
    /// so a deck can say what it is *before* it has a commander in it.
    pub format: String,
    /// What the deck is for, in its owner's words.
    pub description: Option<String>,
    /// The deck this one was copied from, if it was.
    pub copied_from: Option<Uuid>,
    /// Which *state* of that deck was copied — its [`Self::version`] at the
    /// moment of the copy, so the copy can name what it came from even after
    /// the original has moved on.
    pub copied_version: Option<i32>,
    /// The version the cards below are. Starts at 1 and is incremented by
    /// every save that changes them, which is what leaves the state it
    /// replaced behind in [`super::deck_version`].
    pub version: i32,
    /// Main deck, as `"N Card Name"` lines.
    ///
    /// A Postgres `text[]` rather than a `jsonb` document or a second table.
    /// A decklist is read and written whole, is never queried into, and is
    /// ordered — which is exactly the shape an array is for, and exactly the
    /// shape a `deck_card` table with a position column would be an
    /// elaborate way of getting back.
    pub cards: Vec<String>,
    /// Sideboard, in the same form. Cards outside the game a seat may reach;
    /// never shuffled into the library.
    pub sideboard: Vec<String>,
    /// The deck's commanders — none, one, or two under the partner rule
    /// (CR 702.124).
    ///
    /// A list and not a nullable column, because that is what
    /// `baylee_core::preset::SeatSpec::commanders` has always taken and what
    /// the command zone holds. An empty array is a deck that is not playing
    /// Commander.
    pub commanders: Vec<String>,
    /// Image id of the sleeve these cards show face-down. `None` means the
    /// client draws its own generated back.
    pub sleeve: Option<String>,
    /// Image id of the playmat this deck's seat plays on.
    pub playmat: Option<String>,
    /// Last saved.
    pub updated_at: TimeDateTimeWithTimeZone,
}

/// The owning account.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// Cascades.
    #[sea_orm(
        belongs_to = "super::account::Entity",
        from = "Column::AccountId",
        to = "super::account::Column::Id",
        on_delete = "Cascade"
    )]
    Account,
}

impl Related<super::account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Account.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
