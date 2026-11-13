//! `cards/creatures/mv_3/glissa_sunslayer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn glissa() -> CardIndex {
    card_index("8859a692-d107-4de2-9110-4681fc2761c7")
}

#[track_caller]
fn connect(e: &mut Engine<RegistryLookup>) {
    let attacker = lk::obj(e, P0, glissa());
    lk::attack(e, &[attacker]);
    lk::block(e, &[]);
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseCastMode { .. }));
}

#[track_caller]
fn choose_mode(e: &mut Engine<RegistryLookup>, mode: usize) {
    let Pending::ChooseCastMode { options, .. } = e.pending().clone() else {
        panic!("the combat trigger offers its modes: {:?}", e.pending())
    };
    let option = options
        .iter()
        .position(|option| option.kind == CastModeKind::Mode(mode))
        .expect("the printed mode is offered");
    e.apply(P0, PlayerAction::ChooseMode(option)).unwrap();
}

#[test]
fn glissas_first_strike_and_deathtouch_kill_the_spider_before_it_answers() {
    let mut e = lk::game(SEED, &[glissa()], &[giant_spider()], &[], &[]);
    let attacker = lk::obj(&e, P0, glissa());
    let blocker = lk::obj(&e, P1, giant_spider());
    let words = keywords(&e, attacker);
    assert!(words.contains(KeywordSet::FIRST_STRIKE));
    assert!(words.contains(KeywordSet::DEATHTOUCH));

    lk::attack(&mut e, &[attacker]);
    lk::block(&mut e, &[(blocker, attacker)]);
    lk::through_combat(&mut e);

    assert_eq!(lk::zone_of(&e, blocker), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, attacker), Zone::Battlefield);
    assert_eq!(lk::damage_on(&e, attacker), 0, "no regular damage step");
    assert_eq!(lk::life(&e, P1), 20, "the Spider blocked the damage");
}

#[test]
fn glissas_draw_mode_draws_exactly_one_and_loses_one_life() {
    let mut e = lk::game(SEED, &[glissa()], &[], &[forest()], &[]);
    let drawn = hand_to_library_top(&mut e, P0, forest());
    let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();

    connect(&mut e);
    choose_mode(&mut e, 0);
    lk::settle(&mut e);
    lk::through_combat(&mut e);

    assert_eq!(e.state().object(drawn).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand + 1);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 1
    );
    assert_eq!(lk::life(&e, P0), 19, "exactly one life is lost");
    assert_eq!(lk::life(&e, P1), 17, "three unblocked combat damage");
}

#[test]
fn glissas_second_mode_destroys_only_the_chosen_enchantment() {
    let mut e = lk::game(
        SEED,
        &[glissa()],
        &[index::GLORIOUS_ANTHEM, index::GLORIOUS_ANTHEM],
        &[],
        &[],
    );
    let enchantments = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state().object(*id).is_some_and(|object| {
                object
                    .card
                    .is_some_and(|card| card.index == index::GLORIOUS_ANTHEM)
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(enchantments.len(), 2);
    let destroyed = enchantments[1];
    let survivor = enchantments[0];

    connect(&mut e);
    choose_mode(&mut e, 1);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the enchantment mode targets: {:?}", e.pending())
    };
    assert_eq!(options, enchantments);
    lk::aim(&mut e, &[destroyed], &[]);
    lk::settle(&mut e);
    lk::through_combat(&mut e);

    assert_eq!(lk::zone_of(&e, destroyed), Zone::Graveyard);
    assert_eq!(lk::zone_of(&e, survivor), Zone::Battlefield);
    assert_eq!(lk::life(&e, P1), 17);
}

fn removal_game() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = lk::game(SEED, &[glissa()], &[index::JAYEMDAE_TOME], &[], &[]);
    let target = lk::obj(&e, P1, index::JAYEMDAE_TOME);
    let state = e.dev_state_mut(P0).expect("the harness prepares counters");
    for (kind, count) in [
        (CounterKind::P1P1, 2),
        (CounterKind::Charge, 2),
        (CounterKind::Lore, 1),
    ] {
        baylee_engine::replacement::put_counters(state, target, kind, count);
    }
    (e, target)
}

#[test]
fn glissas_third_mode_distributes_three_counters_across_all_present_kinds() {
    let (mut e, target) = removal_game();
    connect(&mut e);
    choose_mode(&mut e, 2);
    lk::aim(&mut e, &[target], &[]);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });

    for question in 0..3 {
        let Pending::ChooseNumber {
            min,
            max,
            reason: baylee_engine::choice::NumberPrompt::Counters { target: asked, .. },
            ..
        } = e.pending().clone()
        else {
            panic!("a present kind is offered: {:?}", e.pending())
        };
        assert_eq!((asked, min), (target, 0));
        assert!(max >= 1);
        e.apply(P0, PlayerAction::ChooseNumber(1)).unwrap();
        if question < 2 {
            assert!(matches!(
                e.pending(),
                Pending::YesNo {
                    prompt: YesNoPrompt::ContinueRemovingCounters { .. },
                    ..
                }
            ));
            lk::yes_no(&mut e, true);
        }
    }
    lk::settle(&mut e);
    lk::through_combat(&mut e);

    assert_eq!(counters_on(&e, target, CounterKind::P1P1), 1);
    assert_eq!(counters_on(&e, target, CounterKind::Charge), 1);
    assert_eq!(counters_on(&e, target, CounterKind::Lore), 0);
}

#[test]
fn glissas_counter_mode_allows_zero_and_stopping_before_the_maximum() {
    for (number, expected) in [(0, 5), (1, 4)] {
        let (mut e, target) = removal_game();
        connect(&mut e);
        choose_mode(&mut e, 2);
        lk::aim(&mut e, &[target], &[]);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseNumber { .. })
        });
        assert!(matches!(e.pending(), Pending::ChooseNumber { .. }));
        e.apply(P0, PlayerAction::ChooseNumber(number)).unwrap();
        assert!(matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::ContinueRemovingCounters { .. },
                ..
            }
        ));
        lk::yes_no(&mut e, false);
        lk::settle(&mut e);

        let remaining: u32 = e
            .state()
            .object(target)
            .unwrap()
            .counters
            .iter()
            .map(|(_, count)| u32::from(count))
            .sum();
        assert_eq!(remaining, expected);
    }
}
