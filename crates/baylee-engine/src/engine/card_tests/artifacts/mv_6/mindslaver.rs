//! Mindslaver controls decisions and sight only during the target's next actual turn.

use super::*;

fn mindslaver() -> CardIndex {
    card_index("a806f4ee-f48a-46c3-8772-5aaabebe4c7e")
}

#[test]
fn mindslaver_waits_for_the_targets_next_turn_even_when_cast_on_their_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[mindslaver(), island(), island(), island(), island()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    reach_their_main_phase(&mut e, p1);
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, mindslaver(), 0);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p0, mindslaver()).is_some());
    assert!(!e.may_inspect_private(p0, p1), "not the current turn");

    reach_their_main_phase(&mut e, p0);
    assert!(
        !e.may_inspect_private(p0, p1),
        "not the controller's intervening turn"
    );
    reach_their_main_phase(&mut e, p1);
    assert!(
        e.may_inspect_private(p0, p1),
        "the target's next turn is controlled"
    );

    let spell = in_hand(&e, p1, gray_ogre()).unwrap();
    for source in all_on_battlefield(&e, p1, mountain()) {
        e.apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    e.apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the controller makes the target's casting decision");
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, p1, gray_ogre()).is_some(),
        "resources still belong to the target"
    );

    reach_their_main_phase(&mut e, p0);
    assert!(
        !e.may_inspect_private(p0, p1),
        "control ends with that turn"
    );
}
