//! `cards/instants/mv_5/reflect_damage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn reflect_damage() -> CardIndex {
    card_index("21565b0d-f814-49ca-8613-f40040c4ba6c")
}

/// Reflect Damage (`{3}{R}{W}`, `Coverage::Partial`): chooses a damage source on
/// resolution and reflects the next damage it deals to you this turn back to
/// that source's controller.
#[test]
fn reflect_damage_reflects_next_damage_from_chosen_source_to_its_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), plains(), plains()])
        .hand(0, &[reflect_damage()])
        .battlefield(1, &[mountain()])
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
    cast_with_floating(&mut engine, p0, reflect_damage());
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
        panic!("expected ChooseDamageSource, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    let source = options
        .into_iter()
        .find(|r| r.object == bolt)
        .expect("Bolt offered as damage source");
    engine
        .apply(p0, PlayerAction::ChooseDamageSource { choice, source })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(engine.state().players[1].life, 17);
}

/// "A source of your choice": only that source's damage is reflected. Two Bolts
/// are on the stack and the choice names the lower one, so the upper Bolt hurts
/// only the caster. The reflection also keeps the original damage (the
/// Partial reason): the chosen Bolt hurts the caster and its controller.
#[test]
fn reflect_damage_reflects_only_the_chosen_source_and_not_the_other() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain(), mountain(), plains(), plains()])
        .hand(0, &[reflect_damage()])
        .battlefield(1, &[mountain(), mountain()])
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
    cast_with_floating(&mut engine, p0, reflect_damage());
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
        panic!("expected ChooseDamageSource, got {:?}", engine.pending());
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
    assert_eq!(engine.state().players[0].life, 17);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "damage from a source that was not chosen is not reflected"
    );
    assert!(
        on_stack(&engine, lightning_bolt()).is_some(),
        "the chosen Bolt is still waiting"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        14,
        "the chosen Bolt's damage is not redirected: the caster still takes it"
    );
    assert_eq!(
        engine.state().players[1].life,
        17,
        "the chosen Bolt's controller takes the same damage"
    );
}
