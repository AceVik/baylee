//! `cards/enchantments/mv_3/growing_rites_of_itlimoc.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn growing_rites_of_itlimoc_transforms_at_four_creatures_and_taps_for_creature_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(
            0,
            &[
                growing_rites_of_itlimoc(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rites = on_battlefield(&engine, p0, growing_rites_of_itlimoc()).expect("the Rites");
    let rites_was = identity(&engine, rites);

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    pass_until(&mut engine, stack_is_empty);

    let itlimoc =
        on_battlefield(&engine, p0, growing_rites_of_itlimoc()).expect("Itlimoc on battlefield");
    assert_eq!(
        engine.state().object(itlimoc).map(|o| o.face_index),
        Some(1),
        "Growing Rites of Itlimoc transformed to face 1"
    );
    assert_eq!(
        identity(&engine, itlimoc),
        rites_was,
        "the same object, turned over: a transform changes no zone (CR 712.18)"
    );

    let t = types(&engine, itlimoc);
    assert!(t.contains(TypeSet::LAND), "Itlimoc is a land");
    assert!(
        !t.contains(TypeSet::ENCHANTMENT),
        "Itlimoc is not an enchantment"
    );

    activate(&mut engine, p0, growing_rites_of_itlimoc(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "Itlimoc added four green mana for the four creatures controlled"
    );
}

#[test]
fn growing_rites_reveals_only_a_creature_and_lets_you_order_the_rest() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(
            0,
            &[
                growing_rites_of_itlimoc(),
                index::GRIZZLY_BEARS,
                index::LIGHTNING_BOLT,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let bear = hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
    hand_to_library_top(&mut e, p0, index::LIGHTNING_BOLT);
    cast_from_hand(&mut e, p0, growing_rites_of_itlimoc());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        unreachable!()
    };
    assert_eq!(options, &vec![bear]);
    assert_eq!((*min, *max), (0, 1));
    let looked = e.inspected_cards().to_vec();
    assert_eq!(looked.len(), 4);
    e.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![bear],
        },
    )
    .unwrap();
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        panic!("the controller orders the other three")
    };
    assert_eq!(cards.len(), 3);
    let ordered: Vec<_> = cards.into_iter().rev().collect();
    e.apply(
        p0,
        PlayerAction::Arrange {
            piles: vec![ordered.clone()],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(bear).unwrap().zone, Zone::Hand);
    assert!(
        ordered
            .iter()
            .all(|id| e.state().zones.list(ZoneLocation::Library(p0))[..3].contains(id))
    );
}

/// Itlimoc, Cradle of the Sun: "{T}: Add {G}." Exactly one green however
/// many creatures are out; the second ability is the one that counts them.
#[test]
fn itlimoc_first_ability_taps_for_one_green_whatever_the_creature_count() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(106, forest())
        .battlefield(
            0,
            &[
                growing_rites_of_itlimoc(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    let rites = on_battlefield(&engine, p0, growing_rites_of_itlimoc()).unwrap();
    let def = baylee_cards::by_index(growing_rites_of_itlimoc()).unwrap();
    assert!(engine.dev_state_mut(p0).unwrap().transform(rites, def, 1));
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, rites));
    activate(&mut engine, p0, growing_rites_of_itlimoc(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "{{T}}: Add {{G}}, with four creatures out"
    );
    assert_eq!(pool.total(), 1, "and nothing else");
    assert!(is_tapped(&engine, rites), "{{T}} is part of the cost");
}

/// "if you control four or more creatures, transform": three are not four.
#[test]
fn growing_rites_stays_an_enchantment_with_only_three_creatures_at_the_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(106, forest())
        .battlefield(
            0,
            &[
                growing_rites_of_itlimoc(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| e.state().turn.active == p1);
    let rites = on_battlefield(&engine, p0, growing_rites_of_itlimoc()).expect("the Rites");
    assert_eq!(
        engine.state().object(rites).map(|o| o.face_index),
        Some(0),
        "three creatures at the end step: the Rites does not transform"
    );
    assert!(types(&engine, rites).contains(TypeSet::ENCHANTMENT));
}
