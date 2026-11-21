//! Q Continuum's land drop, modal trigger, and mana ability.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn q_continuum() -> CardIndex {
    card_index("457cf958-8129-4bf6-b4d5-211eb8c58763")
}

#[test]
fn q_continuum_enters_tapped_and_skips_its_trigger_with_one_other_land() {
    let me = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[q_continuum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let land = play_land(&mut engine, me, q_continuum());
    assert!(entered_tapped(&engine, land));
    assert!(matches!(engine.pending(), Pending::Priority { player, .. } if *player == me));
    assert!(stack_is_empty(&engine));
}

#[test]
fn q_continuum_can_pump_an_opponents_creature_for_the_turn_and_tap_for_green() {
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[q_continuum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let elf = on_battlefield(&engine, them, llanowar_elves()).unwrap();
    let land = play_land(&mut engine, me, q_continuum());
    assert!(!entered_tapped(&engine, land));
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending()
    else {
        panic!("the untapped land should offer both modes");
    };
    assert_eq!((*player, options.len()), (me, 2));
    let pump = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(me, PlayerAction::ChooseMode(pump)).unwrap();
    assert_eq!(aim_at(&mut engine, me, elf), vec![elf]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elf), (2, 2));
    assert!(keywords(&engine, elf).contains(KeywordSet::VIGILANCE));

    activate(&mut engine, me, q_continuum(), 1);
    assert!(is_tapped(&engine, land));
    let mana = &engine.state().players[0].mana_pool;
    assert_eq!(mana.total(), 1);
    assert_eq!(mana.available(ManaColor::Green), 1);

    reach_their_main_phase(&mut engine, them);
    assert_eq!(pt(&engine, elf), (1, 1));
    assert!(!keywords(&engine, elf).contains(KeywordSet::VIGILANCE));
}

#[test]
fn q_continuum_can_surveil_one_even_without_any_creature_targets() {
    let me = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[q_continuum()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let top = *engine
        .state()
        .zones
        .list(ZoneLocation::Library(me))
        .last()
        .unwrap();
    let land = play_land(&mut engine, me, q_continuum());
    assert!(!entered_tapped(&engine, land));
    let Pending::ChooseCastMode {
        player, options, ..
    } = engine.pending()
    else {
        panic!("the untapped land should offer surveil");
    };
    assert_eq!((*player, options.len()), (me, 1));
    let surveil = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(me, PlayerAction::ChooseMode(surveil)).unwrap();

    let (cards, piles) = surveil_offer(&mut engine, me);
    assert_eq!(cards, vec![top]);
    assert_eq!(piles, surveil_piles(1));
    engine.apply(me, look_answer(&cards, &cards)).unwrap();
    assert_eq!(engine.state().object(top).unwrap().zone, Zone::Graveyard);
}
