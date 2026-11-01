//! `cards/creatures/mv_4/yavimaya_ants.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn yavimaya_ants_attacks_with_haste_and_requires_cumulative_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index("3f5bbfaa-cbd7-47af-9aa2-abb49d672734");
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[card])
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);

    let ants = on_battlefield(&engine, p0, card).unwrap();

    // Haste lets Yavimaya Ants attack immediately for 5 combat damage.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ants, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 15);

    // On p0's next upkeep, decline cumulative upkeep {G}{G} -> Yavimaya Ants is sacrificed.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, card).is_none());
    assert!(in_graveyard(&engine, p0, card).is_some());
}

/// The price the pending "pay?" question names, as a mana value.
fn upkeep_price(engine: &Engine<RegistryLookup>) -> u32 {
    let Pending::YesNo {
        prompt: crate::choice::YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending()
    else {
        panic!("a payment question, got {:?}", engine.pending())
    };
    cost.cmc()
}

#[test]
fn yavimaya_ants_cumulative_upkeep_grows_by_gg_each_of_its_controllers_upkeeps() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("3f5bbfaa-cbd7-47af-9aa2-abb49d672734"),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let ants = on_battlefield(
        &engine,
        p0,
        card_index("3f5bbfaa-cbd7-47af-9aa2-abb49d672734"),
    )
    .unwrap();
    // Upkeeps one and two are paid: {G}{G}, then {G}{G}{G}{G}.
    for age in [1u16, 2] {
        pass_until(&mut engine, |e| {
            e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
        });
        assert_eq!(
            upkeep_price(&engine),
            2 * u32::from(age),
            "{{G}}{{G}} per age counter"
        );
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        tap_all_mana(&mut engine, p0);
        engine.apply(p0, PlayerAction::PassPriority).unwrap();
        reach_main_phase(&mut engine, p0);
        let object = engine.state().object(ants).expect("still in play");
        assert_eq!(object.zone, Zone::Battlefield, "paid upkeep {age}");
        assert_eq!(
            object.counters.get(baylee_cards_dsl::counters::AGE),
            age,
            "age counters after upkeep {age}"
        );
        // Through the opponent's turn (no question there: not their upkeep)
        // to p0's next upkeep.
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
        });
    }
    // Upkeep three asks for six mana, of which there are four: declined.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(upkeep_price(&engine), 6);
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(
            &engine,
            p0,
            card_index("3f5bbfaa-cbd7-47af-9aa2-abb49d672734")
        )
        .is_some()
    );
}
