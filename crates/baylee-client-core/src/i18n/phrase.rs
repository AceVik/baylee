use super::Phrase;

impl Phrase {
    /// What to say about a username the rule refused, one sentence per
    /// fault, so the player is told the thing to fix.
    #[must_use]
    pub fn username_fault(fault: baylee_protocol::names::UsernameFault) -> Self {
        use baylee_protocol::names::UsernameFault;
        match fault {
            UsernameFault::Invisible => Self::UsernameInvisible,
            UsernameFault::Character => Self::UsernameCharacters,
            UsernameFault::Length => Self::UsernameLength,
            UsernameFault::Edge => Self::UsernameEdge,
            UsernameFault::Doubled => Self::UsernameDoubled,
        }
    }
}
