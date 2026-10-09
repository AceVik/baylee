//! The walk over every card identity a view shows ([`PlayerView::identities`]),
//! and what it yields.
//!
//! Kept out of the files `tests::wire_shape` reads: nothing here is sent.
//! It is a reading of a view, never a part of one.
//!
//! [`PlayerView::identities`]: crate::PlayerView::identities

use crate::objects::{CardIdentity, DamageSourceView, HandObject, PublicObject};
use baylee_core::ids::{ObjectId, PlayerId};

/// Where [`PlayerView::identities`](crate::PlayerView::identities) found a card identity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SeenIn {
    /// The seat's own hand.
    Hand,
    /// A teammate's hand its owner is showing this seat (#265).
    SharedHand,
    /// A hand this seat inspects through controlling its player (CR 720.4).
    ControlledHand,
    /// The battlefield.
    Battlefield,
    /// The stack.
    Stack,
    /// A graveyard.
    Graveyard,
    /// Exile.
    Exile,
    /// A command zone.
    Command,
    /// What the seat is being shown to answer the question it is asked.
    LookingAt,
    /// A library's top card, revealed to the table.
    LibraryTop,
    /// The source of the targeting question being asked.
    TargetingSource,
    /// A commander, as the seat list names it (CR 903.3).
    Commander,
    /// An offered damage source, as it was when offered.
    DamageSource,
    /// A target, as it was when targeted.
    TargetObject,
}

/// One card identity a view shows, and where ([`PlayerView::identities`](crate::PlayerView::identities)).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Seen<'a> {
    /// Where it was found.
    pub zone: SeenIn,
    /// The card, its printing and face.
    pub card: CardIdentity,
    /// The object it is, where the view names one.
    pub object: Option<ObjectId>,
    /// The name the view gives it (projected; from the registry, English).
    pub name: &'a str,
    /// Its owner, where the view says.
    pub owner: Option<PlayerId>,
    /// Whether it lies face down: its controller is told the card, nobody
    /// else is (CR 708.5).
    pub face_down: bool,
}

/// [`PlayerView::identities`](crate::PlayerView::identities)' reading of a hand card in `zone`, owned by
/// `owner`.
pub(crate) fn from_hand<'a>(zone: SeenIn, owner: PlayerId) -> impl Fn(&'a HandObject) -> Seen<'a> {
    move |o| Seen {
        zone,
        card: o.card,
        object: Some(o.id),
        name: o.name.as_str(),
        owner: Some(owner),
        face_down: false,
    }
}

/// [`PlayerView::identities`](crate::PlayerView::identities)' reading of an object in `zone`: nothing for
/// one with no card the seat may know.
pub(crate) fn from_public<'a>(zone: SeenIn) -> impl Fn(&'a PublicObject) -> Option<Seen<'a>> {
    move |o| {
        o.card.map(|card| Seen {
            zone,
            card,
            object: Some(o.id),
            name: o.name.as_str(),
            owner: Some(o.owner),
            face_down: o.status.is_face_down(),
        })
    }
}

/// [`PlayerView::identities`](crate::PlayerView::identities)' reading of a described source or target.
pub(crate) fn from_described<'a>(
    zone: SeenIn,
) -> impl Fn(&'a DamageSourceView) -> Option<Seen<'a>> {
    move |s| {
        s.card.map(|card| Seen {
            zone,
            card,
            object: Some(s.source.object),
            name: s.name.as_str(),
            owner: None,
            face_down: false,
        })
    }
}
