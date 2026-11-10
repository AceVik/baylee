//! `cards/lands/manlands/restless_anchorage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Anchorage enters tapped, prints `{{T}}: Add {{W}} or {{U}}`,
/// `{1}{W}{U}: Until end of turn, this land becomes a 2/3 white and blue
/// Bird creature with flying. It's still a land.`, and `Whenever this land
/// attacks, create a Map token.`
///
/// This test covers the entry and animation clauses, including their
/// end-of-turn duration; the attack and Map abilities are exercised below.
#[test]
fn restless_anchorage_enters_tapped_and_animates_into_a_bird() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), island()])
        .hand(0, &[restless_anchorage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_anchorage());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    tap_all_mana_but(&mut engine, p0, Some(restless_anchorage()));
    activate(&mut engine, p0, restless_anchorage(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (2, 3));
    let chars = engine
        .state()
        .object(land)
        .expect("land exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(chars.types.contains(TypeSet::LAND), "it's still a land");
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::BIRD)
    );
    assert!(chars.colors.contains(baylee_core::color::Color::White));
    assert!(chars.colors.contains(baylee_core::color::Color::Blue));
    assert!(keywords(&engine, land).contains(KeywordSet::FLYING));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let chars = engine.state().object(land).unwrap().characteristics();
    assert_eq!(chars.types, TypeSet::LAND);
    assert!(
        !chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::BIRD)
    );
    assert!(chars.colors.is_empty());
    assert!(!chars.keywords.contains(KeywordSet::FLYING));
}

/// Activate a Map and choose an offered creature, leaving its effect on the stack.
fn activate_map_target(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    map: ObjectId,
    creature: ObjectId,
) {
    engine
        .apply(
            player,
            PlayerAction::ActivateAbility {
                source: map,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseTargets {
        player: chooser,
        options,
        ..
    } = engine.pending().clone()
    else {
        panic!("Map targets a creature you control: {:?}", engine.pending());
    };
    assert_eq!(chooser, player);
    assert!(options.contains(&creature));
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![creature],
            },
        )
        .unwrap();
}

/// Attacking creates a Map whose sorcery-speed explore activation puts a revealed land in hand.
#[test]
fn batch55_restless_anchorage_attack_creates_a_usable_map() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                restless_anchorage(),
                plains(),
                island(),
                forest(),
                forest(),
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let anchorage = on_battlefield(&engine, p0, restless_anchorage()).unwrap();
    let bear = on_battlefield(&engine, p0, baylee_core::generated::index::GRIZZLY_BEARS).unwrap();
    let forests: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    assert_eq!(forests.len(), 2);
    tap_mana_where(&mut engine, p0, |id| {
        id != anchorage && id != forests[1] && id != bear
    });
    activate(&mut engine, p0, restless_anchorage(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(tokens_of(&engine, p0).is_empty());
    attack_and_collect_blocks(&mut engine, anchorage, p1);
    let maps = tokens_of(&engine, p0);
    assert_eq!(maps.len(), 1);
    let map = maps[0];
    let chars = engine.state().object(map).unwrap().characteristics();
    assert_eq!(chars.types, TypeSet::ARTIFACT);
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::artifact::MAP)
    );
    assert!(chars.colors.is_empty());
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    assert!(
        !priority_offer(&engine).abilities.contains(&(map, 0)),
        "Map is sorcery speed even with a mana source and legal target"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_where(&mut engine, p0, |id| id == forests[1]);
    assert!(
        priority_offer(&engine).abilities.contains(&(map, 0)),
        "own main phase permits the activation"
    );
    activate_map_target(&mut engine, p0, map, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "Map sacrificed as activation cost"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before + 1
    );
    assert_eq!(
        counters_on(&engine, bear, baylee_cards_dsl::CounterKind::P1P1),
        0
    );
    assert_eq!(pt(&engine, bear), (2, 2));
}

/// The printed mana ability offers precisely white or blue and adds exactly one chosen mana.
#[test]
fn batch55_restless_anchorage_taps_for_white_or_blue() {
    let p0 = PlayerId::new(0);
    for color in [ManaColor::White, ManaColor::Blue] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[restless_anchorage()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let anchorage = on_battlefield(&engine, p0, restless_anchorage()).unwrap();
        activate(&mut engine, p0, restless_anchorage(), 0);
        let Pending::ChooseColor {
            player, options, ..
        } = engine.pending().clone()
        else {
            panic!("expected white or blue mana choice: {:?}", engine.pending());
        };
        assert_eq!(player, p0);
        assert_eq!(options.len(), 2);
        assert!(options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue));
        engine.apply(p0, PlayerAction::ChooseColor(color)).unwrap();
        assert!(is_tapped(&engine, anchorage));
        assert!(stack_is_empty(&engine));
        assert_eq!(engine.state().players[0].mana_pool.total(), 1);
        assert_eq!(engine.state().players[0].mana_pool.available(color), 1);
        assert!(!priority_offer(&engine).abilities.contains(&(anchorage, 0)));
    }
}

/// An animated Anchorage does not create a Map when a different creature attacks.
#[test]
fn batch55_restless_anchorage_ignores_another_creatures_attack() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bears = baylee_core::generated::index::GRIZZLY_BEARS;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[restless_anchorage(), plains(), island(), forest(), bears],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let anchorage = on_battlefield(&engine, p0, restless_anchorage()).unwrap();
    let bear = on_battlefield(&engine, p0, bears).unwrap();
    tap_mana_except(&mut engine, p0, anchorage);
    activate(&mut engine, p0, restless_anchorage(), 1);
    pass_until(&mut engine, stack_is_empty);
    attack_and_collect_blocks(&mut engine, bear, p1);
    assert!(tokens_of(&engine, p0).is_empty());
}

/// If the Map's creature is removed in response, it neither reveals nor puts a land in hand.
#[test]
fn batch55_restless_anchorage_map_rechecks_its_creature_target() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bears = baylee_core::generated::index::GRIZZLY_BEARS;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                restless_anchorage(),
                plains(),
                island(),
                forest(),
                forest(),
                bears,
            ],
        )
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let anchorage = on_battlefield(&engine, p0, restless_anchorage()).unwrap();
    let bear = on_battlefield(&engine, p0, bears).unwrap();
    let forests: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == p0 && o.card.is_some_and(|c| c.index == forest()))
        })
        .collect();
    tap_mana_where(&mut engine, p0, |id| id != anchorage && id != forests[1]);
    activate(&mut engine, p0, restless_anchorage(), 1);
    pass_until(&mut engine, stack_is_empty);
    attack_and_collect_blocks(&mut engine, anchorage, p1);
    let map = tokens_of(&engine, p0)[0];
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    let before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let library_before = library_size(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| id == forests[1]);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: map,
                ability_index: 0,
            },
        )
        .unwrap();
    aim_at(&mut engine, p0, bear);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "sacrifice paid before resolution"
    );
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, lightning_bolt());
    aim_at(&mut engine, p1, bear);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, bears).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        before
    );
    assert_eq!(library_size(&engine, p0), library_before);
    assert_eq!(pt(&engine, anchorage), (2, 3));
}
