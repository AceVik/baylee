//! `cards/instants/mv_2/burnout.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn burnout() -> CardIndex {
    card_index("90865918-2f9a-4ab6-a881-e86f61a527cc")
}

/// `Burnout` ({1}{R}, Instant) counters target instant spell if it's blue, and
/// draws a card at the beginning of the next turn's upkeep either way.
#[test]
fn burnout_counters_blue_instant_leaves_nonblue_instant_alone_and_draws_next_upkeep() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[island(), island(), mountain()])
        .hand(0, &[burnout(), burnout()])
        .hand(1, &[lightning_bolt(), counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    // P1 casts Lightning Bolt targeting P0; P1 also casts Counterspell targeting Lightning Bolt.
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();

    cast_with_floating(&mut engine, p1, counterspell());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![bolt],
                players: vec![],
            },
        )
        .unwrap();
    let cspell = on_stack(&engine, counterspell()).unwrap();

    // P0 casts first Burnout targeting Counterspell (blue instant) and second Burnout targeting Lightning Bolt (red instant).
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, burnout());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![cspell],
                players: vec![],
            },
        )
        .unwrap();
    cast_with_floating(&mut engine, p0, burnout());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bolt],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    // Counterspell was countered by the first Burnout; Lightning Bolt was NOT countered by the second Burnout and dealt 3 damage to P0.
    assert_eq!(engine.state().players[0].life, 17);

    // At the beginning of the next turn's upkeep (P0's upkeep), both Burnout delayed triggers draw a card (2 total).
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );

    // It is one upkeep only: the opponent's next upkeep draws P0 nothing more.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    let hand_at_their_upkeep = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_at_their_upkeep
    );
}

/// "Counter target instant spell": a creature spell is not a legal target, so Burnout cannot be
/// cast with only a creature spell on the stack.
#[test]
fn burnout_cannot_target_a_creature_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[burnout()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);

    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, grizzly_bears());
    assert!(on_stack(&engine, grizzly_bears()).is_some());

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, burnout()).expect("Burnout is in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "no instant spell on the stack, so Burnout has no target"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, grizzly_bears()).is_some());
}
