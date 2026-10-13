//! What an open question leaves out, which the table dims (`Interaction::leaves_out`): only this seat's own question, only a question that picks among objects, and only what it neither offers nor already holds. Pinned here: targets and enumerated choices leave out everything not on their list, a hand choice with no list and a question that takes no objects leave out nothing, an attack and a block leave out what is no part of the declaration, and another seat's question dims nothing.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// A pair of objects the question offers and a third it does not.
const OFFERED: u32 = 1;
const ALSO_OFFERED: u32 = 2;
const STRANGER: u32 = 9;

/// A target prompt over objects 1 and 2.
fn targets(min: u32, max: u32) -> Interaction {
    interaction(target_choice(
        vec![obj(OFFERED), obj(ALSO_OFFERED)],
        vec![],
        min,
        max,
    ))
}

/// A target prompt offering one object, addressed to `player` but read by this seat.
fn targets_for(player: PlayerId) -> Interaction {
    Interaction::new(
        Pending::ChooseTargets {
            player,
            options: vec![obj(OFFERED)],
            player_options: vec![],
            min: 1,
            max: 1,
            reason: TargetPrompt::Targets,
        },
        me(),
    )
}

/// A target prompt leaves out every permanent it does not offer, and only those.
#[test]
fn a_target_prompt_leaves_out_what_it_does_not_offer() {
    let i = targets(1, 1);
    assert!(!i.leaves_out(obj(OFFERED)));
    assert!(!i.leaves_out(obj(ALSO_OFFERED)));
    assert!(i.leaves_out(obj(STRANGER)));
}

/// An enumerated choice of cards dims what it did not list, as a target prompt does.
#[test]
fn an_enumerated_card_choice_leaves_out_what_it_does_not_list() {
    let i = interaction(Pending::ChooseCards {
        player: me(),
        options: vec![obj(OFFERED)],
        min: 1,
        max: 1,
        prompt: ChoicePrompt::Generic,
        total: None,
    });
    assert!(!i.leaves_out(obj(OFFERED)));
    assert!(i.leaves_out(obj(STRANGER)));
}

/// A question that takes no objects (`max` 0) leaves nothing out, even with a list.
#[test]
fn a_question_that_takes_no_object_leaves_nothing_out() {
    let i = targets(0, 0);
    assert!(!i.leaves_out(obj(STRANGER)));
    assert!(!i.leaves_out(obj(OFFERED)));
}

/// A discard names no list, so any card in hand is fair game and none is left out.
#[test]
fn a_hand_choice_with_no_list_leaves_nothing_out() {
    let i = interaction(Pending::DiscardChoice {
        player: me(),
        count: 1,
    });
    assert!(!i.leaves_out(obj(STRANGER)));
}

/// An object already picked is never dimmed, whatever the list says.
#[test]
fn an_object_already_picked_is_not_left_out() {
    let mut i = targets(1, 2);
    assert_eq!(i.toggle(obj(OFFERED)), SelectionOutcome::Added);
    assert!(!i.leaves_out(obj(OFFERED)));
    assert!(i.leaves_out(obj(STRANGER)));
}

/// A question addressed to another seat dims nothing on this seat's table.
#[test]
fn another_seats_question_leaves_nothing_out() {
    let i = targets_for(PlayerId::new(1));
    assert!(!i.is_mine());
    assert!(!i.leaves_out(obj(STRANGER)));
    assert!(!i.leaves_out(obj(OFFERED)));
}

/// An attack leaves out what can neither attack nor be attacked: the candidates and a planeswalker defender stay lit.
#[test]
fn an_attack_leaves_out_all_but_attackers_and_planeswalkers_under_attack() {
    let walker = Defender::Planeswalker(obj(50));
    let i = interaction(attack_choice(vec![obj(1), obj(2)], vec![seat(1), walker]));
    assert!(!i.leaves_out(obj(1)), "an attacker candidate");
    assert!(!i.leaves_out(obj(2)), "an attacker candidate");
    assert!(
        !i.leaves_out(obj(50)),
        "a planeswalker that may be attacked"
    );
    assert!(i.leaves_out(obj(STRANGER)), "a permanent in neither role");
}

/// A declared attacker stays lit as well as a candidate.
#[test]
fn a_declared_attacker_is_not_left_out() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    assert!(i.declare_attacker(obj(1), seat(1)));
    assert!(!i.leaves_out(obj(1)));
    assert!(i.leaves_out(obj(STRANGER)));
}

/// A block leaves out whatever is neither a blocker nor an attacker; every blocker candidate stays lit, even one that can block only an attacker the aim is not on.
#[test]
fn a_block_leaves_out_all_but_blockers_and_attackers() {
    let i = interaction(block_choice(vec![
        BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1)],
        },
        BlockOption {
            blocker: obj(11),
            attackers: vec![obj(2)],
        },
    ]));
    assert!(!i.leaves_out(obj(10)), "a blocker");
    assert!(
        !i.leaves_out(obj(11)),
        "a blocker for an attacker other than the first"
    );
    assert!(!i.leaves_out(obj(1)), "an attacker");
    assert!(!i.leaves_out(obj(2)), "the other attacker");
    assert!(i.leaves_out(obj(STRANGER)), "neither");
}

/// Questions that pick no object off the table leave nothing out: priority, a mulligan and an ordering.
#[test]
fn questions_that_pick_no_permanent_leave_nothing_out() {
    let mulligan = interaction(Pending::Mulligan {
        player: me(),
        taken: 0,
        next_is_free: true,
        can_take: true,
    });
    assert!(!mulligan.leaves_out(obj(STRANGER)));

    let ordering = interaction(put_back(vec![obj(1), obj(2)]));
    assert!(!ordering.leaves_out(obj(STRANGER)));
    assert!(!ordering.leaves_out(obj(1)));

    let priority = interaction(Pending::Priority {
        player: me(),
        legal: Box::default(),
    });
    assert!(!priority.leaves_out(obj(STRANGER)));
}
