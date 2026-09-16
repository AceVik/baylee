//! One ordered list of picks and the two ways a player builds it: a click on a candidate, and the aim keys that walk the offer and tick where they stopped. A seat is a pick like a permanent is (CR 115.4), so the offer, the aim and `take_back` run across objects and seats without a seam, and a counted stack of identical tokens takes one click per member instead of asking which one. Refusing a candidate the engine never offered is here; how many picks the question wants is in `bounds`, and an aim that points at a defender or an attacker is in `combat`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn only_offered_targets_can_be_selected() {
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1), obj(2)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    // Not in the offered set: the client refuses to even express it.
    assert_eq!(i.toggle(obj(99)), SelectionOutcome::Rejected);
    assert_eq!(i.selected().collect::<Vec<_>>(), vec![obj(1)]);
}

/// The keyboard's half of a list of cards: one key walks the offer, the
/// other acts where it stopped. Without the second there is no way to
/// answer a search from the keyboard at all — which is how the confirm
/// key came to be doing it, and how a `min: 0` search came to be
/// answered with "nothing" by a player who was passing priority.
#[test]
fn the_focus_keys_can_build_a_whole_answer_on_their_own() {
    let mut i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(1), obj(2), obj(3)],
        min: 0,
        max: 2,
        prompt: ChoicePrompt::Generic,
    });
    // The focus starts on the first option, so this needs no walk.
    assert_eq!(i.aim(), Some(Pick::Object(obj(1))));
    assert_eq!(i.toggle_focused(), SelectionOutcome::Added);
    assert_eq!(i.selected().collect::<Vec<_>>(), vec![obj(1)]);
    // And it takes back where it ticked, which is what makes a stray
    // press harmless: the same key on the same row undoes it.
    assert_eq!(i.toggle_focused(), SelectionOutcome::Removed);
    assert!(i.selected().next().is_none());
    // Walk, then tick: the pair reaches any row in the list.
    i.cycle_focus(2);
    assert_eq!(i.aim(), Some(Pick::Object(obj(3))));
    assert_eq!(i.toggle_focused(), SelectionOutcome::Added);
    assert_eq!(i.selected().collect::<Vec<_>>(), vec![obj(3)]);
}

/// Every mode that has no focus has nothing to tick, and says so rather
/// than reaching for whatever happens to be first.
#[test]
fn a_question_with_no_focus_ticks_nothing() {
    let mut i = interaction(Pending::Mulligan {
        player: me(),
        taken: 0,
        next_is_free: true,
    });
    assert_eq!(i.aim(), None);
    assert_eq!(i.toggle_focused(), SelectionOutcome::Rejected);
}

#[test]
fn a_face_is_a_target_like_any_other() {
    // "Any target" (CR 115.4) spans objects and players, so one prompt
    // has to be answerable with either — or with both, when it takes two.
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![PlayerId::new(0), PlayerId::new(1)],
        min: 2,
        max: 2,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert!(!i.can_confirm());
    assert_eq!(i.toggle_player(PlayerId::new(1)), SelectionOutcome::Added);
    assert_eq!(
        i.selected_players().collect::<Vec<_>>(),
        vec![PlayerId::new(1)]
    );
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseTargets {
            objects: vec![obj(1)],
            players: vec![PlayerId::new(1)],
        })
    );
}

#[test]
fn a_seat_the_spell_cannot_reach_is_refused() {
    // The tab is a camera control the rest of the time, so a rejection
    // here is what lets the click fall through to the camera.
    let mut i = interaction(Pending::ChooseTargets {
        player: me(),
        options: vec![obj(1)],
        player_options: vec![],
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    });
    assert_eq!(
        i.toggle_player(PlayerId::new(1)),
        SelectionOutcome::Rejected
    );
    i.toggle(obj(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::ChooseObjects {
            objects: vec![obj(1)]
        })
    );
}

// Forty Soldiers are one card on the table, and the question "which four
// of them" has no answer a player could mean differently: they are
// identical. So the stack takes clicks the way a card takes one.
#[test]
fn a_counted_stack_takes_one_click_per_member() {
    let members = [obj(1), obj(2), obj(3), obj(4)];
    let mut i = interaction(target_choice(members.to_vec(), vec![], 1, 2));
    assert_eq!(i.toggle_group(&members), SelectionOutcome::Added);
    assert_eq!(i.toggle_group(&members), SelectionOutcome::Added);
    assert_eq!(i.pick_count(), 2);
    assert_eq!(
        i.picks(),
        [Pick::Object(obj(1)), Pick::Object(obj(2))],
        "two clicks pick two different Soldiers, not the same one twice"
    );
    assert_eq!(
        i.toggle_group(&members),
        SelectionOutcome::Full,
        "a third pick past `max` is refused, not silently swapped in"
    );
    assert_eq!(i.pick_count(), 2);
}

#[test]
fn a_stack_with_nothing_left_to_pick_takes_the_last_one_back() {
    let members = [obj(1), obj(2)];
    let mut i = interaction(target_choice(members.to_vec(), vec![], 0, 4));
    i.toggle_group(&members);
    i.toggle_group(&members);
    assert_eq!(i.pick_count(), 2);
    // Every member is picked and `max` is not reached, so the click can
    // only mean "one fewer" — which on a stack of one is the toggle it
    // has always been.
    assert_eq!(i.toggle_group(&members), SelectionOutcome::Removed);
    assert_eq!(i.picks(), [Pick::Object(obj(1))]);
}

#[test]
fn the_aim_walks_the_offer_and_reaches_a_seat() {
    let mut i = interaction(target_choice(
        vec![obj(1), obj(2)],
        vec![PlayerId::new(1)],
        1,
        1,
    ));
    assert_eq!(i.aim(), Some(Pick::Object(obj(1))));
    assert_eq!(i.focus_position(), Some((0, 3)));
    assert_eq!(i.cycle_focus(1), Some(Pick::Object(obj(2))));
    assert_eq!(
        i.cycle_focus(1),
        Some(Pick::Seat(PlayerId::new(1))),
        "a face is a target like a permanent is (CR 115.4), so the aim \
         reaches it without a second key"
    );
    assert_eq!(
        i.cycle_focus(1),
        Some(Pick::Object(obj(1))),
        "and it wraps, so one key covers the whole offer"
    );
}

#[test]
fn a_pick_is_taken_back_one_at_a_time() {
    let mut i = interaction(target_choice(
        vec![obj(1), obj(2)],
        vec![PlayerId::new(1)],
        0,
        3,
    ));
    i.toggle(obj(1));
    i.toggle_player(PlayerId::new(1));
    i.toggle(obj(2));
    assert_eq!(i.take_back(), Some(Pick::Object(obj(2))));
    assert_eq!(
        i.take_back(),
        Some(Pick::Seat(PlayerId::new(1))),
        "objects and seats are one order, or `the last pick` means nothing"
    );
    assert_eq!(i.picks(), [Pick::Object(obj(1))]);
    assert_eq!(i.take_back(), Some(Pick::Object(obj(1))));
    assert_eq!(i.take_back(), None, "and it stops at empty");
}
