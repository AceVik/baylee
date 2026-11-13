//! `cards/creatures/mv_3/mutated_cultist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cultist() -> CardIndex {
    card_index("5168be9b-ffca-41c5-a944-92a1a2e30779")
}

#[test]
fn mutated_cultists_deathtouch_kills_the_spider_with_one_damage() {
    let mut e = lk::game(SEED, &[cultist()], &[giant_spider()], &[], &[]);
    let attacker = lk::obj(&e, P0, cultist());
    let blocker = lk::obj(&e, P1, giant_spider());
    assert!(keywords(&e, attacker).contains(KeywordSet::DEATHTOUCH));

    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(blocker, attacker)]);
    lk::through_combat(&mut e);

    assert_eq!(lk::zone_of(&e, blocker), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, attacker), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, attacker), 2);
    assert_eq!(lk::life(&e, P1), 20);
}

#[test]
fn mutated_cultists_devoid_makes_it_a_legal_doom_blade_target() {
    let mut e = lk::game(
        SEED,
        &[cultist(), festering_goblin()],
        &[],
        &[],
        &[index::DOOM_BLADE],
    );
    let colorless = lk::obj(&e, P0, cultist());
    let black = lk::obj(&e, P0, festering_goblin());
    assert!(keywords(&e, colorless).contains(KeywordSet::DEVOID));
    e.apply(P0, PlayerAction::PassPriority).unwrap();
    lk::float(&mut e, P1, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut e, P1, index::DOOM_BLADE);

    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("Doom Blade offers its targets: {:?}", e.pending())
    };
    assert!(options.contains(&colorless), "devoid overrides black mana");
    assert!(!options.contains(&black), "the black control creature");
    lk::aim(&mut e, &[colorless], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::zone_of(&e, colorless), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, black), Zone::Battlefield);
}

fn player_counters(e: &Engine<RegistryLookup>, player: PlayerId, kind: CounterKind) -> u16 {
    e.state().players[usize::from(player.get())]
        .counters()
        .into_iter()
        .find_map(|(actual, count)| (actual == kind).then_some(count))
        .unwrap_or(0)
}

#[track_caller]
fn cast_cultist(e: &mut Engine<RegistryLookup>) {
    lk::float(e, P0, &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)]);
    cast_with_floating(e, P0, cultist());
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseTargets { .. }));
}

#[test]
fn mutated_cultist_cleanses_a_permanent_or_opponent_and_discounts_only_the_next_spell() {
    for target_kind in 0..3 {
        let mut e = lk::game(
            SEED,
            &[index::JAYEMDAE_TOME, index::ROD_OF_RUIN],
            &[],
            &[cultist(), index::DIVINATION, index::DIVINATION],
            &[],
        );
        let permanent = lk::obj(&e, P0, index::JAYEMDAE_TOME);
        let other = lk::obj(&e, P0, index::ROD_OF_RUIN);
        let state = e.dev_state_mut(P0).expect("the harness prepares counters");
        baylee_engine::replacement::put_counters(state, permanent, CounterKind::P1P1, 2);
        baylee_engine::replacement::put_counters(state, permanent, CounterKind::Charge, 2);
        baylee_engine::replacement::put_counters(state, permanent, CounterKind::Lore, 1);
        baylee_engine::replacement::put_counters(state, other, CounterKind::Charge, 2);
        state.change_player_counters(P1, CounterKind::Poison, 3);
        state.change_player_counters(P1, CounterKind::Energy, 2);
        state.change_player_counters(P0, CounterKind::Poison, 2);

        cast_cultist(&mut e);
        let Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            ..
        } = e.pending().clone()
        else {
            unreachable!()
        };
        assert!(options.contains(&permanent));
        assert_eq!(player_options, vec![P1]);
        assert_eq!((min, max), (0, 1), "up to one permanent or opponent");
        let (objects, players) = match target_kind {
            0 => (vec![permanent], vec![]),
            1 => (vec![], vec![P1]),
            _ => (vec![], vec![]),
        };
        e.apply(P0, PlayerAction::ChooseTargets { objects, players })
            .unwrap();
        lk::settle(&mut e);

        assert!(on_battlefield(&e, P0, cultist()).is_some());
        assert_eq!(
            counters_on(&e, permanent, CounterKind::P1P1),
            if target_kind == 0 { 0 } else { 2 }
        );
        assert_eq!(
            counters_on(&e, permanent, CounterKind::Charge),
            if target_kind == 0 { 0 } else { 2 }
        );
        assert_eq!(
            counters_on(&e, permanent, CounterKind::Lore),
            u16::from(target_kind != 0)
        );
        assert_eq!(counters_on(&e, other, CounterKind::Charge), 2);
        for (kind, original) in [(CounterKind::Poison, 3), (CounterKind::Energy, 2)] {
            assert_eq!(
                player_counters(&e, P1, kind),
                if target_kind == 1 { 0 } else { original }
            );
        }
        assert_eq!(player_counters(&e, P0, CounterKind::Poison), 2);

        // Divination costs {2}{U}: five counters removed make it cost {U},
        // and the following copy returns to its full three mana.
        let first = in_hand(&e, P0, index::DIVINATION).unwrap();
        let Pending::Priority { legal, .. } = e.pending().clone() else {
            panic!("the caster has priority: {:?}", e.pending())
        };
        assert_eq!(
            legal.spell_reduction(first, CastModeKind::Normal),
            if target_kind == 2 { 0 } else { 5 }
        );
        lk::float(&mut e, P0, &[(ManaColor::Blue, 1)]);
        if target_kind == 2 {
            lk::float(&mut e, P0, &[(ManaColor::Colorless, 2)]);
        }
        cast_with_floating(&mut e, P0, index::DIVINATION);
        assert_eq!(lk::pool_total(&e, P0), 0, "the reduced generic cost");
        assert_eq!(lk::zone_of(&e, first), Zone::Stack);
        lk::settle(&mut e);

        let second = in_hand(&e, P0, index::DIVINATION).unwrap();
        lk::float(
            &mut e,
            P0,
            &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        );
        let Pending::Priority { legal, .. } = e.pending().clone() else {
            panic!("priority before the second spell: {:?}", e.pending())
        };
        assert_eq!(legal.spell_reduction(second, CastModeKind::Normal), 0);
        assert!(
            !legal.castable.contains(&second),
            "the next spell is full price"
        );
        lk::float(&mut e, P0, &[(ManaColor::Colorless, 1)]);
        cast_with_floating(&mut e, P0, index::DIVINATION);
        assert_eq!(lk::pool_total(&e, P0), 0);
        assert_eq!(lk::zone_of(&e, second), Zone::Stack);
    }
}

#[test]
fn mutated_cultists_unused_discount_expires_before_the_next_turn() {
    let mut e = lk::game(
        SEED,
        &[index::JAYEMDAE_TOME],
        &[],
        &[cultist(), index::DIVINATION],
        &[],
    );
    let permanent = lk::obj(&e, P0, index::JAYEMDAE_TOME);
    let state = e.dev_state_mut(P0).expect("the harness prepares counters");
    baylee_engine::replacement::put_counters(state, permanent, CounterKind::Charge, 3);
    cast_cultist(&mut e);
    lk::aim(&mut e, &[permanent], &[]);
    lk::settle(&mut e);
    assert_eq!(e.state().per_turn.next_spell_cost_reduction[0], 3);

    lk::through_combat(&mut e);
    pass_until(&mut e, |e| {
        e.state().turn.phase == baylee_engine::turn::Phase::FirstMain
            && e.state().turn.active == P0
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(e.state().per_turn.next_spell_cost_reduction[0], 0);
    let divination = in_hand(&e, P0, index::DIVINATION).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        panic!("the next turn offers priority: {:?}", e.pending())
    };
    assert_eq!(legal.spell_reduction(divination, CastModeKind::Normal), 0);
}
