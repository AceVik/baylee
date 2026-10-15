//! `cards/lands/check/fire_nation_palace.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fire Nation Palace prints `This land enters tapped unless you control a basic land`, `{{T}}: Add {{R}}`, and a firebending ability.
/// When played while controlling a basic `forest()`, Fire Nation Palace enters untapped and immediately taps to add red mana to the pool.
#[test]
fn fire_nation_palace_enters_untapped_with_basic_land_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[fire_nation_palace()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let palace = play_land(&mut engine, p0, fire_nation_palace());
    assert!(!entered_tapped(&engine, palace));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );
}

/// An opposing basic land or an own nonbasic land is insufficient.
#[test]
fn fire_nation_palace_enters_tapped_without_your_basic_land() {
    for own_lands in [vec![], vec![baylee_core::generated::index::TAIGA]] {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &own_lands)
            .battlefield(1, &[forest()])
            .hand(0, &[fire_nation_palace()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let palace = play_land(&mut engine, p0, fire_nation_palace());
        assert!(entered_tapped(&engine, palace));
        assert!(!priority_offer(&engine).abilities.contains(&(palace, 0)));
    }
}

/// The chosen own creature produces four red mana on attack, retained only through combat.
#[test]
fn batch55_fire_nation_palace_grants_targeted_firebending_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fire_nation_palace(),
                mountain(),
                mountain(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[baylee_core::generated::index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let palace = on_battlefield(&engine, p0, fire_nation_palace()).unwrap();
    let bear = on_battlefield(&engine, p0, baylee_core::generated::index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let their_bear =
        on_battlefield(&engine, p1, baylee_core::generated::index::GRIZZLY_BEARS).unwrap();
    tap_mana_where(&mut engine, p0, |id| id != palace && id != elf);
    activate(&mut engine, p0, fire_nation_palace(), 1);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected target creature you control: {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&bear) && options.contains(&elf));
    assert!(!options.contains(&their_bear) && !options.contains(&palace));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, palace));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    attack_and_collect_blocks(&mut engine, bear, p1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatEnd);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        4
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    attack_and_collect_blocks(&mut engine, bear, p1);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "grant expired"
    );
}

/// A different own attacker receives no mana from the chosen creature's grant.
#[test]
fn batch55_fire_nation_palace_does_not_grant_firebending_to_bystanders() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fire_nation_palace(),
                mountain(),
                mountain(),
                baylee_core::generated::index::GRIZZLY_BEARS,
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let palace = on_battlefield(&engine, p0, fire_nation_palace()).unwrap();
    let bear = on_battlefield(&engine, p0, baylee_core::generated::index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    tap_mana_where(&mut engine, p0, |id| id != palace && id != elf);
    activate(&mut engine, p0, fire_nation_palace(), 1);
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);
    attack_and_collect_blocks(&mut engine, elf, p1);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

/// One red mana cannot pay the two-mana activation, despite a valid own creature.
#[test]
fn batch55_fire_nation_palace_requires_both_mana_and_tap_costs() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                fire_nation_palace(),
                mountain(),
                baylee_core::generated::index::GRIZZLY_BEARS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let palace = on_battlefield(&engine, p0, fire_nation_palace()).unwrap();
    tap_mana_except(&mut engine, p0, palace);
    assert!(!priority_offer(&engine).abilities.contains(&(palace, 1)));
    assert!(!is_tapped(&engine, palace));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
}
