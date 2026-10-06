//! `cards/artifacts/mv_4/birthing_pod.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Birthing Pod — {3}{G/P} artifact: "{1}{G/P}, {T}, Sacrifice a creature:
/// Search your library for a creature card with mana value equal to 1 plus
/// the sacrificed creature's mana value, put that card onto the battlefield,
/// then shuffle. Activate only as a sorcery."
///
/// The sacrificed creature's mana value is written on the ability as the
/// cost is paid, and read back when it resolves — by then the creature is a
/// card in the graveyard, which is why it cannot be asked then. Ornithopter
/// (0) is paid, and the library of Llanowar Elves (1) is offered whole. Two
/// Forests pay the `{G/P}` either way, so the seat is asked and pays green.
#[test]
fn birthing_pod_finds_a_creature_one_mana_value_above_the_sacrifice() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), forest(), birthing_pod(), ornithopter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    let library_before = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, birthing_pod(), 0);
    let Pending::YesNo { prompt, .. } = engine.pending().clone() else {
        panic!("{{G/P}} asks 2 life or green, got {:?}", engine.pending())
    };
    assert_eq!(prompt, crate::choice::YesNoPrompt::PayLife { amount: 2 });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    let Pending::ChooseCards {
        options, prompt, ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which creature, got {:?}", engine.pending())
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert_eq!(options, vec![thopter]);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![thopter],
            },
        )
        .unwrap();
    assert!(in_graveyard(&engine, p0, ornithopter()).is_some());

    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(options.len(), library_before, "0 + 1 is every Elves");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert_eq!(library_size(&engine, p0), library_before - 1);
    let pod = on_battlefield(&engine, p0, birthing_pod()).unwrap();
    assert!(is_tapped(&engine, pod));
    assert_eq!(engine.state().players[0].life, 20, "paid with green");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// "{G/P} can be paid with either {G} or 2 life." One Forest pays the `{1}`
/// and nothing is left for the green, so the `{G/P}` can only be 2 life: the
/// activation is offered, nobody is asked, and the life is paid with the
/// rest of the cost. Before activation costs read Phyrexian mana the Pod was
/// never offered off one Forest.
#[test]
fn birthing_pod_pays_its_phyrexian_green_with_two_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), birthing_pod(), ornithopter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, birthing_pod(), 0);
    let Pending::ChooseCards { prompt, .. } = engine.pending().clone() else {
        panic!(
            "only life can pay the {{G/P}}, so nothing is asked; got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![thopter],
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[0].life, 18, "2 life for the {{G/P}}");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the Forest's one"
    );
    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!("the helper returns only a card choice")
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
}

/// With green to spare the seat chooses, and 2 life is an answer: the
/// green stays in the pool. At 1 life with one Forest neither way pays,
/// and the Pod is not offered (CR 119.4).
#[test]
fn birthing_pod_asks_life_or_green_and_is_not_offered_when_neither_pays() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), forest(), birthing_pod(), ornithopter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, birthing_pod(), 0);
    assert!(matches!(engine.pending(), Pending::YesNo { .. }));
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let thopter = on_battlefield(&engine, p0, ornithopter()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![thopter],
            },
        )
        .unwrap();
    assert_eq!(engine.state().players[0].life, 18);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest paid the {{1}}, the other is still floating"
    );

    let mut broke = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), birthing_pod(), ornithopter()])
        .life(0, 1)
        .start();
    keep_mulligans(&mut broke);
    reach_main_phase(&mut broke, p0);
    tap_all_mana(&mut broke, p0);
    let Pending::Priority { legal, .. } = broke.pending() else {
        panic!("expected priority, got {:?}", broke.pending())
    };
    let pod = on_battlefield(&broke, p0, birthing_pod()).unwrap();
    assert!(
        !legal.abilities.contains(&(pod, 0)),
        "one mana and one life pay neither {{1}}{{G}} nor {{1}} and 2 life"
    );
}
