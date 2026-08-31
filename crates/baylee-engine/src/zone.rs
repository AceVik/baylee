//! Zones and their ordered storage.
//!
//! Order is significant: the top of a library is the END of its `Vec`,
//! the top of the stack is the END, and the most recent graveyard card is
//! the END. Removal preserves order (`Vec::remove`, not swap).

use baylee_core::ids::{ObjectId, PlayerId};
use serde::{Deserialize, Serialize};

/// The seven zones (CR 400.1), plus one that is not a zone at all.
/// Phasing is a status, not a zone (CR 702.26).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Zone {
    /// Per-player, ordered, hidden.
    Library,
    /// Per-player, ordered (insertion), hidden from opponents.
    Hand,
    /// Shared, unordered list (controllers distinguish).
    Battlefield,
    /// Per-player, ordered, public.
    Graveyard,
    /// Per-player, ordered, public.
    Exile,
    /// Shared, ordered (top = end).
    Stack,
    /// Per-player (commanders, emblems, schemes, dungeons).
    Command,
    /// Not a zone in the rules: cards *outside the game* are in no zone at
    /// all (CR 400.1). They are stored as one anyway, because a wish has to
    /// offer them as objects with ids — and giving them a home makes them
    /// impossible to confuse with anything in the game.
    OutsideGame,
}

impl Zone {
    /// Whether the zone is shared rather than per-player.
    #[must_use]
    pub const fn is_shared(self) -> bool {
        matches!(self, Zone::Battlefield | Zone::Stack)
    }

    /// Whether the zone's contents are hidden from other players by default.
    #[must_use]
    pub const fn is_hidden_by_default(self) -> bool {
        matches!(self, Zone::Library | Zone::Hand | Zone::OutsideGame)
    }
}

/// A concrete zone location: per-player zones carry their player.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ZoneLocation {
    /// A player's library.
    Library(PlayerId),
    /// A player's hand.
    Hand(PlayerId),
    /// The battlefield.
    Battlefield,
    /// A player's graveyard.
    Graveyard(PlayerId),
    /// A player's exile.
    Exile(PlayerId),
    /// The stack.
    Stack,
    /// A player's command zone.
    Command(PlayerId),
    /// A player's cards outside the game (sideboard).
    OutsideGame(PlayerId),
}

impl ZoneLocation {
    /// The zone kind of this location.
    #[must_use]
    pub const fn zone(self) -> Zone {
        match self {
            ZoneLocation::Library(_) => Zone::Library,
            ZoneLocation::Hand(_) => Zone::Hand,
            ZoneLocation::Battlefield => Zone::Battlefield,
            ZoneLocation::Graveyard(_) => Zone::Graveyard,
            ZoneLocation::Exile(_) => Zone::Exile,
            ZoneLocation::Stack => Zone::Stack,
            ZoneLocation::Command(_) => Zone::Command,
            ZoneLocation::OutsideGame(_) => Zone::OutsideGame,
        }
    }

    /// The owning player, if per-player.
    #[must_use]
    pub const fn player(self) -> Option<PlayerId> {
        match self {
            ZoneLocation::Library(p)
            | ZoneLocation::Hand(p)
            | ZoneLocation::Graveyard(p)
            | ZoneLocation::Exile(p)
            | ZoneLocation::Command(p)
            | ZoneLocation::OutsideGame(p) => Some(p),
            ZoneLocation::Battlefield | ZoneLocation::Stack => None,
        }
    }

    /// Builds the per-player location for a zone (shared zones ignore `p`).
    #[must_use]
    pub const fn of(zone: Zone, p: PlayerId) -> Self {
        match zone {
            Zone::Library => ZoneLocation::Library(p),
            Zone::Hand => ZoneLocation::Hand(p),
            Zone::Battlefield => ZoneLocation::Battlefield,
            Zone::Graveyard => ZoneLocation::Graveyard(p),
            Zone::Exile => ZoneLocation::Exile(p),
            Zone::Stack => ZoneLocation::Stack,
            Zone::Command => ZoneLocation::Command(p),
            Zone::OutsideGame => ZoneLocation::OutsideGame(p),
        }
    }
}

/// Where exactly an object enters a zone.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ZonePosition {
    /// The end of the list (library top, stack top, newest graveyard card).
    Top,
    /// The start of the list (library bottom).
    Bottom,
    /// A specific index.
    Index(usize),
}

/// Ordered zone contents for the whole game.
#[derive(Clone, Debug)]
pub struct Zones {
    battlefield: Vec<ObjectId>,
    stack: Vec<ObjectId>,
    libraries: Vec<Vec<ObjectId>>,
    hands: Vec<Vec<ObjectId>>,
    graveyards: Vec<Vec<ObjectId>>,
    exiles: Vec<Vec<ObjectId>>,
    commands: Vec<Vec<ObjectId>>,
    outside: Vec<Vec<ObjectId>>,
}

impl Zones {
    /// Empty zone storage for `players` seats.
    #[must_use]
    pub fn new(players: usize) -> Self {
        Self {
            battlefield: Vec::new(),
            stack: Vec::new(),
            libraries: vec![Vec::new(); players],
            hands: vec![Vec::new(); players],
            graveyards: vec![Vec::new(); players],
            exiles: vec![Vec::new(); players],
            commands: vec![Vec::new(); players],
            outside: vec![Vec::new(); players],
        }
    }

    /// Read access to a zone's contents.
    #[must_use]
    pub fn list(&self, loc: ZoneLocation) -> &Vec<ObjectId> {
        match loc {
            ZoneLocation::Battlefield => &self.battlefield,
            ZoneLocation::Stack => &self.stack,
            ZoneLocation::Library(p) => &self.libraries[p.get() as usize],
            ZoneLocation::Hand(p) => &self.hands[p.get() as usize],
            ZoneLocation::Graveyard(p) => &self.graveyards[p.get() as usize],
            ZoneLocation::Exile(p) => &self.exiles[p.get() as usize],
            ZoneLocation::Command(p) => &self.commands[p.get() as usize],
            ZoneLocation::OutsideGame(p) => &self.outside[p.get() as usize],
        }
    }

    /// Mutable access to a zone's contents.
    pub fn list_mut(&mut self, loc: ZoneLocation) -> &mut Vec<ObjectId> {
        match loc {
            ZoneLocation::Battlefield => &mut self.battlefield,
            ZoneLocation::Stack => &mut self.stack,
            ZoneLocation::Library(p) => &mut self.libraries[p.get() as usize],
            ZoneLocation::Hand(p) => &mut self.hands[p.get() as usize],
            ZoneLocation::Graveyard(p) => &mut self.graveyards[p.get() as usize],
            ZoneLocation::Exile(p) => &mut self.exiles[p.get() as usize],
            ZoneLocation::Command(p) => &mut self.commands[p.get() as usize],
            ZoneLocation::OutsideGame(p) => &mut self.outside[p.get() as usize],
        }
    }

    /// Removes an object from a zone, preserving order. Returns success.
    pub fn remove(&mut self, id: ObjectId, loc: ZoneLocation) -> bool {
        let list = self.list_mut(loc);
        if let Some(pos) = list.iter().position(|&x| x == id) {
            list.remove(pos);
            true
        } else {
            false
        }
    }

    /// Inserts an object into a zone at the given position.
    pub fn insert(&mut self, id: ObjectId, loc: ZoneLocation, pos: ZonePosition) {
        let list = self.list_mut(loc);
        match pos {
            ZonePosition::Top => list.push(id),
            ZonePosition::Bottom => list.insert(0, id),
            ZonePosition::Index(i) => list.insert(i.min(list.len()), id),
        }
    }

    /// Whether the object is in the given zone.
    #[must_use]
    pub fn contains(&self, id: ObjectId, loc: ZoneLocation) -> bool {
        self.list(loc).contains(&id)
    }

    /// Whether the stack is empty (timing rules need this constantly).
    #[must_use]
    pub fn stack_is_empty(&self) -> bool {
        self.stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    #[test]
    fn ordered_insert_remove() {
        let mut zones = Zones::new(2);
        let lib = ZoneLocation::Library(PlayerId::new(0));
        zones.insert(id(1), lib, ZonePosition::Top);
        zones.insert(id(2), lib, ZonePosition::Top);
        zones.insert(id(3), lib, ZonePosition::Bottom);
        assert_eq!(zones.list(lib).as_slice(), &[id(3), id(1), id(2)]);
        assert!(zones.remove(id(1), lib));
        assert_eq!(zones.list(lib).as_slice(), &[id(3), id(2)]);
        assert!(!zones.remove(id(9), lib));
    }
}
