//! `cards/creatures/mv_2/theorist_s_proxy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flash allows the Proxy in the end step while the affordable Channeler
/// remains unavailable. Its entry empowers Jace, and its sacrifice protects
/// a second flashed Proxy, which then empowers the same Jace again.
#[test]
fn theorist_s_proxy_flashes_in_empowers_jace_and_protects_the_next_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(
            0,
            &[theorist_s_proxy(), theorist_s_proxy(), aether_channeler()],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, theorist_s_proxy()).expect("the Proxy is in hand");
    let channeler = in_hand(&engine, p0, aether_channeler()).expect("the Channeler is in hand");
    assert!(
        legal.castable.contains(&card),
        "flash permits casting in the end step"
    );
    assert!(
        !legal.castable.contains(&channeler),
        "five blue can pay for Channeler, but its timing forbids casting"
    );
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    let proxy = on_battlefield(&engine, p0, theorist_s_proxy()).expect("the Proxy resolved");
    assert_eq!(pt(&engine, proxy), (0, 3));
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let jace = tokens[0];
    assert!(
        engine
            .state()
            .object(jace)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::planeswalker::JACE)
    );
    assert_eq!(counters_on(&engine, jace, CounterKind::Loyalty), 3);

    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(proxy, 1)),
        "the sacrifice is offered"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3
    );
    activate(&mut engine, p0, theorist_s_proxy(), 1);
    assert_eq!(engine.state().object(proxy).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        2
    );
    pass_until(&mut engine, stack_is_empty);

    let second = in_hand(&engine, p0, theorist_s_proxy()).expect("the second Proxy");
    engine
        .apply(p0, PlayerAction::CastSpell { card: second })
        .unwrap();
    assert!(
        engine
            .state()
            .object(second)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Uncounterable)
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p0), vec![jace]);
    assert_eq!(counters_on(&engine, jace, CounterKind::Loyalty), 6);
}
