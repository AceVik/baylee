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
