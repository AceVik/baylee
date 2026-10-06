use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- turn shape

/// A phase of the turn (CR 500).
///
/// A wire-stable enum rather than a debug-formatted string: renaming an engine
/// variant must not silently change the protocol.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Phase {
    /// Beginning phase.
    Beginning,
    /// Precombat main phase.
    FirstMain,
    /// Combat phase.
    Combat,
    /// Postcombat main phase.
    SecondMain,
    /// Ending phase.
    Ending,
}

/// The game's day/night designation (CR 730.1).
///
/// Wire-stable for the reason [`Phase`] is, and `Option`al where it is
/// carried: a game starts with neither designation and keeps having neither
/// until a card gives it one, which is most games.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DayNight {
    /// It is day.
    Day,
    /// It is night.
    Night,
}

/// A step within a phase (CR 500.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Step {
    /// Untap step.
    Untap,
    /// Upkeep step.
    Upkeep,
    /// Draw step.
    Draw,
    /// A main phase (no step boundary in rules terms).
    Main,
    /// Beginning of combat step.
    CombatBegin,
    /// Declare attackers step.
    DeclareAttackers,
    /// Declare blockers step.
    DeclareBlockers,
    /// First-strike combat damage step.
    CombatDamageFirst,
    /// Regular combat damage step.
    CombatDamage,
    /// End of combat step.
    CombatEnd,
    /// End step.
    End,
    /// Cleanup step.
    Cleanup,
}

impl Step {
    /// A short label for the turn-structure strip in a client.
    #[must_use]
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Untap => "UT",
            Self::Upkeep => "UP",
            Self::Draw => "DR",
            Self::Main => "M",
            Self::CombatBegin => "BC",
            Self::DeclareAttackers => "DA",
            Self::DeclareBlockers => "DB",
            Self::CombatDamageFirst => "FS",
            Self::CombatDamage => "CD",
            Self::CombatEnd => "EC",
            Self::End => "END",
            Self::Cleanup => "CL",
        }
    }

    /// Whether this step belongs to the combat phase — clients use it to
    /// decide when to show the combat lane and attack arrows.
    #[must_use]
    pub const fn is_combat(self) -> bool {
        matches!(
            self,
            Self::CombatBegin
                | Self::DeclareAttackers
                | Self::DeclareBlockers
                | Self::CombatDamageFirst
                | Self::CombatDamage
                | Self::CombatEnd
        )
    }
}
