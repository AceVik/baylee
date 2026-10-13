//! `cards/lands/utility/balduvian_trading_post.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pay the untapped Mountain entry replacement, then tap for {C}{R}.
#[test]
fn balduvian_trading_post_taps_for_colorless_and_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(330, forest())
        .hand(0, &[balduvian_trading_post()])
        .battlefield(0, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, balduvian_trading_post());
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    let payment = on_battlefield(&engine, p0, mountain()).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![payment],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(payment).unwrap().zone,
        Zone::Graveyard
    );
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, balduvian_trading_post(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// "{1}, {T}: This land deals 1 damage to target attacking creature." Only an
/// attacking creature is a target, and the land is the damage source.
#[test]
fn balduvian_trading_post_deals_one_damage_to_an_attacking_creature() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[balduvian_trading_post(), index::GRIZZLY_BEARS])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_their_main_phase(&mut e, p1);
    let post = on_battlefield(&e, p0, balduvian_trading_post()).unwrap();
    let bears = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
    let elves = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| e.state().object(*id).is_some_and(|o| o.controller == p1))
        .collect::<Vec<_>>();
    let (attacker, idle) = (elves[0], elves[1]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, p0, balduvian_trading_post(), 1);
    let options = aim_at(&mut e, p0, attacker);
    assert!(!options.contains(&idle), "a creature that is not attacking");
    assert!(
        !options.contains(&bears),
        "none of mine is attacking either"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(attacker).unwrap().zone,
        Zone::Graveyard,
        "1 damage kills the 1/1 attacker"
    );
    assert_eq!(e.state().object(idle).unwrap().zone, Zone::Battlefield);
    assert!(is_tapped(&e, post));
}
