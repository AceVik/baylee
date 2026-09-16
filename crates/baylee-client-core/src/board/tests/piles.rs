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
