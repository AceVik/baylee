//! A combat decision's table, with its objects found by handle.

use crate::combat::Fighter;
use baylee_core::ids::{Defender, ObjectId};
use baylee_view::{PlayerView, PublicObject};

/// A [`PlayerView`] whose objects and declared attackers are indexed by
/// handle, built once for a decision that looks many of them up.
///
/// [`PlayerView::object`] walks every zone of the view until it meets the
/// handle, which is the right shape for a question asked once and the wrong
/// one for a combat decision, which asks it once per creature and, in a
/// `sort_by_key`, once per comparison. On self-play game r001 #431, where
/// tokens doubled to tens of thousands, about 90 % of the main thread's
/// samples were in it. This answers every handle the way that walk does —
/// where two zones name one handle, the zone the walk reaches first wins —
/// so a decision reads the same table either way; building it is one sort,
/// and each question a binary search.
///
/// The view is the protocol's and keeps its shape (`VIEW_VERSION`): the
/// index lives here, for as long as one decision does.
pub(crate) struct Board<'a> {
    pub(crate) view: &'a PlayerView,
    /// One per handle, in handle order.
    objects: Vec<&'a PublicObject>,
    /// What each creature in `view.combat.attackers` is aimed at, one per
    /// handle, in handle order.
    aims: Vec<(ObjectId, Defender)>,
}

impl<'a> Board<'a> {
    pub(crate) fn new(view: &'a PlayerView) -> Self {
        // `PlayerView::object`'s order, zone by zone. The sort is stable, so
        // of two objects under one handle the one that walk meets first is
        // still first, and `dedup_by_key` keeps the first.
        let mut objects: Vec<&PublicObject> = view
            .battlefield
            .iter()
            .chain(view.stack.iter())
            .chain(view.graveyards.iter().flatten())
            .chain(view.exile.iter().flatten())
            .chain(view.command.iter().flatten())
            .chain(view.looking_at.iter())
            .chain(view.library_tops.iter())
            .collect();
        objects.sort_by_key(|o| o.id);
        objects.dedup_by_key(|o| o.id);
        let mut aims: Vec<(ObjectId, Defender)> = view
            .combat
            .attackers
            .iter()
            .map(|a| (a.creature, a.defending))
            .collect();
        aims.sort_by_key(|(id, _)| *id);
        aims.dedup_by_key(|(id, _)| *id);
        Self {
            view,
            objects,
            aims,
        }
    }

    /// [`PlayerView::object`], by binary search.
    pub(crate) fn object(&self, id: ObjectId) -> Option<&'a PublicObject> {
        self.objects
            .binary_search_by_key(&id, |o| o.id)
            .ok()
            .map(|i| self.objects[i])
    }

    /// The creature under `id`, reduced to what combat reads; `None` when the
    /// view does not name it or cannot say its power and toughness.
    pub(crate) fn fighter(&self, id: ObjectId) -> Option<Fighter> {
        Fighter::from_object(self.object(id)?)
    }

    /// What `id` is declared attacking (CR 508.1b), when the view's combat
    /// names it as an attacker.
    pub(crate) fn aim(&self, id: ObjectId) -> Option<Defender> {
        self.aims
            .binary_search_by_key(&id, |(creature, _)| *creature)
            .ok()
            .map(|i| self.aims[i].1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::PlayerId;
    use baylee_view::AttackerView;

    fn id(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// Every handle the view names answers as `PlayerView::object` answers
    /// it, including the handles two zones share, whichever zone the walk
    /// reaches first; and a handle nobody names is `None` in both.
    #[test]
    fn the_index_answers_every_handle_as_the_walk_does() {
        let named = |slot: u32, name: &str| token(slot, 0, name, 1, 1);
        let mut view = ViewBuilder::new(2)
            .with_battlefield(1, [named(5, "on the table"), named(1, "first")])
            .with_stack(vec![named(9, "on the stack"), named(5, "stacked twin")])
            .with_graveyard(0, vec![named(3, "in a graveyard")])
            .with_exile(1, vec![named(3, "exiled twin"), named(7, "exiled")])
            .with_command(0, vec![named(8, "in command")])
            .with_looking_at(vec![named(1, "shown twin"), named(11, "shown")])
            .build();
        view.library_tops = vec![named(11, "top twin"), named(12, "on top")];
        let board = Board::new(&view);
        for slot in 0..14 {
            assert_eq!(
                board.object(id(slot)).map(|o| &o.name),
                view.object(id(slot)).map(|o| &o.name),
                "slot {slot}"
            );
        }
        assert_eq!(
            board.object(id(5)).map(|o| o.name.as_str()),
            Some("on the table"),
            "a handle on the table and on the stack answers as the table's"
        );
    }

    /// The first declaration of a creature is its aim, as `Iterator::find`
    /// over the view's attackers reads it.
    #[test]
    fn an_attacker_is_aimed_where_the_view_first_says() {
        let walker = Defender::Planeswalker(id(20));
        let player = Defender::Player(PlayerId::new(0));
        let view = ViewBuilder::new(2)
            .with_combat(
                vec![
                    AttackerView {
                        creature: id(4),
                        defending: walker,
                        blocked: false,
                    },
                    AttackerView {
                        creature: id(2),
                        defending: player,
                        blocked: false,
                    },
                    AttackerView {
                        creature: id(4),
                        defending: player,
                        blocked: false,
                    },
                ],
                vec![],
            )
            .build();
        let board = Board::new(&view);
        assert_eq!(board.aim(id(4)), Some(walker));
        assert_eq!(board.aim(id(2)), Some(player));
        assert_eq!(board.aim(id(3)), None);
    }
}
