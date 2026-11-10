//! `cards/creatures/mv_5/arrogant_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

fn arrogant_wurm() -> CardIndex {
    card_index("dc7f0c33-80ea-453e-a237-473f077f038e")
}

/// The ordinary {3}{G}{G} cast produces the printed 4/4 trampler and spends
/// exactly the five mana floated by the fixture.
#[test]
fn arrogant_wurm_casts_for_five_as_a_four_four_trampler() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[arrogant_wurm()])
        .battlefield(0, &[forest(); 5])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    cast_with_floating(&mut engine, p0, arrogant_wurm());
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, stack_is_empty);

    let wurm = on_battlefield(&engine, p0, arrogant_wurm()).unwrap();
    assert_eq!(pt(&engine, wurm), (4, 4));
    assert!(keywords(&engine, wurm).contains(KeywordSet::TRAMPLE));
    assert!(in_graveyard(&engine, p0, arrogant_wurm()).is_none());
}

/// Madness {2}{G} casts the exact discarded incarnation during an opponent's
/// turn. Prying Eyes supplies a real discard and leaves exactly three mana.
#[test]
fn arrogant_wurm_madness_casts_the_exiled_card_for_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[arrogant_wurm(), index::PRYING_EYES])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 9);

    cast_with_floating(&mut engine, p0, index::PRYING_EYES);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseCards { player, min: 2, max: 2, .. } if *player == p0),
    );
    let wurm = in_hand(&engine, p0, arrogant_wurm()).unwrap();
    let other = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|card| *card != wurm)
        .expect("Prying Eyes drew another card to discard");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![wurm, other],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::Madness { card },
                ..
            } if *player == p0 && *card == wurm
        )
    });
    assert_eq!(engine.state().object(wurm).unwrap().zone, Zone::Exile);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, arrogant_wurm()).is_some() && stack_is_empty(e)
    });

    assert_eq!(engine.state().turn.active, p1, "madness ignored timing");
    assert_eq!(
        on_battlefield(&engine, p0, arrogant_wurm()),
        Some(wurm),
        "the discarded incarnation entered under its owner's control"
    );
    assert_eq!(pt(&engine, wurm), (4, 4));
    assert!(keywords(&engine, wurm).contains(KeywordSet::TRAMPLE));
    assert!(mine(&engine, p0, arrogant_wurm(), Zone::Graveyard).is_empty());
    assert!(mine(&engine, p0, arrogant_wurm(), Zone::Exile).is_empty());
}

/// Trample assigns lethal damage to the 2/2 blocker and the remaining two to
/// the defending player. The 4/4 survives the two damage dealt back.
#[test]
fn arrogant_wurm_tramples_two_damage_over_a_grizzly_bears() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arrogant_wurm()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wurm = on_battlefield(&engine, p0, arrogant_wurm()).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    let blocks = attack_and_collect_blocks(&mut engine, wurm, p1);
    assert!(blocks.iter().any(|block| block.blocker == bears));
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, wurm)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && stack_is_empty(e)
    });

    assert!(on_battlefield(&engine, p1, index::GRIZZLY_BEARS).is_none());
    assert_eq!(
        engine.state().players[1].life,
        18,
        "two damage tramples through"
    );
    assert_eq!(engine.state().object(wurm).unwrap().damage, 2);
    assert!(on_battlefield(&engine, p0, arrogant_wurm()).is_some());
}
