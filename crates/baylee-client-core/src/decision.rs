//! The decision actor and the owner of the resources are separate identities.
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::{HandObject, PlayerView, PublicObject, TargetRef};

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

/// Whether `object` on the stack is a thing `me` has to answer: the other
/// side's, or an ability of this side's that someone else aimed at `me`.
///
/// The second half is [`PlayerView::foreign_targets`]. Confusion in the Ranks
/// is controlled by its own player and aimed by whoever's permanent entered,
/// so reading the controller alone passed the window in which that player
/// could have answered the exchange of their own creature (Argon v Astra,
/// three times in one game).
#[must_use]
pub fn opposes(
    view: &PlayerView,
    object: &PublicObject,
    me: PlayerId,
    same_side: impl Fn(PlayerId, PlayerId) -> bool,
) -> bool {
    !same_side(object.controller, me)
        || (view.foreign_targets.contains(&object.id)
            && object.targets.iter().any(|target| match target {
                TargetRef::Player(player) => *player == me,
                TargetRef::Object(source) => view
                    .object(source.object)
                    .is_some_and(|aimed| aimed.controller == me),
            }))
}

/// All visible hands, for identification and previews; this grants no control.
pub fn known_cards(view: &PlayerView) -> impl Iterator<Item = &HandObject> {
    view.hand
        .iter()
        .chain(view.controlled_hands.iter().flat_map(|h| &h.cards))
        .chain(view.shared_hands.iter().flat_map(|h| &h.cards))
}

/// The one player who owns every card of `cards` the view shows, which is
/// whose library a question about them means: a card goes only into its
/// owner's library or graveyard (CR 400.3), so the cards an arrangement puts
/// back, or a search or a put-back offers, name the library they go into.
///
/// Architects of Will looks at the top of *target player's* library and puts
/// the cards back there; the question carries no owner of its own, and a
/// line that said "your library" was wrong whenever the target was someone
/// else. Read off what the seat is already shown (`looking_at`, a zone, a
/// hand), so it tells nothing new.
///
/// `None` when the view shows none of the cards (a trigger's handle, a
/// search that found nothing), or when they are more than one player's.
#[must_use]
pub fn cards_owner(view: &PlayerView, cards: &[ObjectId]) -> Option<PlayerId> {
    let mut owners = cards.iter().filter_map(|&id| shown_owner(view, id));
    let first = owners.next()?;
    owners.all(|owner| owner == first).then_some(first)
}

/// Who owns `id`, as the view shows it: an object's owner, or the player
/// whose hand it is in (a card in a hand is its owner's, CR 400.3).
fn shown_owner(view: &PlayerView, id: ObjectId) -> Option<PlayerId> {
    if let Some(object) = view.object(id) {
        return Some(object.owner);
    }
    if view.hand.iter().any(|card| card.id == id) {
        return Some(view.seat);
    }
    view.controlled_hands
        .iter()
        .chain(&view.shared_hands)
        .find(|hand| hand.cards.iter().any(|card| card.id == id))
        .map(|hand| hand.player)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_core::ids::{DamageSourceRef, ObjectId};

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

    /// The owner of the cards a question is about is read off what the view
    /// shows: a card looked at in another player's library is theirs, a
    /// card in this seat's hand its own, and a mixed or unseen set no one's.
    #[test]
    fn the_cards_owner_is_the_one_player_who_owns_every_card_shown() {
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let looked = vec![
            token(101, 1, "Mountain", 0, 0),
            token(82, 1, "Island", 0, 0),
        ];
        let view = ViewBuilder::new(2)
            .with_hand(vec![("Shock", 1, 7)])
            .with_looking_at(looked.clone())
            .build();
        let looked: Vec<ObjectId> = looked.iter().map(|o| o.id).collect();
        let held = view.hand[0].id;
        let unseen = ObjectId::new(400, 0);
        assert_eq!(cards_owner(&view, &looked), Some(them));
        assert_eq!(cards_owner(&view, &[held]), Some(me));
        assert_eq!(cards_owner(&view, &[unseen, looked[0]]), Some(them));
        assert_eq!(cards_owner(&view, &[held, looked[0]]), None, "two owners");
        assert_eq!(cards_owner(&view, &[unseen]), None, "nothing shown");
        assert_eq!(cards_owner(&view, &[]), None);
    }

    fn aimed_at(object: ObjectId) -> TargetRef {
        TargetRef::Object(DamageSourceRef { object, version: 0 })
    }

    /// The stack is the other side's, or it is this side's and someone else
    /// aimed it at this seat: a player, a permanent, not a permanent of
    /// somebody else's.
    #[test]
    fn who_aimed_it_counts_as_well_as_who_controls_it() {
        let mut view = ViewBuilder::new(2).build();
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let same = |a: PlayerId, b: PlayerId| a == b;
        let mine = token(1, 0, "Hill Giant", 3, 3);
        let theirs = token(2, 1, "Ondu Cleric", 1, 1);
        let (my_creature, their_creature) = (mine.id, theirs.id);
        view.battlefield = vec![mine, theirs];

        let mut ability = token(40, 0, "Confusion in the Ranks", 0, 0);
        ability.targets = vec![aimed_at(my_creature)];
        assert!(!opposes(&view, &ability, me, same), "this seat aimed it");
        assert!(opposes(&view, &ability, them, same), "the other side's");

        view.foreign_targets = vec![ability.id];
        assert!(opposes(&view, &ability, me, same));
        ability.targets = vec![TargetRef::Player(me)];
        assert!(opposes(&view, &ability, me, same));
        ability.targets = vec![aimed_at(their_creature)];
        assert!(!opposes(&view, &ability, me, same));
    }
}
