//! The piles beside a seat's mat, and what a hover spreads out of one. A pile is drawn from a count and a face and never from a list the viewer could read, which is where the library's two claims live: it is browsable at no depth at all (CR 401.2), and it fans backs because a `PlayerView` carries it as a count and hands the fan nothing to draw. Anything about `ZonePile` and `FannedCard` belongs here — how deep a fan goes and in which order, the blank slot a token leaves in it, which seat's pile a hovered card opens, and the faces that have to be resident before the hover, because a fan has no frame to spend on a fetch. Which slots a seat is drawn at all is `command_slots`, and a card on the battlefield is in no pile and is `lanes`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Nobody may look through a library, their own included (CR 401.2), so
/// the pile beside the mat is inert rather than merely empty — and it
/// stays inert with sixty cards in it, which is the case an "is it empty"
/// reading would get wrong.
#[test]
fn a_library_is_never_browsable_however_full_it_is() {
    let mut library = ZonePile::empty(PileKind::Library);
    library.count = 60;
    assert!(!library.is_browsable());

    let mut graveyard = ZonePile::empty(PileKind::Graveyard);
    assert!(!graveyard.is_browsable(), "an empty pile opens nothing");
    graveyard.count = 1;
    assert!(graveyard.is_browsable());
}

#[test]
fn courser_library_top_is_one_face_with_the_real_count_and_no_browsable_contents() {
    let mut view = ViewBuilder::new(3).build();
    view.seats[1].library_count = 51;
    let top = printed(120, 1, "Forest", 9);
    view.library_tops.push(top.clone());
    let board = model(&view);
    let pod = board
        .pods
        .iter()
        .find(|p| p.player == PlayerId::new(1))
        .unwrap();
    let pile = pod
        .piles
        .iter()
        .find(|p| p.kind == PileKind::Library)
        .unwrap();
    assert_eq!(pile.count, 51);
    assert_eq!(pile.top, Some(top.id));
    assert!(pile.art.is_some());
    assert!(pile.fan.is_empty(), "no lower cards can be fanned face up");
    assert!(!pile.is_browsable());
    view.library_tops.clear();
    let board = model(&view);
    let pile = board
        .pods
        .iter()
        .find(|p| p.player == PlayerId::new(1))
        .unwrap()
        .piles
        .iter()
        .find(|p| p.kind == PileKind::Library)
        .unwrap();
    assert_eq!(pile.count, 51);
    assert!(pile.top.is_none() && pile.art.is_none());
}
