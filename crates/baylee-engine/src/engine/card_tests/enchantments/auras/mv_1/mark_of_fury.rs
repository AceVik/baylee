//! `cards/enchantments/auras/mv_1/mark_of_fury.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn mark_of_fury() -> CardIndex {
    card_index("1c350891-8810-4c14-b8ea-84486864637b")
}

/// Mark of Fury (1c350891-8810-4c14-b8ea-84486864637b) — "Enchant creature /
/// Enchanted creature has haste. / At the beginning of the end step, return
/// this Aura to its owner's hand."
#[test]
fn mark_of_fury_grants_haste_and_returns_to_owners_hand_at_end_step() {
    let mut e = lk::game(SEED, &[], &[], &[gray_ogre(), mark_of_fury()], &[]);
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, lk::P0, gray_ogre());
    lk::settle(&mut e);
    let ogre = lk::obj(&e, lk::P0, gray_ogre());
    assert!(!keywords(&e, ogre).contains(KeywordSet::HASTE));

    cast_with_floating(&mut e, lk::P0, mark_of_fury());
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .expect("Gray Ogre is a legal host");
    lk::settle(&mut e);
    assert!(keywords(&e, ogre).contains(KeywordSet::HASTE));

    // Summoning-sick Gray Ogre can now attack thanks to Haste.
    let _ = lk::attack(&mut e, &[ogre]);
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);
    assert_eq!(lk::life(&e, lk::P1), 18);

    // Passing through P0's end step returns Mark of Fury to P0's hand.
    reach_their_main_phase(&mut e, lk::P1);
    assert!(in_hand(&e, lk::P0, mark_of_fury()).is_some());
    assert!(on_battlefield(&e, lk::P0, mark_of_fury()).is_none());
}

/// Mark of Fury — "At the beginning of the end step" triggers on an opponent's
/// end step too, not only its controller's: if Mark of Fury's trigger on P0's
/// end step is countered by Tishana's Tidebinder so the Aura remains on the
/// battlefield into P1's turn, it triggers again at P1's end step and returns
/// to P0's hand.
#[test]
fn mark_of_fury_also_returns_at_opponents_end_step() {
    let mut e = lk::game(
        SEED,
        &[gray_ogre()],
        &[],
        &[mark_of_fury(), tishanas_tidebinder()],
        &[],
    );
    let ogre = lk::obj(&e, lk::P0, gray_ogre());
    lk::float(&mut e, lk::P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, lk::P0, mark_of_fury());
    e.apply(
        lk::P0,
        PlayerAction::ChooseObjects {
            objects: vec![ogre],
        },
    )
    .unwrap();
    lk::settle(&mut e);

    // Walk to P0's end step when Mark of Fury's return trigger is on the stack,
    // and counter that trigger with Tishana's Tidebinder.
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::End && !stack_is_empty(e)
    });
    let trigger_on_stack = e.state().zones.list(ZoneLocation::Stack)[0];
    lk::float(&mut e, lk::P0, &[(ManaColor::Blue, 3)]);
    cast_with_floating(&mut e, lk::P0, tishanas_tidebinder());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    lk::aim(&mut e, &[trigger_on_stack], &[]);
    lk::settle(&mut e);

    // Reach P1's main phase: Mark of Fury is still on the battlefield under P0's control.
    reach_their_main_phase(&mut e, lk::P1);
    assert!(on_battlefield(&e, lk::P0, mark_of_fury()).is_some());

    // Passing through P1's end step triggers Mark of Fury and returns it to P0's hand.
    reach_their_main_phase(&mut e, lk::P0);
    assert!(
        in_hand(&e, lk::P0, mark_of_fury()).is_some(),
        "returned to owner's hand at P1's end step"
    );
    assert!(on_battlefield(&e, lk::P0, mark_of_fury()).is_none());
}
