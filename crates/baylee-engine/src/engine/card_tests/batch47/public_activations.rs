#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch47_clergy_cannot_regenerate_through_terminate() {
    let mut e = prepared(
        &[index::TERMINATE],
        &[index::CLERGY_OF_THE_HOLY_NIMBUS],
        &[],
    );
    let clergy = on_battlefield(&e, P0, index::CLERGY_OF_THE_HOLY_NIMBUS).unwrap();
    cast_at(&mut e, index::TERMINATE, vec![clergy]);
    assert_eq!(e.state.object(clergy).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch47_clergy_zero_toughness_dies_without_regenerating() {
    let mut e = prepared(&[index::WEAKNESS], &[index::CLERGY_OF_THE_HOLY_NIMBUS], &[]);
    let clergy = on_battlefield(&e, P0, index::CLERGY_OF_THE_HOLY_NIMBUS).unwrap();
    cast_at(&mut e, index::WEAKNESS, vec![clergy]);
    assert_eq!(e.state.object(clergy).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch47_ifh_lifelink_belongs_to_efreets_controller_when_an_opponent_activates() {
    let mut e = prepared(&[], &[index::IFH_BIFF_EFREET], &[index::SERRA_ANGEL]);
    let efreet = on_battlefield(&e, P0, index::IFH_BIFF_EFREET).unwrap();
    let base = e.state.object_mut(efreet).unwrap().base_mut();
    base.keywords = base.keywords.union(KeywordSet::LIFELINK);
    let before = [e.state.players[0].life, e.state.players[1].life];
    set_pool(&mut e, P1, &[(ManaColor::Green, 1)]);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    activate_as(&mut e, P1, efreet, 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        [e.state.players[0].life, e.state.players[1].life],
        [before[0] + 3, before[1] - 1]
    );
}

#[test]
fn batch47_clergy_replaces_repeated_destruction_and_opponent_can_disable_it() {
    for disable in [false, true] {
        let mut e = prepared(
            &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT],
            &[index::CLERGY_OF_THE_HOLY_NIMBUS],
            &[],
        );
        let clergy = on_battlefield(&e, P0, index::CLERGY_OF_THE_HOLY_NIMBUS).unwrap();
        assert!(
            e.apply(
                P0,
                PlayerAction::ActivateAbility {
                    source: clergy,
                    ability_index: 1
                }
            )
            .is_err()
        );
        if disable {
            set_pool(&mut e, P1, &[(ManaColor::Colorless, 1)]);
            e.apply(P0, PlayerAction::PassPriority).unwrap();
            activate_as(&mut e, P1, clergy, 1);
            assert_eq!(e.state.players[1].mana_pool.total(), 0);
            pass_until(&mut e, stack_is_empty);
        }
        cast_at(&mut e, index::LIGHTNING_BOLT, vec![clergy]);
        assert_eq!(
            e.state.object(clergy).unwrap().zone,
            if disable {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            }
        );
        if !disable {
            assert!(is_tapped(&e, clergy));
            assert_eq!(e.state.object(clergy).unwrap().damage, 0);
            cast_at(&mut e, index::LIGHTNING_BOLT, vec![clergy]);
            assert_eq!(e.state.object(clergy).unwrap().zone, Zone::Battlefield);
        }
    }
}

#[test]
fn batch47_ifh_opponent_pays_controls_ability_and_the_efreet_remains_damage_source() {
    let mut e = prepared(
        &[],
        &[index::IFH_BIFF_EFREET, index::GRIZZLY_BEARS],
        &[index::SERRA_ANGEL],
    );
    let efreet = on_battlefield(&e, P0, index::IFH_BIFF_EFREET).unwrap();
    let bear = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let angel = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    let before = [e.state.players[0].life, e.state.players[1].life];
    let event_start = e.journal().entries().len();
    set_pool(&mut e, P1, &[(ManaColor::Green, 1)]);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    activate_as(&mut e, P1, efreet, 0);
    let ability = *e.state.zones.list(ZoneLocation::Stack).last().unwrap();
    assert_eq!(e.state.object(ability).unwrap().controller, P1);
    assert_eq!(e.state.object(efreet).unwrap().controller, P0);
    assert_eq!(e.state.players[1].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        [e.state.players[0].life, e.state.players[1].life],
        [before[0] - 1, before[1] - 1]
    );
    assert_eq!(e.state.object(efreet).unwrap().damage, 1);
    assert_eq!(e.state.object(angel).unwrap().damage, 1);
    assert_eq!(e.state.object(bear).unwrap().damage, 0);
    assert!(
        damage_events(&e, event_start)
            .iter()
            .all(|(source, _, _, _)| *source == efreet)
    );
}

#[test]
fn batch47_lands_edge_opponent_discards_own_card_and_only_land_discard_deals_damage() {
    for discarded in [index::FOREST, index::GRIZZLY_BEARS] {
        let mut e = Duel::new(SEED, index::FOREST)
            .battlefield(0, &[index::LAND_S_EDGE])
            .hand(1, &[discarded])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, P0);
        let edge = on_battlefield(&e, P0, index::LAND_S_EDGE).unwrap();
        let card = in_hand(&e, P1, discarded).unwrap();
        let before = e.state.players[0].life;
        let mine = e.state.zones.list(ZoneLocation::Hand(P0)).clone();
        e.apply(P0, PlayerAction::PassPriority).unwrap();
        activate_as(&mut e, P1, edge, 0);
        // Targets are announced even for a nonland discard.
        if matches!(e.pending(), Pending::ChooseTargets { .. }) {
            targets(&mut e, vec![], vec![P0]);
        }
        let Pending::ChooseCards {
            player, options, ..
        } = e.pending().clone()
        else {
            panic!("{:?}", e.pending())
        };
        assert_eq!(player, P1);
        assert!(options.contains(&card));
        e.apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![card],
            },
        )
        .unwrap();
        if matches!(e.pending(), Pending::ChooseTargets { .. }) {
            targets(&mut e, vec![], vec![P0]);
        }
        assert_eq!(e.state.object(card).unwrap().zone, Zone::Graveyard);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state.players[0].life,
            before - if discarded == index::FOREST { 2 } else { 0 }
        );
        assert_eq!(e.state.zones.list(ZoneLocation::Hand(P0)), &mine);
    }
}

#[test]
fn batch47_clock_accepts_zero_counter_removal_during_upkeep_and_damages_in_draw() {
    // Stop before the first upkeep trigger resolves, rather than using
    // prepared(), which has already added a doom counter and dealt damage.
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::ARMAGEDDON_CLOCK])
        .start();
    keep_mulligans(&mut e);
    let clock = on_battlefield(&e, P0, index::ARMAGEDDON_CLOCK).unwrap();
    let before = [e.state.players[0].life, e.state.players[1].life];
    pass_until(&mut e, |e| {
        e.state.turn.active == P0 && e.state.turn.step == crate::turn::Step::Upkeep
    });
    assert_eq!(counters_on(&e, clock, baylee_cards_dsl::counters::DOOM), 0);
    set_pool(&mut e, P1, &[(ManaColor::Colorless, 4)]);
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    activate_as(&mut e, P1, clock, 2);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, clock, baylee_cards_dsl::counters::DOOM), 1);
    assert_eq!([e.state.players[0].life, e.state.players[1].life], before);
    // The starting player skips the first draw step. Wait for the next
    // controller upkeep, which adds a second counter, then its draw trigger.
    let turn = e.state.turn.number;
    pass_until(&mut e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == P0
            && e.state.turn.step == crate::turn::Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, clock, baylee_cards_dsl::counters::DOOM), 2);
    pass_until(&mut e, |e| {
        e.state.turn.active == P0 && e.state.turn.step == crate::turn::Step::Draw
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        [e.state.players[0].life, e.state.players[1].life],
        [before[0] - 2, before[1] - 2]
    );
    assert!(
        e.apply(
            P0,
            PlayerAction::ActivateAbility {
                source: clock,
                ability_index: 2
            }
        )
        .is_err()
    );
}

#[test]
fn batch47_clock_removes_counters_as_an_effect_during_an_opponents_upkeep() {
    let mut e = prepared(&[], &[index::ARMAGEDDON_CLOCK], &[]);
    let clock = on_battlefield(&e, P0, index::ARMAGEDDON_CLOCK).unwrap();
    assert_eq!(
        counters_on(&e, clock, baylee_cards_dsl::counters::DOOM),
        1,
        "prepared has resolved the controller's first upkeep"
    );
    e.state
        .object_mut(clock)
        .unwrap()
        .counters
        .add(baylee_cards_dsl::counters::DOOM, 1);
    e.state.invalidate_projections();
    let before = [e.state.players[0].life, e.state.players[1].life];
    pass_until(&mut e, |e| {
        e.state.turn.active == P1
            && e.state.turn.step == crate::turn::Step::Upkeep
            && matches!(e.pending(), Pending::Priority { player: P1, .. })
    });
    assert_eq!(
        counters_on(&e, clock, baylee_cards_dsl::counters::DOOM),
        2,
        "only the controller's upkeep adds a counter"
    );
    set_pool(&mut e, P1, &[(ManaColor::Colorless, 4)]);
    activate_as(&mut e, P1, clock, 2);
    assert_eq!(e.state.players[1].mana_pool.total(), 0);
    assert_eq!(
        counters_on(&e, clock, baylee_cards_dsl::counters::DOOM),
        2,
        "counter removal is an effect, not an activation cost"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, clock, baylee_cards_dsl::counters::DOOM), 1);
    pass_until(&mut e, |e| {
        e.state.turn.active == P0 && e.state.turn.step == crate::turn::Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, clock, baylee_cards_dsl::counters::DOOM), 2);
    pass_until(&mut e, |e| e.state.turn.step == crate::turn::Step::Draw);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        [e.state.players[0].life, e.state.players[1].life],
        [before[0] - 2, before[1] - 2]
    );
}

#[test]
fn batch47_lands_edge_target_menu_is_players_and_planeswalkers_only() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::LAND_S_EDGE, index::GRIZZLY_BEARS])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .hand(1, &[index::FOREST])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let edge = on_battlefield(&e, P0, index::LAND_S_EDGE).unwrap();
    let card = in_hand(&e, P1, index::FOREST).unwrap();
    let before = e.state.players[0].life;
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    activate_as(&mut e, P1, edge, 0);

    // The discard and the target are asked in either order; read the target
    // menu whenever it comes.
    let mut menu = None;
    for _ in 0..4 {
        match e.pending().clone() {
            Pending::ChooseCards { player, .. } => {
                e.apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![card],
                    },
                )
                .unwrap();
            }
            Pending::ChooseTargets {
                options,
                player_options,
                ..
            } => {
                menu = Some((options, player_options));
                targets(&mut e, vec![], vec![P0]);
            }
            _ => break,
        }
    }
    let (options, mut player_options) = menu.expect("the ability asked for a target");
    assert!(
        options.is_empty(),
        "\"target player or planeswalker\": the Grizzly Bears of both players are no target: {options:?}"
    );
    player_options.sort();
    assert_eq!(
        player_options,
        vec![P0, P1],
        "either player may be targeted"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[0].life,
        before - 2,
        "the discarded land card made the 2 damage to the chosen player"
    );
}
