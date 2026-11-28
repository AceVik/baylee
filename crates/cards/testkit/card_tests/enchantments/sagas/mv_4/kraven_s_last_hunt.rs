//! Kraven's Last Hunt (`{3}{G}`, Enchantment — Saga), played.
//!
//! Chapter I mills five cards and creates a reflexive triggered ability
//! (CR 603.12, CR 701.13, CR 714) that deals damage equal to the greatest power
//! among creature cards in your graveyard to target creature.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn kraven_s_last_hunt() -> CardIndex {
    card_index("e74efcf2-d77f-4fb5-9027-77e462ec829e")
}

/// Starts a duel with Kraven's Last Hunt in seat 0's hand, four Forests on
/// seat 0's battlefield, `library_top` stacked on top of seat 0's Forest
/// library, and both a 3/3 (Wild Elephant) and a 6/6 (Rootbreaker Wurm) on
/// seat 1's battlefield.
fn setup_kraven_board(library_top: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut hand = vec![kraven_s_last_hunt()];
    hand.extend_from_slice(library_top);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest()])
        .hand(0, &hand)
        .battlefield(1, &[wild_elephant(), rootbreaker_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for &card in library_top {
        let id = in_hand(&engine, p0, card).expect("stacked card in opening hand");
        engine
            .dev_state_mut(p0)
            .expect("test harness has dev state")
            .move_object(
                id,
                ZoneLocation::Library(p0),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .expect("card moves to top of library");
    }
    engine.refresh_offer();

    let elephant = on_battlefield(&engine, p1, wild_elephant()).expect("3/3 Wild Elephant");
    let wurm = on_battlefield(&engine, p1, rootbreaker_wurm()).expect("6/6 Rootbreaker Wurm");
    (engine, elephant, wurm)
}

/// Chapter I mills five cards including a 4-power (Fangren Hunter) and a
/// 2-power (Giant Spider) creature card. The reflexive triggered ability
/// triggers, asks for a target creature, and deals 4 damage (the greatest
/// power among creature cards in your graveyard), killing the 3/3.
#[test]
fn kraven_s_last_hunt_chapter_one_kills_smaller_creature_with_greatest_graveyard_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, elephant, wurm) = setup_kraven_board(&[giant_spider(), fangren_hunter()]);
    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, kraven_s_last_hunt());
    let options = pass_until_targets(&mut engine, p0);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 5,
        "chapter I milled five cards before the reflexive trigger chose its target"
    );
    assert!(
        in_graveyard(&engine, p0, fangren_hunter()).is_some()
            && in_graveyard(&engine, p0, giant_spider()).is_some(),
        "both stacked creatures were milled into seat 0's graveyard"
    );
    assert!(
        options.contains(&elephant) && options.contains(&wurm),
        "reflexive trigger offers both opponent creatures as legal targets: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elephant],
                players: vec![],
            },
        )
        .expect("aim reflexive damage at the 3/3");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, wild_elephant()).is_none()
            && in_graveyard(&engine, p1, wild_elephant()).is_some(),
        "4 damage from Fangren Hunter's power kills the 3/3 Wild Elephant"
    );
    assert_eq!(
        engine
            .state()
            .object(wurm)
            .expect("6/6 Wurm remains on battlefield")
            .damage,
        0,
        "untargeted 6/6 Wurm takes no damage"
    );
}

/// Aiming chapter I's reflexive ability at a 6/6 deals exactly 4 damage (the
/// greatest power between the milled 4/4 and 2/4), so the 6/6 survives with 4
/// damage marked.
#[test]
fn kraven_s_last_hunt_chapter_one_marks_greatest_power_damage_on_larger_creature() {
    let p0 = PlayerId::new(0);
    let (mut engine, elephant, wurm) = setup_kraven_board(&[giant_spider(), fangren_hunter()]);

    cast_from_hand(&mut engine, p0, kraven_s_last_hunt());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&wurm));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![wurm],
                players: vec![],
            },
        )
        .expect("aim reflexive damage at the 6/6");
    pass_until(&mut engine, stack_is_empty);

    let wurm_obj = engine.state().object(wurm).expect("6/6 Wurm survives");
    assert_eq!(wurm_obj.zone, Zone::Battlefield);
    assert_eq!(
        wurm_obj.damage, 4,
        "greatest power among milled creature cards (4 vs 2) marks 4 damage"
    );
    assert_eq!(
        engine
            .state()
            .object(elephant)
            .expect("3/3 untouched")
            .damage,
        0
    );
}

/// Milling five noncreature cards still satisfies "Mill five cards. When you
/// do" and deals 0 damage; milling fewer than five cards from a short library
/// (CR 701.13b) does not arm the reflexive trigger.
#[test]
fn kraven_s_last_hunt_chapter_one_with_zero_creatures_or_short_library() {
    let p0 = PlayerId::new(0);

    // 1. Five milled lands, no creature cards in graveyard: reflexive trigger
    // still triggers and asks for a target, then deals 0 damage.
    let (mut engine, elephant, _wurm) = setup_kraven_board(&[]);
    cast_from_hand(&mut engine, p0, kraven_s_last_hunt());
    let options = pass_until_targets(&mut engine, p0);
    assert!(options.contains(&elephant));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elephant],
                players: vec![],
            },
        )
        .expect("target 3/3 with 0-damage reflexive ability");
    pass_until(&mut engine, stack_is_empty);
    let elephant_obj = engine
        .state()
        .object(elephant)
        .expect("3/3 survives 0 damage");
    assert_eq!(elephant_obj.zone, Zone::Battlefield);
    assert_eq!(elephant_obj.damage, 0);

    // 2. Only three cards in library (fewer than five): chapter I mills all
    // three (CR 701.13b) and the reflexive trigger does not arm.
    let (mut short, elephant, wurm) = setup_kraven_board(&[fangren_hunter()]);
    {
        let state = short.dev_state_mut(p0).expect("test harness has dev state");
        let extra: Vec<ObjectId> = state
            .zones
            .list(ZoneLocation::Library(p0))
            .iter()
            .copied()
            .rev()
            .skip(3)
            .collect();
        for card in extra {
            state
                .move_object(
                    card,
                    ZoneLocation::Exile(p0),
                    baylee_engine::zone::ZonePosition::Top,
                    baylee_engine::event::Cause::Effect,
                )
                .expect("trim library to 3 cards");
        }
    }
    short.refresh_offer();
    assert_eq!(library_size(&short, p0), 3);

    cast_from_hand(&mut short, p0, kraven_s_last_hunt());
    pass_until(&mut short, stack_is_empty);
    assert_eq!(
        library_size(&short, p0),
        0,
        "CR 701.13b: milled all three available cards"
    );
    assert!(
        in_graveyard(&short, p0, fangren_hunter()).is_some(),
        "the 4/4 was milled into the graveyard"
    );
    assert_eq!(
        short
            .state()
            .object(elephant)
            .expect("3/3 untouched")
            .damage,
        0
    );
    assert_eq!(short.state().object(wurm).expect("6/6 untouched").damage, 0);
}
