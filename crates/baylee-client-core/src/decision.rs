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
    let player = resource_player(view);
    if player == view.seat {
        return &view.hand;
    }
    view.controlled_hands
        .iter()
        .find(|h| h.player == player)
        .map_or(&[], |h| h.cards.as_slice())
}

/// All hands explicitly entitled to this view, for identifying an offered card.
pub fn known_cards(view: &PlayerView) -> impl Iterator<Item = &HandObject> {
    view.hand
        .iter()
        .chain(view.controlled_hands.iter().flat_map(|h| &h.cards))
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
}
