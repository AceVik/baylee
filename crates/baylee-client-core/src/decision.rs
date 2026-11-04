//! The decision actor and the owner of the resources are separate identities.
use baylee_core::ids::PlayerId;
use baylee_view::{HandObject, PlayerView};

/// Resource owner only during an entitled local decision; otherwise the viewer.
#[must_use]
pub fn resource_player(view: &PlayerView) -> PlayerId {
    if view.awaiting == Some(view.seat) {
        view.decision_player.unwrap_or(view.seat)
    } else {
        view.seat
    }
}

/// Private hand currently operated by the local decision actor.
/// Missing control entitlement yields an empty hand, never another hand fallback.
#[must_use]
pub fn hand(view: &PlayerView) -> &[HandObject] {
    hand_of(view, resource_player(view))
}

/// `player`'s hand as this seat may play from it: its own, or the hand of a
/// player it controls (CR 723.4). Any other player's is empty, never a
/// fallback: a perspective shows only what the view carries.
#[must_use]
pub fn hand_of(view: &PlayerView, player: PlayerId) -> &[HandObject] {
    if player == view.seat {
        return &view.hand;
    }
    view.controlled_hands
        .iter()
        .find(|h| h.player == player)
        .map_or(&[], |h| h.cards.as_slice())
}

/// All visible hands, for identification and previews; this grants no control.
pub fn known_cards(view: &PlayerView) -> impl Iterator<Item = &HandObject> {
    view.hand
        .iter()
        .chain(view.controlled_hands.iter().flat_map(|h| &h.cards))
        .chain(view.shared_hands.iter().flat_map(|h| &h.cards))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ViewBuilder;
    #[test]
    fn a_foreign_decision_never_borrows_the_actors_resources() {
        let mut view = ViewBuilder::new(2).build();
        view.decision_player = Some(PlayerId::new(1));
        assert_eq!(resource_player(&view), PlayerId::new(1));
        assert!(hand(&view).is_empty());
        view.awaiting = Some(PlayerId::new(1));
        assert_eq!(resource_player(&view), view.seat);
    }

    /// A shown hand is one the view carries: this seat's, or a controlled
    /// player's; any other seat's is empty, never somebody else's.
    #[test]
    fn a_hand_is_shown_only_where_the_view_carries_it() {
        let view = ViewBuilder::new(3)
            .with_hand(vec![("Mine", 1, 10)])
            .controlling(1, vec![("Theirs", 1, 20)])
            .build();
        assert_eq!(hand_of(&view, view.seat)[0].name, "Mine");
        assert_eq!(hand_of(&view, PlayerId::new(1))[0].name, "Theirs");
        assert!(hand_of(&view, PlayerId::new(2)).is_empty());
    }
}
