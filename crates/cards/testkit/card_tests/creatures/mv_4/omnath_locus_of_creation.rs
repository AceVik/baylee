//! `cards/creatures/mv_4/omnath_locus_of_creation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// "Landfall — Whenever a land you control enters, you gain 4 life if this
/// is the first time this ability has resolved this turn. If it's the second
/// time, add {R}{G}{W}{U}. If it's the third time, Omnath deals 4 damage to
/// each opponent and each planeswalker you don't control."
///
/// Three lands in one turn, the way the deck makes them: Arid Mesa played
/// (4 life), cracked for a Plains (the four mana), and Crop Rotation paid
/// from that mana for another (4 damage to the opponent and to their Oko,
/// which the opponent had raised to 6 loyalty).
#[allow(clippy::too_many_lines)] // two turns of setup, then three landfalls
#[test]
fn omnath_gains_then_adds_then_deals_damage_as_its_landfall_resolves() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[omnath_locus_of_creation(), forest()])
        .hand(0, &[arid_mesa(), crop_rotation()])
        .battlefield(1, &[forest(), island(), forest()])
        .hand(1, &[oko_thief_of_crowns()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, oko_thief_of_crowns());
    pass_until(&mut engine, stack_is_empty);
    let oko = on_battlefield(&engine, p1, oko_thief_of_crowns()).unwrap();
    activate(&mut engine, p1, oko_thief_of_crowns(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters_on(&engine, oko, CounterKind::Loyalty), 6);
    reach_their_main_phase(&mut engine, p0);
    let life =
        |e: &Engine<RegistryLookup>, seat: PlayerId| e.state().players[seat.get() as usize].life;
    let (mine, theirs) = (life(&engine, p0), life(&engine, p1));

    // The first time: 4 life.
    let mesa = in_hand(&engine, p0, arid_mesa()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: mesa })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), mine + 4, "the first resolution");
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    // The second time: {R}{G}{W}{U}. The Mesa costs 1 life.
    let mesa = on_battlefield(&engine, p0, arid_mesa()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mesa,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!()
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
    assert_eq!(life(&engine, p0), mine + 3, "no second 4 life");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 4, "the second resolution");
    for color in [
        ManaColor::Red,
        ManaColor::Green,
        ManaColor::White,
        ManaColor::Blue,
    ] {
        assert_eq!(pool.available(color), 1, "{color:?}");
    }
    assert_eq!(life(&engine, p1), theirs);

    // The third time: 4 damage to the opponent and to their planeswalker.
    cast_with_floating(&mut engine, p0, crop_rotation());
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the additional cost, got {:?}", engine.pending())
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!()
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
    assert_eq!(life(&engine, p1), theirs - 4, "each opponent");
    assert_eq!(
        counters_on(&engine, oko, CounterKind::Loyalty),
        2,
        "each planeswalker you don't control"
    );
    assert_eq!(life(&engine, p0), mine + 3, "not Omnath's controller");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{G}} paid for Crop Rotation and no more mana"
    );
}

/// A new turn is a new count: on the next turn the first land's landfall
/// gains 4 life again.
#[test]
fn omnath_counts_its_resolutions_afresh_each_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[omnath_locus_of_creation()])
        .hand(0, &[plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = |e: &Engine<RegistryLookup>| e.state().players[0].life;
    for turn in 0..2 {
        let before = life(&engine);
        let land = in_hand(&engine, p0, plains()).unwrap();
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(life(&engine), before + 4, "turn {turn}: the first time");
        pass_until(&mut engine, |e| e.state().turn.active != p0);
        reach_their_main_phase(&mut engine, p0);
    }
}

/// CR 400.7: Omnath exiled and returned by Ephemerate is a new object, and
/// its landfall has resolved no times this turn. The Mesa's own landfall was
/// the old Omnath's first; the Plains the Mesa fetches is the new Omnath's
/// first too, so it gains 4 life again and adds no mana. The tally used to
/// be keyed by the object's id alone, which a blink does not change.
#[test]
fn omnath_blinked_counts_from_the_first_time_again() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[omnath_locus_of_creation(), plains()])
        .hand(0, &[arid_mesa(), ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let life = |e: &Engine<RegistryLookup>| e.state().players[0].life;
    let before = life(&engine);
    let mesa = in_hand(&engine, p0, arid_mesa()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: mesa })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine), before + 4);

    let omnath = on_battlefield(&engine, p0, omnath_locus_of_creation()).unwrap();
    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, ephemerate());
    let _ = aim_at(&mut engine, p0, omnath);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, omnath_locus_of_creation()).is_some());

    let mesa = on_battlefield(&engine, p0, arid_mesa()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mesa,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = pass_to_card_choice(&mut engine) else {
        unreachable!()
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
    assert_eq!(
        life(&engine),
        before + 4 - 1 + 4,
        "the new Omnath's first time: 4 life again"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and not the old Omnath's second time"
    );
}
