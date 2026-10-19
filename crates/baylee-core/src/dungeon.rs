//! The three dungeons entered by the venture keyword action.

/// A dungeon that ordinary venture can enter (the Undercity requires initiative).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Dungeon {
    /// Seven rooms, with two branching levels.
    LostMine,
    /// Nine rooms, with two branching levels.
    MadMage,
    /// Five rooms, including the Oubliette shortcut.
    Tomb,
}

impl Dungeon {
    /// The complete ordinary-venture menu, in stable order.
    pub const ALL: [Self; 3] = [Self::LostMine, Self::MadMage, Self::Tomb];

    /// Printed English name, as for other untranslated card names.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::LostMine => "Lost Mine of Phandelver",
            Self::MadMage => "Dungeon of the Mad Mage",
            Self::Tomb => "Tomb of Annihilation",
        }
    }

    /// Printed room names in graph-index order; room zero is the entrance.
    #[must_use]
    pub const fn rooms(self) -> &'static [&'static str] {
        match self {
            Self::LostMine => &[
                "Cave Entrance",
                "Goblin Lair",
                "Mine Tunnels",
                "Storeroom",
                "Dark Pool",
                "Fungi Cavern",
                "Temple of Dumathoin",
            ],
            Self::MadMage => &[
                "Yawning Portal",
                "Dungeon Level",
                "Goblin Bazaar",
                "Twisted Caverns",
                "Lost Level",
                "Runestone Caverns",
                "Muiral's Graveyard",
                "Deep Mines",
                "Mad Wizard's Lair",
            ],
            Self::Tomb => &[
                "Trapped Entry",
                "Veils of Fear",
                "Sandfall Cell",
                "Oubliette",
                "Cradle of the Death God",
            ],
        }
    }

    /// Printed arrows out of one room. Empty means the last room.
    #[must_use]
    pub const fn next(self, room: u8) -> &'static [u8] {
        match (self, room) {
            (Self::LostMine, 0) => &[1, 2],
            (Self::LostMine, 1) => &[3, 4],
            (Self::LostMine, 2) => &[4, 5],
            (Self::LostMine, 3..=5) => &[6],
            (Self::MadMage, 0) => &[1],
            (Self::MadMage, 1) => &[2, 3],
            (Self::MadMage | Self::Tomb, 2 | 3) => &[4],
            (Self::MadMage, 4) => &[5, 6],
            (Self::MadMage, 5 | 6) => &[7],
            (Self::MadMage, 7) => &[8],
            (Self::Tomb, 0) => &[1, 3],
            (Self::Tomb, 1) => &[2],
            _ => &[],
        }
    }

    /// Printed Oracle effect text of each room in [`Self::rooms`] order.
    #[must_use]
    pub const fn room_effects(self) -> &'static [&'static str] {
        match self {
            Self::LostMine => &[
                "Scry 1.",
                "Create a 1/1 red Goblin creature token.",
                "Create a Treasure token.",
                "Put a +1/+1 counter on target creature.",
                "Each opponent loses 1 life and you gain 1 life.",
                "Target creature gets -4/-0 until your next turn.",
                "Draw a card.",
            ],
            Self::MadMage => &[
                "You gain 1 life.",
                "Scry 1.",
                "Create a Treasure token.",
                "Target creature can't attack until your next turn.",
                "Scry 2.",
                "Exile the top two cards of your library. You may play them.",
                "Create two 1/1 black Skeleton creature tokens.",
                "Scry 3.",
                "Draw three cards and reveal them. You may cast one of them without paying its mana cost.",
            ],
            Self::Tomb => &[
                "Each player loses 1 life.",
                "Each player loses 2 life unless they discard a card.",
                "Each player loses 2 life unless they sacrifice a creature, artifact, or land of their choice.",
                "Discard a card and sacrifice a creature, an artifact, and a land.",
                "Create The Atropal, a legendary 4/4 black God Horror creature token with deathtouch.",
            ],
        }
    }

    /// Resolves a projected dungeon name (`"<Dungeon> — <Room>"` or `"<Dungeon>"`)
    /// into its [`Dungeon`] and 0-based active room index.
    #[must_use]
    pub fn from_public_name(name: &str) -> Option<(Self, usize)> {
        for dungeon in Self::ALL {
            if name == dungeon.name() {
                return Some((dungeon, 0));
            }
            if let Some(rest) = name
                .strip_prefix(dungeon.name())
                .and_then(|s| s.strip_prefix(" — "))
                && let Some(idx) = dungeon.rooms().iter().position(|&r| r == rest)
            {
                return Some((dungeon, idx));
            }
        }
        None
    }
}
