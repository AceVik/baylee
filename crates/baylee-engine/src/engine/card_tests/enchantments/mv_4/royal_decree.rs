//! `cards/enchantments/mv_4/royal_decree.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;

fn royal_decree() -> CardIndex {
    card_index("fca14cbd-5ffc-441f-bccb-7ec22b03acbb")
}

/// The price the pending "pay?" question names, as a mana value.
fn price(engine: &Engine<RegistryLookup>) -> u32 {
    let Pending::YesNo {
        prompt: crate::choice::YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending()
    else {
        panic!("a payment question, got {:?}", engine.pending())
    };
    cost.cmc()
}

/// Royal Decree ({2}{W}{W} Enchantment):
/// - Cumulative upkeep {W}
/// - "Whenever a Swamp, Mountain, black permanent, or red permanent becomes tapped, this enchantment deals 1 damage to that permanent's controller."
#[test]
fn royal_decree_deals_one_damage_when_swamp_mountain_black_or_red_permanent_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[royal_decree(), plains()])
        .battlefield(1, &[swamp(), mountain(), forest()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(price(&engine), 1, "{{W}} for the one age counter");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    // Advance to P1's main phase.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });

    let p1_swamp = on_battlefield(&engine, p1, swamp()).unwrap();
    let p1_mtn = on_battlefield(&engine, p1, mountain()).unwrap();
    let p1_forest = on_battlefield(&engine, p1, forest()).unwrap();
    let life_before = life(&engine, p1);

    // Tapping a Forest does not trigger Royal Decree.
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_forest })
        .unwrap();
    assert!(stack_is_empty(&engine));
    assert_eq!(life(&engine, p1), life_before);

    // Tapping a Swamp triggers Royal Decree for 1 damage to P1.
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_swamp })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), life_before - 1);

    // Tapping a Mountain triggers Royal Decree for 1 damage to P1.
    engine
        .apply(p1, PlayerAction::ActivateManaAbility { source: p1_mtn })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p1), life_before - 2);
}

/// Royal Decree — "Cumulative upkeep {W}": one more {W} each of your
/// upkeeps, and the enchantment is sacrificed when the price is not paid.
#[test]
fn royal_decree_is_sacrificed_when_its_cumulative_upkeep_is_not_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[royal_decree(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    let decree = on_battlefield(&engine, p0, royal_decree()).unwrap();

    // Upkeeps one and two are paid: {W}, then {W}{W}, each with an age
    // counter more on the enchantment.
    for age in [1u16, 2] {
        pass_until(&mut engine, |e| {
            e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
        });
        assert_eq!(price(&engine), u32::from(age), "{{W}} per age counter");
        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        tap_all_mana(&mut engine, p0);
        engine.apply(p0, PlayerAction::PassPriority).unwrap();
        reach_main_phase(&mut engine, p0);
        let object = engine.state().object(decree).expect("still in play");
        assert_eq!(object.zone, Zone::Battlefield, "paid upkeep {age}");
        assert_eq!(
            object.counters.get(baylee_cards_dsl::counters::AGE),
            age,
            "age counters after upkeep {age}"
        );
        // Through the opponent's turn to P0's next upkeep.
        pass_until(&mut engine, |e| {
            e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
        });
    }

    // Upkeep three asks {W}{W}{W}, of which there are two: declined.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(price(&engine), 3, "{{W}} for each of three age counters");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, royal_decree()).is_some());
}
