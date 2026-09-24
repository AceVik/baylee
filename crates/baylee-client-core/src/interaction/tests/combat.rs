//! `ChooseAttackers` and `ChooseBlockers`, the two choices that are a *pairing* rather than a selection: a declaration is a creature plus the thing it is pointed at, and the focus supplies the second half so that a tap on the table is enough. Pinned here: a tap declares rather than merely lighting a card up, re-declaring moves an attacker instead of duplicating it, a pairing the rules forbid is refused by the client instead of being sent and bounced, declaring nothing is a valid answer, and cancel forgets the declarations and the aim together. The generic selection list is not read in these two modes at all, so bounds, offers and `take_back` are in `bounds` and `picks`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

#[test]
fn declaring_attackers_checks_both_the_creature_and_the_defender() {
    let mut i = interaction(attack_choice(vec![obj(1), obj(2)], vec![seat(1)]));

    assert!(!i.declare_attacker(obj(9), seat(1)), "not a candidate");
    assert!(!i.declare_attacker(obj(1), seat(7)), "not a defender");
    assert!(i.declare_attacker(obj(1), seat(1)));

    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(1))]
        })
    );
}

#[test]
fn re_declaring_an_attacker_replaces_its_defender_rather_than_duplicating() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), seat(2)]));
    i.declare_attacker(obj(1), seat(1));
    i.declare_attacker(obj(1), seat(2));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(2))]
        })
    );
}

#[test]
fn declaring_no_attackers_is_a_valid_answer() {
    let i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    assert!(i.can_confirm());
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn blockers_must_block_an_actual_attacker() {
    let mut i = interaction(Pending::ChooseBlockers {
        player: me(),
        attacker: PlayerId::new(1),
        blockers: vec![BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1)],
        }],
    });
    assert!(!i.declare_blocker(obj(10), obj(99)));
    assert!(i.declare_blocker(obj(10), obj(1)));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareBlockers {
            blockers: vec![(obj(10), obj(1))]
        })
    );
}

// The bug this whole pairing model exists to close: tapping a creature in
// combat pushed it onto the generic selection list, which `confirm` never
// reads for these two modes. A player could light up their entire board
// and still declare no attackers — the client looked like it had combat
// and did not.
#[test]
fn tapping_a_creature_in_combat_actually_declares_it() {
    let mut i = interaction(attack_choice(vec![obj(1), obj(2)], vec![seat(1)]));
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Added);
    assert!(i.is_selected(obj(1)), "a declared attacker reads as chosen");
    assert_eq!(i.declared(), 1);
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(1))]
        })
    );
}

#[test]
fn tapping_a_declared_attacker_again_calls_it_off() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    i.toggle(obj(1));
    assert_eq!(i.toggle(obj(1)), SelectionOutcome::Removed);
    assert!(!i.is_selected(obj(1)));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn a_table_with_one_defender_needs_no_aiming_at_all() {
    // The two-player case has to cost nothing: one thing to attack, and
    // the focus already on it before the player touches anything.
    let i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    assert_eq!(i.combat_focus(), CombatFocus::Defender(seat(1)));
}

#[test]
fn attacks_go_where_the_focus_points_and_the_focus_can_be_moved() {
    let mut i = interaction(attack_choice(
        vec![obj(1), obj(2)],
        vec![seat(1), seat(2), Defender::Planeswalker(obj(50))],
    ));
    i.toggle(obj(1));
    assert_eq!(i.cycle_focus(1), Some(Pick::Seat(PlayerId::new(2))));
    i.toggle(obj(2));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), seat(1)), (obj(2), seat(2))]
        }),
        "two attackers, two different seats"
    );
    // And it wraps in both directions, so one key is enough to reach
    // every defender at a four-player table.
    assert_eq!(i.cycle_focus(-1), Some(Pick::Seat(PlayerId::new(1))));
    assert_eq!(
        i.cycle_focus(-1),
        Some(Pick::Object(obj(50))),
        "stepping back past the start wraps round"
    );
}

#[test]
fn tapping_a_planeswalker_aims_at_it() {
    let walker = Defender::Planeswalker(obj(50));
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), walker]));
    // A pointer should never have to find a cycle key: the thing being
    // attacked is on the table and can be tapped.
    assert_eq!(i.toggle(obj(50)), SelectionOutcome::Added);
    assert_eq!(i.combat_focus(), CombatFocus::Defender(walker));
    i.toggle(obj(1));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers {
            attackers: vec![(obj(1), walker)]
        })
    );
    assert_eq!(i.assignment(obj(1)), Some(CombatFocus::Defender(walker)));
}

#[test]
fn blocks_are_paired_with_the_attacker_in_focus() {
    let mut i = interaction(block_choice(vec![
        BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1), obj(2)],
        },
        BlockOption {
            blocker: obj(11),
            attackers: vec![obj(2)],
        },
    ]));
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
    i.toggle(obj(10));
    // Tap the second attacker to aim at it, then the blocker for it.
    assert_eq!(i.toggle(obj(2)), SelectionOutcome::Added);
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(2)));
    i.toggle(obj(11));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareBlockers {
            blockers: vec![(obj(10), obj(1)), (obj(11), obj(2))]
        })
    );
}

#[test]
fn a_block_the_rules_forbid_is_refused_rather_than_sent() {
    // Evasion is a pairing question — a flier is a legal blocker and
    // still not a legal block — so the client must not send it and wait
    // for the engine to bounce it.
    let mut i = interaction(block_choice(vec![
        BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1)],
        },
        BlockOption {
            blocker: obj(11),
            attackers: vec![obj(2)],
        },
    ]));
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
    assert_eq!(
        i.toggle(obj(11)),
        SelectionOutcome::Rejected,
        "obj(11) may only block obj(2)"
    );
    assert_eq!(i.declared(), 0);
    // The same creature against the attacker it *can* block goes through.
    i.cycle_focus(1);
    assert_eq!(i.toggle(obj(11)), SelectionOutcome::Added);
}

#[test]
fn calling_off_combat_forgets_the_declarations_and_the_aim() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1), seat(2)]));
    i.cycle_focus(1);
    i.toggle(obj(1));
    i.cancel();
    assert_eq!(i.declared(), 0);
    assert_eq!(i.combat_focus(), CombatFocus::Defender(seat(1)));
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn a_creature_that_cannot_attack_is_refused() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    assert_eq!(i.toggle(obj(99)), SelectionOutcome::Rejected);
    assert_eq!(i.declared(), 0);
}

#[test]
fn cancelling_clears_a_selection_and_any_declarations() {
    let mut i = interaction(attack_choice(vec![obj(1)], vec![seat(1)]));
    i.declare_attacker(obj(1), seat(1));
    i.cancel();
    assert_eq!(
        i.confirm(),
        Some(PlayerAction::DeclareAttackers { attackers: vec![] })
    );
}

#[test]
fn a_creature_that_cannot_block_the_aimed_attacker_is_not_offered() {
    // The one place aiming changes what is *offered*: `BlockOption` is
    // per blocker, so a flier in the focus leaves the ground with nothing
    // to answer. Lighting it anyway invites a click `toggle` then refuses.
    let mut i = interaction(block_choice(vec![
        BlockOption {
            blocker: obj(10),
            attackers: vec![obj(1)],
        },
        BlockOption {
            blocker: obj(11),
            attackers: vec![obj(2)],
        },
    ]));
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
    assert!(i.is_selectable(obj(10)));
    assert!(
        !i.is_selectable(obj(11)),
        "obj(11) may only block obj(2), which is not what is aimed at"
    );
    assert!(i.is_selectable(obj(2)), "an attacker is always aimable");
    i.cycle_focus(1);
    assert!(i.is_selectable(obj(11)));
    assert!(!i.is_selectable(obj(10)));
}

// Twelve Soldiers are one card until some of them are declared, and a
// declaration is what splits them (`board::Proposal`). So a click on the
// undeclared card sends one more, a click on the declared card takes one
// back, and neither is ever the same Soldier twice.
#[test]
fn a_stack_of_attackers_is_a_pool_a_click_draws_from() {
    let soldiers: Vec<ObjectId> = (1..=5).map(obj).collect();
    let mut i = interaction(attack_choice(soldiers.clone(), vec![seat(1)]));

    assert_eq!(i.toggle_group(&soldiers), SelectionOutcome::Added);
    assert_eq!(i.toggle_group(&soldiers), SelectionOutcome::Added);
    assert_eq!(
        i.assignments(),
        vec![
            (obj(2), CombatFocus::Defender(seat(1))),
            (obj(3), CombatFocus::Defender(seat(1))),
        ],
        "two clicks, two Soldiers, and the card drawn for the stack stays home"
    );

    // The declared two are a card of their own now, drawn as the first of
    // them; a click there takes the newest back and leaves that one.
    let declared = [obj(2), obj(3)];
    assert_eq!(i.toggle_group(&declared), SelectionOutcome::Removed);
    assert!(i.is_selected(obj(2)) && !i.is_selected(obj(3)));

    // And the whole card in one gesture, each paired with the focus.
    let home = [obj(1), obj(3), obj(4), obj(5)];
    assert_eq!(i.toggle_all(&home), SelectionOutcome::Added);
    assert_eq!(i.declared(), 5);
    assert_eq!(i.toggle_all(&soldiers), SelectionOutcome::Removed);
    assert_eq!(i.declared(), 0);
}

// Blocking draws from a stack the same way, and stops where the rules do: a
// whole card of blockers put in front of an attacker only some of them may
// block is cut at the first that may not, not sent for the engine to bounce.
#[test]
fn a_stack_of_blockers_fills_in_front_of_the_focus_as_far_as_it_may() {
    let wall = |blocker: u32, attackers: Vec<ObjectId>| BlockOption {
        blocker: obj(blocker),
        attackers,
    };
    // Eleven can block only the other attacker — a flier in the focus, say.
    let mut i = interaction(block_choice(vec![
        wall(12, vec![obj(1)]),
        wall(13, vec![obj(1)]),
        wall(11, vec![obj(2)]),
    ]));
    assert_eq!(i.combat_focus(), CombatFocus::Attacker(obj(1)));
    let blockers = [obj(11), obj(12), obj(13)];
    assert_eq!(i.toggle_group(&blockers), SelectionOutcome::Added);
    assert_eq!(
        i.assignments(),
        vec![(obj(12), CombatFocus::Attacker(obj(1)))]
    );
    assert_eq!(i.toggle_all(&blockers), SelectionOutcome::Added);
    assert_eq!(
        i.assignments(),
        vec![
            (obj(12), CombatFocus::Attacker(obj(1))),
            (obj(13), CombatFocus::Attacker(obj(1))),
        ],
        "the one that may not block the focus was left out, not sent"
    );
}
