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
    /// The subset of [`Self::stack`] whose characteristics the layer system
    /// can change — spells and copies of spells, never abilities.
    ///
    /// The layer refresh runs once per engine step and re-walks the stack
    /// every time an effect set or a counter changes. An ability on the
    /// stack has nothing for layers to modify, so visiting it is pure
    /// waste — and an Ally deck can leave six figures of them there, which
    /// turns "pure waste" into a game that never finishes. Keeping the
    /// projectable subset here rather than filtering on the way past means
    /// the refresh never touches the abilities at all.
    stack_projectable: Vec<ObjectId>,
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
            stack_projectable: Vec::new(),
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
        if loc == ZoneLocation::Stack
            && let Some(pos) = self.stack_projectable.iter().position(|&x| x == id)
        {
            // Cheap even on a huge stack: this list holds only the spells.
            self.stack_projectable.remove(pos);
        }
        let list = self.list_mut(loc);
        if let Some(pos) = list.iter().position(|&x| x == id) {
            list.remove(pos);
            true
        } else {
            false
        }
    }

    /// Inserts an object into a zone at the given position.
    ///
    /// `projectable` says whether the layer system can change this object's
    /// characteristics; it is only consulted for the stack, where it
    /// separates spells from abilities. It is a parameter rather than
    /// something derived here because [`Zones`] has no arena to ask — and
    /// making every caller answer is what keeps [`Self::stack_projectable`]
    /// from silently drifting out of sync with the stack.
    pub fn insert(
        &mut self,
        id: ObjectId,
        loc: ZoneLocation,
        pos: ZonePosition,
        projectable: bool,
    ) {
        let list = self.list_mut(loc);
        match pos {
            ZonePosition::Top => list.push(id),
            ZonePosition::Bottom => list.insert(0, id),
            ZonePosition::Index(i) => list.insert(i.min(list.len()), id),
        }
        if loc == ZoneLocation::Stack && projectable {
            self.stack_projectable.push(id);
        }
    }

    /// The stack objects the layer refresh has to visit.
    ///
    /// Order is insertion order rather than stack order, which is fine
    /// because every projection is computed independently of the others —
    /// nothing here depends on what was projected before it.
    #[must_use]
    pub fn stack_projectable(&self) -> &[ObjectId] {
        &self.stack_projectable
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
        zones.insert(id(1), lib, ZonePosition::Top, true);
        zones.insert(id(2), lib, ZonePosition::Top, true);
        zones.insert(id(3), lib, ZonePosition::Bottom, true);
        assert_eq!(zones.list(lib).as_slice(), &[id(3), id(1), id(2)]);
        assert!(zones.remove(id(1), lib));
        assert_eq!(zones.list(lib).as_slice(), &[id(3), id(2)]);
        assert!(!zones.remove(id(9), lib));
    }

    /// The projectable subset has to survive removals from the middle of a
    /// stack — a countered spell leaves under the abilities stacked on top
    /// of it, and if its id stayed behind the layer refresh would keep
    /// projecting an object that no longer exists.
    #[test]
    fn the_projectable_subset_tracks_the_stack() {
        let mut zones = Zones::new(2);
        let stack = ZoneLocation::Stack;
        zones.insert(id(1), stack, ZonePosition::Top, true); // a spell
        zones.insert(id(2), stack, ZonePosition::Top, false); // its trigger
        zones.insert(id(3), stack, ZonePosition::Top, true); // a response

        assert_eq!(zones.list(stack).as_slice(), &[id(1), id(2), id(3)]);
        assert_eq!(zones.stack_projectable(), &[id(1), id(3)]);

        assert!(zones.remove(id(1), stack), "the spell is countered");
        assert_eq!(zones.list(stack).as_slice(), &[id(2), id(3)]);
        assert_eq!(zones.stack_projectable(), &[id(3)]);

        assert!(zones.remove(id(2), stack), "the ability resolves");
        assert_eq!(
            zones.stack_projectable(),
            &[id(3)],
            "abilities were never in it"
        );

        assert!(zones.remove(id(3), stack));
        assert!(zones.stack_projectable().is_empty());
    }

    /// The flag is a stack concept. Everywhere else it is ignored, and a
    /// battlefield permanent must not leak into the stack's subset.
    #[test]
    fn only_the_stack_has_a_projectable_subset() {
        let mut zones = Zones::new(2);
        zones.insert(id(1), ZoneLocation::Battlefield, ZonePosition::Top, true);
        zones.insert(
            id(2),
            ZoneLocation::Graveyard(PlayerId::new(0)),
            ZonePosition::Top,
            true,
        );
        assert!(zones.stack_projectable().is_empty());
    }

    /// The eight zones, written out because there is no `ALL` to walk and a
    /// ninth would otherwise be added to one of these functions and not the
    /// others. A `Zone` reached by neither `of` nor this list is a zone
    /// nothing below is asked about.
    const EVERY_ZONE: [Zone; 8] = [
        Zone::Library,
        Zone::Hand,
        Zone::Battlefield,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Stack,
        Zone::Command,
        Zone::OutsideGame,
    ];

    /// Three functions, one fact. `of` builds a location, `zone` reads the
    /// kind back off it and `player` says whose it is — and `is_shared` says
    /// the same thing from the other end, in a different file's vocabulary.
    /// A zone added to one of them and not the others makes the pair
    /// disagree silently: a shared zone that carries a player would give
    /// every seat its own battlefield.
    #[test]
    fn a_zone_and_its_location_agree_about_whose_it_is() {
        let p = PlayerId::new(1);
        for zone in EVERY_ZONE {
            let loc = ZoneLocation::of(zone, p);
            assert_eq!(loc.zone(), zone, "{zone:?} did not survive the round trip");
            assert_eq!(
                loc.player().is_none(),
                zone.is_shared(),
                "{zone:?}: `player` and `is_shared` disagree"
            );
            if !zone.is_shared() {
                assert_eq!(loc.player(), Some(p), "{zone:?} kept somebody else's seat");
                assert_ne!(
                    ZoneLocation::of(zone, PlayerId::new(0)),
                    loc,
                    "{zone:?}: two seats' zones are two locations"
                );
            }
        }
    }

    /// A shared zone is the same location whoever asks for it, which is what
    /// lets a caller pass any seat when it has one to hand.
    #[test]
    fn a_shared_zone_ignores_the_player_it_is_asked_with() {
        for zone in EVERY_ZONE.into_iter().filter(|z| z.is_shared()) {
            assert_eq!(
                ZoneLocation::of(zone, PlayerId::new(0)),
                ZoneLocation::of(zone, PlayerId::new(7)),
                "{zone:?}"
            );
        }
        assert!(Zone::Battlefield.is_shared() && Zone::Stack.is_shared());
    }

    /// The three zones a view may not simply hand over. Written as a list
    /// rather than derived, because this is the rule itself: a graveyard is
    /// public even though a hand is not, and the sideboard is hidden even
    /// though it is not in the game at all.
    #[test]
    fn the_hidden_zones_are_the_library_the_hand_and_the_cards_outside_the_game() {
        let hidden: Vec<Zone> = EVERY_ZONE
            .into_iter()
            .filter(|z| z.is_hidden_by_default())
            .collect();
        assert_eq!(
            hidden,
            vec![Zone::Library, Zone::Hand, Zone::OutsideGame],
            "a zone joining or leaving this list is a change to what a \
             player may see"
        );
    }

    /// Order is significant and removal preserves it: the top of a library
    /// is the end of its `Vec`, so a swap-remove would reorder the cards
    /// under the one that left — and a library is shuffled exactly when the
    /// rules say and never by a data structure.
    #[test]
    fn removing_from_the_middle_leaves_the_order_alone() {
        let mut zones = Zones::new(2);
        let lib = ZoneLocation::Library(PlayerId::new(0));
        for slot in 1..=5 {
            zones.insert(id(slot), lib, ZonePosition::Top, true);
        }
        assert_eq!(
            zones.list(lib).as_slice(),
            &[id(1), id(2), id(3), id(4), id(5)]
        );
        assert!(zones.remove(id(3), lib));
        assert_eq!(
            zones.list(lib).as_slice(),
            &[id(1), id(2), id(4), id(5)],
            "the cards under it kept their order"
        );
        assert!(zones.contains(id(5), lib));
        assert!(!zones.contains(id(3), lib));
        assert!(
            !zones.contains(id(5), ZoneLocation::Library(PlayerId::new(1))),
            "and it is in one player's library, not in the zone kind"
        );
    }
}
