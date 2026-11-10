//! `cards/lands/manlands/inkmoth_nexus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Inkmoth Nexus prints `{{T}}: Add {{C}}` and `{1}: This land becomes a 1/1
/// Phyrexian Blinkmoth artifact creature with flying and infect until end of turn.
/// It's still a land.`
///
/// Paying `{1}` animates the land into a 1/1 Phyrexian Blinkmoth artifact
/// creature with flying and infect that remains a land until end of turn.
#[test]
fn inkmoth_nexus_animates_into_a_flying_artifact_blinkmoth() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[inkmoth_nexus(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nexus = on_battlefield(&engine, p0, inkmoth_nexus()).expect("nexus on battlefield");
    assert!(
        !engine
            .state()
            .object(nexus)
            .expect("nexus exists")
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "a land is not a creature before activation"
    );

    tap_all_mana_but(&mut engine, p0, Some(inkmoth_nexus()));
    activate(&mut engine, p0, inkmoth_nexus(), 1);
    pass_until(&mut engine, stack_is_empty);

    let chars = engine
        .state()
        .object(nexus)
        .expect("nexus exists")
        .characteristics();
    assert!(chars.types.contains(TypeSet::LAND));
    assert!(chars.types.contains(TypeSet::ARTIFACT));
    assert!(chars.types.contains(TypeSet::CREATURE));
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::PHYREXIAN)
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::BLINKMOTH)
    );
    assert_eq!(pt(&engine, nexus), (1, 1));
    assert!(keywords(&engine, nexus).contains(KeywordSet::FLYING));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    let chars = engine.state().object(nexus).unwrap().characteristics();
    assert_eq!(chars.types, TypeSet::LAND);
    assert!(
        !chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::PHYREXIAN)
    );
    assert!(
        !chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::BLINKMOTH)
    );
    assert!(!chars.keywords.contains(KeywordSet::FLYING));
}

/// The unanimated land taps for exactly one colorless mana without using the stack.
#[test]
fn inkmoth_nexus_taps_for_one_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[inkmoth_nexus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nexus = on_battlefield(&engine, p0, inkmoth_nexus()).unwrap();
    activate(&mut engine, p0, inkmoth_nexus(), 0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, nexus));
    assert!(stack_is_empty(&engine));
}

/// Infect turns unblocked damage into poison, while flying excludes a ground blocker.
#[test]
fn batch55_inkmoth_nexus_infect_gives_poison_instead_of_life_loss() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[inkmoth_nexus(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nexus = on_battlefield(&engine, p0, inkmoth_nexus()).unwrap();
    tap_mana_except(&mut engine, p0, nexus);
    activate(&mut engine, p0, inkmoth_nexus(), 1);
    pass_until(&mut engine, stack_is_empty);
    let blocks = attack_and_collect_blocks(&mut engine, nexus, p1);
    assert!(!blocks.iter().any(|b| b.attackers.contains(&nexus)));
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(engine.state().players[1].poison, 1);
    assert_eq!(engine.state().players[0].poison, 0);
}

/// Infect damage leaves counters, not marked damage, on a surviving reach blocker.
#[test]
fn batch55_inkmoth_nexus_infect_counters_survive_cleanup() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[inkmoth_nexus(), forest()])
        .battlefield(1, &[giant_spider(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nexus = on_battlefield(&engine, p0, inkmoth_nexus()).unwrap();
    let spider = on_battlefield(&engine, p1, giant_spider()).unwrap();
    let elf = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    assert_eq!(pt(&engine, spider), (2, 4));
    tap_mana_except(&mut engine, p0, nexus);
    activate(&mut engine, p0, inkmoth_nexus(), 1);
    pass_until(&mut engine, stack_is_empty);
    let blocks = attack_and_collect_blocks(&mut engine, nexus, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == spider && b.attackers.contains(&nexus))
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(spider, nexus)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        counters_on(&engine, spider, baylee_cards_dsl::CounterKind::M1M1),
        1
    );
    assert_eq!(engine.state().object(spider).unwrap().damage, 0);
    assert_eq!(pt(&engine, spider), (1, 3));
    assert_eq!(pt(&engine, elf), (1, 1));
    assert_eq!(engine.state().players[1].poison, 0);
    assert!(in_graveyard(&engine, p0, inkmoth_nexus()).is_some());
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, spider), (1, 3));
    assert_eq!(
        counters_on(&engine, spider, baylee_cards_dsl::CounterKind::M1M1),
        1
    );
}

/// A newly played animated Nexus cannot attack or pay its tap cost until the next own turn.
#[test]
fn batch55_inkmoth_nexus_new_land_has_summoning_sickness_when_animated() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[inkmoth_nexus()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let nexus = play_land(&mut engine, p0, inkmoth_nexus());
    tap_mana_except(&mut engine, p0, nexus);
    activate(&mut engine, p0, inkmoth_nexus(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(!priority_offer(&engine).abilities.contains(&(nexus, 0)));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(!attackers.contains(&nexus));
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    tap_mana_except(&mut engine, p0, nexus);
    activate(&mut engine, p0, inkmoth_nexus(), 1);
    pass_until(&mut engine, stack_is_empty);
    assert!(priority_offer(&engine).abilities.contains(&(nexus, 0)));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(attackers.contains(&nexus));
}
