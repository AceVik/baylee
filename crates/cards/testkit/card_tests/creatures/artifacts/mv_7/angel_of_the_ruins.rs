//! `cards/creatures/artifacts/mv_7/angel_of_the_ruins.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn angel_of_the_ruins() -> CardIndex {
    card_index("14878a5f-7983-4133-a1a7-9d539531e092")
}

#[test]
fn angel_of_the_ruins_exiles_up_to_two_artifacts_or_enchantments() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[angel_of_the_ruins()])
        .battlefield(1, &[sol_ring(), their_enchantment(), gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let artifact = on_battlefield(&e, p1, sol_ring()).expect("artifact target");
    let enchantment = on_battlefield(&e, p1, their_enchantment()).expect("enchantment target");
    let creature = on_battlefield(&e, p1, gray_ogre()).expect("non-target control");
    cast_from_hand(&mut e, p0, angel_of_the_ruins());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!("the arrival trigger targets")
    };
    assert_eq!((min, max), (0, 2), "up to two targets");
    assert!(options.contains(&artifact) && options.contains(&enchantment));
    assert!(!options.contains(&creature));
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![artifact, enchantment],
        },
    )
    .expect("both printed permanent types are legal targets");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(artifact).expect("artifact").zone,
        Zone::Exile
    );
    assert_eq!(
        e.state().object(enchantment).expect("enchantment").zone,
        Zone::Exile
    );
    let angel = on_battlefield(&e, p0, angel_of_the_ruins()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
}

#[test]
fn angel_of_the_ruins_plainscycles_from_hand_for_a_plains() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[angel_of_the_ruins()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    activate(&mut e, p0, angel_of_the_ruins(), 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options,
        prompt,
        min,
        max,
        ..
    } = e.pending().clone()
    else {
        unreachable!("plainscycling searches")
    };
    assert_eq!(prompt, ChoicePrompt::SearchLibrary);
    assert_eq!(
        (min, max),
        (0, 1),
        "a hidden-library search for a named quality may fail to find"
    );
    let found = options[0];
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![found],
        },
    )
    .expect("a Plains is a legal plainscycling search");
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, p0, angel_of_the_ruins()).is_some());
    assert_eq!(
        e.state().object(found).expect("found Plains").zone,
        Zone::Hand
    );
}
