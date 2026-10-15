//! `cards/lands/restricted/diamond_city.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Diamond City: "This land enters with a shield counter on it." / "{T}: Add {C}." / "{T}: Move a shield counter from this land onto target creature..."
/// Under `Coverage::Partial`, shield counter clauses are omitted because `CounterKind` has no shield counter variant.
/// Playing Diamond City enters untapped with no counters and taps immediately for colorless mana.
#[test]
fn diamond_city_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(221, forest()).hand(0, &[diamond_city()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, diamond_city());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, diamond_city(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aim(e: &mut Engine<RegistryLookup>, seat: PlayerId, objects: &[ObjectId], players: &[PlayerId]) {
    let Pending::ChooseTargets { player, .. } = e.pending() else {
        panic!("target question: {:?}", e.pending())
    };
    assert_eq!(*player, seat);
    e.apply(
        seat,
        PlayerAction::ChooseTargets {
            objects: objects.to_vec(),
            players: players.to_vec(),
        },
    )
    .unwrap();
}

fn cast_aim(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    cast_from_hand(e, seat, card);
    aim(e, seat, objects, players);
}

fn city_ready(
    theirs: &[CardIndex],
    hand1: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::DIAMOND_CITY,
                index::PLAINS,
                index::PLAINS,
                index::ROOTBREAKER_WURM,
            ],
        )
        .hand(0, &[index::RAISE_THE_ALARM])
        .battlefield(1, theirs)
        .hand(1, hand1)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let city = on_battlefield(&e, P0, index::DIAMOND_CITY).unwrap();
    let root = on_battlefield(&e, P0, index::ROOTBREAKER_WURM).unwrap();
    assert_eq!(counters_on(&e, city, CounterKind::Shield), 1);
    assert!(!priority_offer(&e).abilities.contains(&(city, 1)));
    tap_mana_except(&mut e, P0, city);
    cast_with_floating(&mut e, P0, index::RAISE_THE_ALARM);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(tokens_of(&e, P0).len(), 2);
    assert!(priority_offer(&e).abilities.contains(&(city, 1)));
    (e, city, root)
}

#[test]
fn batch57_diamond_moves_one_shield_to_either_players_creature_and_prevents_one_damage_event() {
    for own in [false, true] {
        let (mut e, city, root) = city_ready(
            &[index::MOUNTAIN, index::MOUNTAIN, index::CRAW_WURM],
            &[index::LIGHTNING_BOLT, index::LIGHTNING_BOLT],
        );
        let other = on_battlefield(&e, P1, index::CRAW_WURM).unwrap();
        let recipient = if own { root } else { other };
        kit::activate_object(&mut e, P0, city, 1);
        let Pending::ChooseTargets {
            player, options, ..
        } = e.pending()
        else {
            panic!("target creature")
        };
        assert_eq!(*player, P0);
        assert!(options.contains(&root) && options.contains(&other));
        assert!(!options.contains(&city));
        aim(&mut e, P0, &[recipient], &[]);
        pass_until(&mut e, stack_is_empty);
        assert!(is_tapped(&e, city));
        assert_eq!(counters_on(&e, city, CounterKind::Shield), 0);
        assert_eq!(counters_on(&e, recipient, CounterKind::Shield), 1);
        cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[recipient], &[]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, recipient, CounterKind::Shield), 0);
        assert_eq!(e.state().object(recipient).unwrap().damage, 0);
        cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[recipient], &[]);
        pass_until(&mut e, stack_is_empty);
        assert_eq!(e.state().object(recipient).unwrap().damage, 3);
        assert_eq!(
            counters_on(&e, if own { other } else { root }, CounterKind::Shield),
            0
        );
    }
}

#[test]
fn batch57_diamond_counts_departed_entrants_and_does_not_require_a_shield_as_activation_cost() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::DIAMOND_CITY,
                index::PLAINS,
                index::FOREST,
                index::ISLAND,
            ],
        )
        .hand(
            0,
            &[
                index::THRABEN_INSPECTOR,
                index::LLANOWAR_ELVES,
                index::TWIDDLE,
            ],
        )
        .battlefield(1, &[index::PLAINS, index::GRIZZLY_BEARS])
        .hand(1, &[index::SWORDS_TO_PLOWSHARES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let city = on_battlefield(&e, P0, index::DIAMOND_CITY).unwrap();
    tap_mana_except(&mut e, P0, city);
    cast_with_floating(&mut e, P0, index::THRABEN_INSPECTOR);
    pass_until(&mut e, stack_is_empty);
    let inspector = on_battlefield(&e, P0, index::THRABEN_INSPECTOR).unwrap();
    assert!(
        !priority_offer(&e).abilities.contains(&(city, 1)),
        "one creature plus a Clue is only one creature arrival"
    );
    cast_aim(&mut e, P1, index::SWORDS_TO_PLOWSHARES, &[inspector], &[]);
    pass_until(&mut e, stack_is_empty);
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority{player,..}if *player==P0),
    );
    cast_with_floating(&mut e, P0, index::LLANOWAR_ELVES);
    pass_until(&mut e, stack_is_empty);
    let elf = on_battlefield(&e, P0, index::LLANOWAR_ELVES).unwrap();
    assert!(
        priority_offer(&e).abilities.contains(&(city, 1)),
        "the departed Inspector still counts"
    );
    kit::activate_object(&mut e, P0, city, 1);
    aim(&mut e, P0, &[elf], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, elf, CounterKind::Shield), 1);
    cast_aim(&mut e, P0, index::TWIDDLE, &[city], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(counters_on(&e, city, CounterKind::Shield), 0);
    assert!(!is_tapped(&e, city));
    assert!(
        priority_offer(&e).abilities.contains(&(city, 1)),
        "moving is an effect, not a counter-removal cost"
    );
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    kit::activate_object(&mut e, P0, city, 1);
    aim(&mut e, P0, &[bear], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, city));
    assert_eq!(counters_on(&e, bear, CounterKind::Shield), 0);
    reach_their_main_phase(&mut e, P1);
    reach_their_main_phase(&mut e, P0);
    assert!(
        !priority_offer(&e).abilities.contains(&(city, 1)),
        "arrival count resets next turn"
    );
}

#[test]
fn batch57_diamond_shield_replaces_destruction_once_but_not_sacrifice() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::DIAMOND_CITY])
        .battlefield(1, &[index::MOUNTAIN; 3])
        .hand(1, &[index::STONE_RAIN, index::STONE_RAIN])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    let city = on_battlefield(&e, P0, index::DIAMOND_CITY).unwrap();
    cast_aim(&mut e, P1, index::STONE_RAIN, &[city], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, index::DIAMOND_CITY).is_some());
    assert_eq!(counters_on(&e, city, CounterKind::Shield), 0);
    reach_their_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    cast_aim(&mut e, P1, index::STONE_RAIN, &[city], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::DIAMOND_CITY).is_some());
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::DIAMOND_CITY, index::ZURAN_ORB])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let city = on_battlefield(&e, P0, index::DIAMOND_CITY).unwrap();
    let orb = on_battlefield(&e, P0, index::ZURAN_ORB).unwrap();
    assert_eq!(counters_on(&e, city, CounterKind::Shield), 1);
    kit::activate_object(&mut e, P0, orb, 0);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![city],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(in_graveyard(&e, P0, index::DIAMOND_CITY).is_some());
    assert_eq!(e.state().players[0].life, 22);
}
