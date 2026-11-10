//! `cards/instants/mv_2/shadowbane.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn shadowbane() -> CardIndex {
    card_index("9e9ec33b-b923-40e5-81a8-e1e78df63ec7")
}

/// Shadowbane (`{1}{W}`, `Coverage::Partial`): prevents the next damage a
/// chosen source would deal to you this turn.
#[test]
fn shadowbane_prevents_next_damage_from_chosen_source() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[mountain()])
        .hand(0, &[shadowbane()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
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

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shadowbane());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, p0);
    let source = options.into_iter().find(|r| r.object == bolt).unwrap();
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20);
}

/// "A source of your choice": only that source is shielded. Two Bolts are on
/// the stack and the shield names the lower one, so the upper Bolt still hits
/// and the shield is still waiting for the chosen one.
#[test]
fn shadowbane_prevents_only_the_chosen_source_and_lets_another_source_hit() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[mountain(), mountain()])
        .hand(0, &[shadowbane()])
        .hand(1, &[lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
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
    let first_bolt = on_stack(&engine, lightning_bolt()).unwrap();
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

    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, shadowbane());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        player,
        choice,
        options,
        ..
    } = engine.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(player, p0);
    let source = options
        .into_iter()
        .find(|r| r.object == first_bolt)
        .expect("the lower Bolt is offered as a damage source");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();

    // The upper Bolt resolves first and is not the chosen source.
    pass_until(&mut engine, |e| e.state().players[0].life < 20);
    assert_eq!(
        engine.state().players[0].life,
        17,
        "the Bolt that was not chosen deals its 3 damage"
    );
    assert!(
        on_stack(&engine, lightning_bolt()).is_some(),
        "the chosen Bolt is still waiting, and so is the shield"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        17,
        "the chosen Bolt's damage is prevented"
    );
}
