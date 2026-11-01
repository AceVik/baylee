//! Windows and restrictions on playing a card exiled by an effect.
use baylee_core::types::TypeSet;

/// A permission follows the benefiting player's turns, including extra turns.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayWindow {
    /// Expires in this turn's cleanup step.
    ThisTurn,
    /// Expires as the benefiting player's next turn starts.
    UntilYourNextTurn,
    /// Expires as the benefiting player's next actual upkeep begins.
    UntilYourNextUpkeep,
    /// Expires in the cleanup step of that player's next turn.
    ThroughYourNextTurn,
}

/// How many library cards to exile and the permission granted for each.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ExilePlay {
    /// Top cards, or all remaining cards if the library contains fewer.
    pub count: u8,
    /// Lifetime independent of the source remaining on the battlefield.
    pub window: PlayWindow,
    /// Waive the mana cost, but still pay additional costs and increases.
    pub free: bool,
    /// Mana of any actual type may pay any mana symbol while casting this card.
    pub mana_any_type: bool,
    /// Restrict casting by the prospective spell's types; disallow land plays.
    /// `None` permits both casting spells and playing lands.
    pub spell_types: Option<TypeSet>,
}
