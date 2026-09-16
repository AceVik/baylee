//! Which zones a view has anything in, the order they are listed in - the viewing seat's own piles before anybody else's, which is what `BrowseZone`'s `Ord` encodes - and what ticking them does. Two ticks are one merged list that is still grouped by zone, no tick at all means every zone rather than none, and a tick dies with the pile it was standing on. The pin a question puts on a tab is not here: that is the question's doing and not the player's, and it is tested where the question is.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn a_pile_can_be_read_with_no_question_pending() {
    let view = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
        .build();
    let mut b = Browser::new();
    b.open_at(BrowseZone::Graveyard(PlayerId::new(1)));

    let rows = b.rows(&view, None, Names::projected());
    assert_eq!(rows.len(), 1, "the tab confines it to one pile");
    assert_eq!(rows[0].name, "Birds of Paradise");
    assert!(
        !rows[0].standing.selectable,
        "there is nothing to select for"
    );
    assert!(rows[0].place.is_none());

    b.show(None);
    assert_eq!(
        b.rows(&view, None, Names::projected()).len(),
        2,
        "both piles, unfiltered"
    );
}

/// Two boxes ticked is one list, and no box ticked is every list.
///
/// The owner asked for the checkbox on 14.09.2026 — *"so das man sie
/// durch das checken quasi mergen kann"* — and the merge is the whole of
/// what has to be asserted: that the rows really do span both piles, that
/// they stay **grouped by zone** while they do (which is what
/// [`BrowseZone`]'s `Ord` is for, and the one property a set could have
/// thrown away), and that unticking the last box lands on everything
/// rather than on nothing. That last one is the empty set's meaning, and
/// a panel that could be emptied by a second click on one chip is the bug
/// this is written against.
#[test]
fn ticking_two_zones_merges_their_lists_and_unticking_the_last_shows_all() {
    let view = ViewBuilder::new(2)
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
        .build();
    let names = Names::projected();
    let mut b = Browser::new();
    b.open();
    assert!(b.shows_every_zone(), "a fresh panel is on Alle");
    assert_eq!(b.rows(&view, None, names).len(), 3, "all three piles");

    b.tick(BrowseZone::Graveyard(me()));
    assert_eq!(ticks(&b), vec![BrowseZone::Graveyard(me())]);
    assert_eq!(b.rows(&view, None, names).len(), 1, "one pile alone");

    b.tick(BrowseZone::Stack);
    assert_eq!(
        ticks(&b),
        vec![BrowseZone::Stack, BrowseZone::Graveyard(me())],
        "and they are held in tab order, not in the order they were ticked"
    );
    let zones: Vec<BrowseZone> = b
        .rows(&view, None, names)
        .iter()
        .map(|row| row.zone)
        .collect();
    assert_eq!(
        zones,
        vec![BrowseZone::Stack, BrowseZone::Graveyard(me())],
        "the merged list is still grouped by zone"
    );
    assert!(
        !b.shows(BrowseZone::Graveyard(PlayerId::new(1))),
        "a pile nobody ticked is not in the merge"
    );

    b.tick(BrowseZone::Stack);
    b.tick(BrowseZone::Graveyard(me()));
    assert!(b.shows_every_zone(), "the last tick off is Alle again");
    assert_eq!(b.rows(&view, None, names).len(), 3);
}

/// A ticked pile that empties takes its tick with it.
///
/// Nothing draws a chip for a zone with nothing in it ([`zones_of`]), so a
/// tick left behind on one is a state the player can see the effect of —
/// an empty list — and not the cause. `follow` is where it goes, because
/// that is the door a view comes in through.
#[test]
fn a_tick_does_not_outlive_the_pile_it_is_on() {
    let full = ViewBuilder::new(2)
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .build();
    let mut b = Browser::new();
    b.open();
    b.tick(BrowseZone::Graveyard(me()));
    b.tick(BrowseZone::Stack);
    b.follow(&full, None);
    assert_eq!(ticks(&b).len(), 2, "both piles are still there");

    let emptied = ViewBuilder::new(2)
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .build();
    b.follow(&emptied, None);
    assert_eq!(
        ticks(&b),
        vec![BrowseZone::Stack],
        "the graveyard's tick went with the graveyard"
    );
}

#[test]
fn the_viewing_seats_own_piles_come_first() {
    let view = ViewBuilder::new(2)
        .with_stack(vec![printed(3, 1, "Counterspell", 2)])
        .with_graveyard(0, vec![printed(4, 0, "Llanowar Elves", 3)])
        .with_graveyard(1, vec![printed(5, 1, "Birds of Paradise", 4)])
        .with_looking_at(vec![printed(7, 0, "Ponder", 6)])
        .build();
    assert_eq!(
        Browser::new().zones(&view),
        vec![
            BrowseZone::Looking,
            BrowseZone::Stack,
            BrowseZone::Graveyard(PlayerId::new(0)),
            BrowseZone::Graveyard(PlayerId::new(1)),
        ],
        "shown cards, then the stack, then mine, then theirs"
    );
}
