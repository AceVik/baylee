//! `cards/enchantments/mv_4/grasping_shadows.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Grasping Shadows` // `Shadows' Lair` (`Coverage::Partial`):
/// "Whenever a creature you control attacks alone, it gains deathtouch and lifelink until
/// end of turn. Put a dread counter on this enchantment. Then if there are three or more dread
/// counters on it, transform it. // `{{T}}`: Add `{{B}}`. `{{B}}`, `{{T}}`, Remove a dread counter
/// from this land: You draw a card and you lose 1 life."
///
/// The lone-attacker trigger is written up to the transform (#206). Cast from hand, the
/// enchantment sees a lone Llanowar Elves attack: the Elf — the event object, not a chosen
/// target — gains deathtouch and lifelink, and the dread counter lands on the enchantment and
/// not on the Elf, which is the one place the two halves of the sentence could be confused
/// (`Effect::AddCounter` follows the event object; `AddCounterFilter` over `Filter::This`
/// finds the source). The card stays on its front face.
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
        "no transform in place (#206): Grasping Shadows stays on face 0"
    );
}

/// `Grasping Shadows`: two attackers are not one attacking alone (CR 506.5), so neither
/// gains anything and no dread counter is put — the intervening `if` is asked after the
/// whole declaration, not as each attacker is added to it.
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

/// `Shadows' Lair`, reached by turning the card over on the board — the one door a
/// transforming land face has while nothing transforms it in play (#206) — with one dread
/// counter on it, as the front face would have left it. `{{B}}`, `{{T}}`, remove the counter:
/// draw a card and lose 1 life. The counter is part of the cost, so the ability leaves none.
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
