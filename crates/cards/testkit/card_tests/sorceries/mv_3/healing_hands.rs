//! Oracle: target player gains 4 life; draw a card.
#[allow(clippy::wildcard_imports)]
use super::*;
fn healing_hands() -> CardIndex {
    card_index("3cc48835-3ac0-4774-b380-f9b21d2dc974")
}
/// Checklist: either player may be targeted; only chosen player gains four; caster draws one.
#[test]
fn gains_life_for_target_and_draws_for_caster() {
    let (caster, target_player) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[healing_hands(), plains()])
        .life(0, 15)
        .life(1, 12)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, caster);
    let library_before = library_size(&engine, caster);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(caster)).len();
    cast_from_hand(&mut engine, caster, healing_hands());
    let Pending::ChoosePlayer { options, .. } = engine.pending().clone() else {
        panic!("target player")
    };
    assert!(options.contains(&caster) && options.contains(&target_player));
    engine
        .apply(caster, PlayerAction::ChoosePlayer(target_player))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 15);
    assert_eq!(engine.state().players[1].life, 16);
    assert_eq!(library_size(&engine, caster), library_before - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(caster)).len(),
        hand_before
    );
    assert!(
        in_hand(&engine, caster, plains()).is_some(),
        "the draw replaced the sorcery in hand"
    );
}
