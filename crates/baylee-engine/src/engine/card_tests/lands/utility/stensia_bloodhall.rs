//! `cards/lands/utility/stensia_bloodhall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stensia Bloodhall: "{T}: Add {C}." / "{3}{B}{R}, {T}: This land deals 2 damage to target player or planeswalker."
/// Both the mana and damage abilities are available at their printed costs.
/// The land taps for its mana ability adding {C} to the mana pool.
#[test]
fn stensia_bloodhall_taps_for_colorless_and_offers_damage_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        // {3}{B}{R} is what the missing ability charges, and the offer is
        // filtered by `can_afford` — which reads the pool. Two Swamps and
        // three Mountains make exactly that price, in exactly those
        // colours.
        .battlefield(
            0,
            &[
                stensia_bloodhall(),
                swamp(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall = on_battlefield(&engine, p0, stensia_bloodhall()).expect("Bloodhall deployed");

    // Before the mana ability, not after it: the land's own {T} is part of
    // the price too, so a pin read off a tapped land proves nothing twice
    // over.
    tap_mana_except(&mut engine, p0, hall);
    let (black, red) = {
        let pool = &engine.state().players[0].mana_pool;
        (
            pool.available(ManaColor::Black),
            pool.available(ManaColor::Red),
        )
    };
    assert_eq!((black, red), (2, 3), "the {{3}}{{B}}{{R}} is floating");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.iter().any(|(s, ai)| *s == hall && *ai == 1),
        "the damage ability is offered, and its price is standing here: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, stensia_bloodhall(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, hall));
}

#[test]
fn bloodhall_deals_two_to_its_chosen_player() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::STENSIA_BLOODHALL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let pool = &mut e.dev_state_mut(p0).unwrap().players[0].mana_pool;
    pool.add(ManaColor::Black, 4);
    pool.add(ManaColor::Red, 1);
    e.refresh_offer();
    activate(&mut e, p0, index::STENSIA_BLOODHALL, 1);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 18);
}

/// "{3}{B}{R}, {T}: This land deals 2 damage to target player or
/// planeswalker." A creature is no target; a planeswalker is, whoever
/// controls it, and the damage takes its loyalty away.
#[test]
fn bloodhall_targets_players_and_planeswalkers_but_not_creatures() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::STENSIA_BLOODHALL])
        .battlefield(1, &[index::JACE_THE_MIND_SCULPTOR, llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let walker = on_battlefield(&e, p1, index::JACE_THE_MIND_SCULPTOR).unwrap();
    let elf = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    state
        .object_mut(walker)
        .unwrap()
        .counters
        .set(CounterKind::Loyalty, 5);
    state.players[0].mana_pool.add(ManaColor::Black, 4);
    state.players[0].mana_pool.add(ManaColor::Red, 1);
    e.refresh_offer();
    activate(&mut e, p0, index::STENSIA_BLOODHALL, 1);
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = e.pending().clone()
    else {
        panic!("a target choice was expected, got {:?}", e.pending());
    };
    assert!(!options.contains(&elf), "a creature is neither");
    assert_eq!(
        options,
        vec![walker],
        "the planeswalker alone among objects"
    );
    assert!(player_options.contains(&p1), "and the player");
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![walker],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, walker, CounterKind::Loyalty), 3, "5 - 2");
    assert_eq!(e.state().players[1].life, 20, "the player was not hit");
    assert_eq!(e.state().object(elf).unwrap().damage, 0);
}
