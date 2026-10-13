//! `cards/enchantments/mv_2/search_for_azcanta.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn search_for_azcanta_surveils_at_upkeep_and_remains_on_face_zero() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(104, forest())
        .battlefield(0, &[search_for_azcanta(), island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });

    let Pending::Arrange {
        cards,
        prompt,
        piles,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected a surveil arrangement, got {:?}", engine.pending());
    };
    assert_eq!(prompt, ArrangePrompt::Surveil);
    assert_eq!(piles, surveil_piles(1), "surveil 1 allows at most 1 card");
    assert_eq!(cards.len(), 1, "surveil 1 looks at the top card of library");

    let milled_card = cards[0];
    engine
        .apply(p0, look_answer(&cards, &[milled_card]))
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let azcanta = on_battlefield(&engine, p0, search_for_azcanta())
        .expect("Search for Azcanta on battlefield");
    assert_eq!(
        engine.state().object(azcanta).map(|o| o.face_index),
        Some(0),
        "Search for Azcanta remains on face 0"
    );
    assert!(
        types(&engine, azcanta).contains(TypeSet::ENCHANTMENT),
        "Search for Azcanta is an enchantment"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        1,
        "the surveilled card was put into the graveyard"
    );
}

#[test]
fn azcanta_checks_seven_after_surveilling_then_offers_transform_and_filtered_search() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    for accept in [false, true] {
        let mut e = Duel::new(104, forest())
            .battlefield(0, &[search_for_azcanta(), island(), island(), island()])
            .hand(0, &[index::LIGHTNING_BOLT, index::GRIZZLY_BEARS])
            .start();
        keep_mulligans(&mut e);
        seed_graveyard(&mut e, p0, 6);
        pass_until(&mut e, |e| {
            matches!(
                e.pending(),
                Pending::Arrange {
                    prompt: ArrangePrompt::Surveil,
                    ..
                }
            )
        });
        let Pending::Arrange { cards, .. } = e.pending().clone() else {
            unreachable!()
        };
        e.apply(p0, look_answer(&cards, &cards)).unwrap();
        assert!(matches!(e.pending(), Pending::YesNo { .. }));
        e.apply(p0, PlayerAction::YesNo(accept)).unwrap();
        pass_until(&mut e, stack_is_empty);
        let azcanta = on_battlefield(&e, p0, search_for_azcanta()).unwrap();
        assert_eq!(
            e.state().object(azcanta).unwrap().face_index,
            u8::from(accept)
        );
        if !accept {
            continue;
        }
        reach_main_phase(&mut e, p0);
        let bolt = hand_to_library_top(&mut e, p0, index::LIGHTNING_BOLT);
        hand_to_library_top(&mut e, p0, index::GRIZZLY_BEARS);
        tap_mana_except(&mut e, p0, azcanta);
        activate(&mut e, p0, search_for_azcanta(), 1);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending()
        else {
            unreachable!()
        };
        assert_eq!(options, &vec![bolt]);
        assert_eq!((*min, *max), (0, 1));
        assert_eq!(e.inspected_cards().len(), 4);
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
        let Pending::Arrange { cards, .. } = e.pending().clone() else {
            panic!("choose bottom order")
        };
        e.apply(p0, PlayerAction::Arrange { piles: vec![cards] })
            .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(bolt).unwrap().zone, Zone::Hand);
    }
}

/// Azcanta, the Sunken Ruin: "{T}: Add {U}." One blue, no more, and the tap
/// is the cost.
#[test]
fn azcanta_the_sunken_ruin_taps_for_exactly_one_blue() {
    let p0 = PlayerId::new(0);
    let (mut engine, azcanta) =
        placed_face(search_for_azcanta(), 1, &[]).expect("Azcanta on the battlefield");
    assert!(!is_tapped(&engine, azcanta));
    activate(&mut engine, p0, search_for_azcanta(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "{{T}}: Add {{U}}");
    assert_eq!(pool.total(), 1, "and nothing else");
    assert!(is_tapped(&engine, azcanta), "{{T}} is part of the cost");
}

/// "Then if you have seven or more cards in your graveyard": with six, and
/// the surveilled card kept on top, there is no seventh card, so nothing is
/// asked and the enchantment stays as it is.
#[test]
fn search_for_azcanta_offers_no_transform_with_only_six_cards_in_the_graveyard() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(104, forest())
        .battlefield(0, &[search_for_azcanta(), island()])
        .start();
    keep_mulligans(&mut e);
    seed_graveyard(&mut e, p0, 6);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });
    let Pending::Arrange { cards, .. } = e.pending().clone() else {
        unreachable!()
    };
    e.apply(p0, look_answer(&cards, &[])).unwrap();
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        6,
        "the surveilled card stayed on top, so the graveyard is still six"
    );
    assert!(
        !matches!(e.pending(), Pending::YesNo { .. }),
        "six cards are not seven: no \"you may transform\" is asked, got {:?}",
        e.pending()
    );
    pass_until(&mut e, stack_is_empty);
    let azcanta = on_battlefield(&e, p0, search_for_azcanta()).unwrap();
    assert_eq!(
        e.state().object(azcanta).unwrap().face_index,
        0,
        "still Search for Azcanta"
    );
}
