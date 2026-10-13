//! `cards/enchantments/mv_4/grasping_shadows.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// An attack by one creature grants keywords to that creature and a dread counter to the enchantment.
#[test]
fn grasping_shadows_arms_a_lone_attacker_and_gathers_a_dread_counter() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(109, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[grasping_shadows()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves on battlefield");
    // Not `cast_from_hand`: it taps every mana source, and Llanowar Elves is
    // one — a tapped creature cannot be declared an attacker (CR 508.1a), so
    // the Elf has to be kept out of the payment it is not needed for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, grasping_shadows());
    pass_until(&mut engine, stack_is_empty);

    let shadows =
        on_battlefield(&engine, p0, grasping_shadows()).expect("Grasping Shadows on battlefield");
    assert!(
        types(&engine, shadows).contains(TypeSet::ENCHANTMENT),
        "Grasping Shadows is an enchantment"
    );
    assert_eq!(counters_on(&engine, shadows, counters::DREAD), 0);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(
        !stack_is_empty(&engine),
        "a lone attacker triggers the enchantment"
    );
    pass_until(&mut engine, stack_is_empty);

    let kw = keywords(&engine, elf);
    assert!(
        kw.contains(KeywordSet::DEATHTOUCH) && kw.contains(KeywordSet::LIFELINK),
        "the creature that attacked alone gains deathtouch and lifelink, got {kw:?}"
    );
    assert_eq!(
        counters_on(&engine, shadows, counters::DREAD),
        1,
        "the dread counter goes on the enchantment"
    );
    assert_eq!(
        counters_on(&engine, elf, counters::DREAD),
        0,
        "and not on the creature the rest of the sentence is about"
    );
    assert_eq!(
        engine.state().object(shadows).map(|o| o.face_index),
        Some(0),
        "one dread counter is below the transform threshold"
    );
}

/// `Grasping Shadows`: two attackers are not one attacking alone (CR 506.5), so neither
/// gains anything and no dread counter is put — the trigger checks the
/// whole declaration, not each attacker in isolation.
#[test]
fn grasping_shadows_ignores_an_attack_by_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(110, swamp())
        .battlefield(0, &[grasping_shadows(), llanowar_elves(), young_wolf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shadows =
        on_battlefield(&engine, p0, grasping_shadows()).expect("Grasping Shadows on battlefield");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Llanowar Elves");
    let wolf = on_battlefield(&engine, p0, young_wolf()).expect("Young Wolf");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(p1)), (wolf, Defender::Player(p1))],
            },
        )
        .unwrap();
    assert!(stack_is_empty(&engine), "two attackers trigger nothing");

    for creature in [elf, wolf] {
        let kw = keywords(&engine, creature);
        assert!(
            !kw.contains(KeywordSet::DEATHTOUCH) && !kw.contains(KeywordSet::LIFELINK),
            "neither attacker attacked alone, got {kw:?}"
        );
    }
    assert_eq!(counters_on(&engine, shadows, counters::DREAD), 0);
}

/// The back face spends its inherited dread counters as part of the draw cost.
#[test]
fn shadows_lair_spends_a_dread_counter_to_draw_and_lose_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(108, swamp())
        .battlefield(0, &[grasping_shadows(), swamp()])
        .start();
    let lair = on_battlefield(&engine, p0, grasping_shadows()).expect("the card is on the board");
    let def = baylee_cards::by_index(grasping_shadows()).expect("Grasping Shadows is in the pool");
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    assert!(
        state.transform(lair, def, 1),
        "turned over to Shadows' Lair"
    );
    crate::replacement::put_counters(state, lair, counters::DREAD, 1);
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        types(&engine, lair).contains(TypeSet::LAND),
        "Shadows' Lair is a land"
    );
    assert_eq!(counters_on(&engine, lair, counters::DREAD), 1);

    let life = engine.state().players[0].life;
    let library = library_size(&engine, p0);
    let hand = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .len();

    // The Swamp pays {B}; the Lair's own {T} is part of the cost.
    tap_all_mana_but(&mut engine, p0, Some(grasping_shadows()));
    activate(&mut engine, p0, grasping_shadows(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, lair, counters::DREAD),
        0,
        "the dread counter was the cost"
    );
    assert!(is_tapped(&engine, lair), "and so was the tap");
    assert_eq!(engine.state().players[0].life, life - 1, "you lose 1 life");
    assert_eq!(library_size(&engine, p0), library - 1, "you draw a card");
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Hand(p0))
            .len(),
        hand + 1,
        "into your hand"
    );
}

#[test]
fn third_lone_attack_transforms_in_place_even_if_the_attacker_leaves() {
    use crate::engine::card_tests::legends_kit as kit;
    for attacker_leaves in [false, true] {
        let mut e = kit::game(975, &[grasping_shadows(), llanowar_elves()], &[], &[], &[]);
        let shadows = kit::obj(&e, kit::P0, grasping_shadows());
        let elf = kit::obj(&e, kit::P0, llanowar_elves());
        let original = identity(&e, shadows);
        crate::replacement::put_counters(&mut e.state, shadows, counters::DREAD, 2);
        kit::attackers_offered(&mut e);
        e.apply(
            kit::P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(elf, Defender::Player(kit::P1))],
            },
        )
        .unwrap();
        if attacker_leaves {
            e.state
                .move_object(
                    elf,
                    ZoneLocation::Graveyard(kit::P0),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(identity(&e, shadows), original);
        assert_eq!(e.state.object(shadows).unwrap().face_index, 1);
        assert_eq!(counters_on(&e, shadows, counters::DREAD), 3);
        assert!(types(&e, shadows).contains(TypeSet::LAND));
    }
}

/// Shadows' Lair: "{T}: Add {B}." One black, no more, and the tap is the
/// cost.
#[test]
fn shadows_lair_taps_for_exactly_one_black() {
    let p0 = PlayerId::new(0);
    let (mut engine, lair) =
        placed_face(grasping_shadows(), 1, &[]).expect("Shadows' Lair on the battlefield");
    assert!(!is_tapped(&engine, lair));
    activate(&mut engine, p0, grasping_shadows(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{T}}: Add {{B}}");
    assert_eq!(pool.total(), 1, "and nothing else");
    assert!(is_tapped(&engine, lair), "{{T}} is part of the cost");
}

/// "Then if there are three or more dread counters on it, transform it":
/// the second lone attack leaves two counters and the enchantment as it is.
#[test]
fn second_lone_attack_adds_a_dread_counter_and_does_not_transform() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut e = kit::game(975, &[grasping_shadows(), llanowar_elves()], &[], &[], &[]);
    let shadows = kit::obj(&e, kit::P0, grasping_shadows());
    let elf = kit::obj(&e, kit::P0, llanowar_elves());
    crate::replacement::put_counters(&mut e.state, shadows, counters::DREAD, 1);
    kit::attackers_offered(&mut e);
    e.apply(
        kit::P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(elf, Defender::Player(kit::P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, shadows, counters::DREAD),
        2,
        "the lone attack put a dread counter on it"
    );
    assert_eq!(
        e.state.object(shadows).unwrap().face_index,
        0,
        "two dread counters are not three: no transform"
    );
    assert!(types(&e, shadows).contains(TypeSet::ENCHANTMENT));
}
