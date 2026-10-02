//! Independent Alpha sorceries review.

#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

#[test]
fn drain_life_independent_creature_cap_reads_toughness_after_damage_prevention() {
    let p0 = PlayerId::new(0);
    let hydra_card = card_index("aff84707-f5f8-4f53-869e-feec78da8d8d");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[hydra_card, drain_life()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, hydra_card);
    assert!(matches!(engine.pending(), Pending::ChooseNumber { max, .. } if *max >= 3));
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    let hydra = on_battlefield(&engine, p0, hydra_card).unwrap();
    assert_eq!(pt(&engine, hydra), (3, 3));

    let before = engine.journal().entries().len();
    announce_drain(&mut engine, p0, 4, hydra);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        engine.journal().entries()[before..].iter().any(|entry| {
            matches!(
                entry.event,
                crate::event::GameEvent::DamageDealt {
                    target: crate::event::DamageTarget::Object(target),
                    amount: 1,
                    ..
                } if target == hydra
            )
        }),
        "three counters prevent three damage; the fourth damage is dealt"
    );
    assert!(in_graveyard(&engine, p0, hydra_card).is_some());
    // CR 608.2c, 608.2h, 613.5: the life-gain instruction reads the
    // creature's current toughness, after prevention removed its counters.
    assert_eq!(
        engine.state().players[0].life,
        20,
        "zero toughness after damage caps the life gain at zero"
    );
}

#[test]
fn drain_life_independent_previously_damaged_creature_uses_full_toughness_cap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let sorcerer = card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe");
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                sorcerer,
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .hand(0, &[drain_life()])
        .battlefield(1, &[body])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, body).unwrap();
    activate(&mut engine, p0, sorcerer, 0);
    aim(&mut engine, vec![target], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(target).unwrap().damage, 1);
    announce_drain(&mut engine, p0, 5, target);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        23,
        "cap is full three toughness, not remaining two lethal damage"
    );
}

#[test]
fn drain_life_independent_player_cap_uses_life_before_lethal_damage_at_a_three_player_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::table(SEED, swamp(), 3)
        .battlefield(0, &[swamp(); 7])
        .hand(0, &[drain_life()])
        .life(1, 2)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, drain_life());
    let Pending::ChooseNumber { player, .. } = engine.pending().clone() else {
        panic!("X expected");
    };
    engine.apply(player, PlayerAction::ChooseNumber(5)).unwrap();
    aim(&mut engine, vec![], vec![p1]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        22,
        "gain capped by two life before lethal five damage"
    );
    assert_eq!(
        engine.state().players[2].life,
        20,
        "third player was not targeted"
    );
}

#[test]
fn drain_life_independent_planeswalker_cap_uses_loyalty_before_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 7])
        .hand(0, &[drain_life()])
        .battlefield(1, &[jace_the_mind_sculptor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, jace_the_mind_sculptor()).unwrap();
    assert_eq!(
        engine
            .state()
            .object(target)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        3
    );
    announce_drain(&mut engine, p0, 5, target);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        23,
        "gain capped by starting loyalty, not post-damage zero"
    );
    assert!(in_graveyard(&engine, p1, jace_the_mind_sculptor()).is_some());
}

#[test]
fn drain_life_independent_gain_is_capped_by_creature_toughness() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 7])
        .battlefield(1, &[body])
        .hand(0, &[drain_life()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, body).unwrap();
    assert_eq!(pt(&engine, target), (2, 3));
    announce_drain(&mut engine, p0, 5, target);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, body).is_some(),
        "five damage kills a 2/3"
    );
    assert_eq!(
        engine.state().players[0].life,
        23,
        "gain capped at three toughness"
    );
}

#[test]
fn drain_life_independent_x_menu_excludes_nonblack_mana_but_base_can_use_it() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), mountain(), mountain(), mountain()])
        .hand(0, &[drain_life()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, drain_life());
    let Pending::ChooseNumber { player, max, .. } = engine.pending().clone() else {
        panic!("X menu expected: {:?}", engine.pending());
    };
    assert_eq!(
        max, 1,
        "one black pays B; only one remaining black can pay X"
    );
    engine.apply(player, PlayerAction::ChooseNumber(1)).unwrap();
    aim(&mut engine, vec![], vec![PlayerId::new(1)]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        0
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "nonblack mana pays generic base, never X"
    );
}

#[test]
fn drain_life_independent_prevented_damage_does_not_gain_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let healer = card_index("95a0ca48-d924-47f4-86ed-42c673ee778c");
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[healer, swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[body])
        .hand(0, &[drain_life()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, body).unwrap();
    activate(&mut engine, p0, healer, 0);
    aim(&mut engine, vec![target], vec![]);
    pass_until(&mut engine, stack_is_empty);
    announce_drain(&mut engine, p0, 2, target);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(target).unwrap().damage,
        1,
        "shield prevents one of the two damage"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "gain follows actual damage, not chosen X"
    );
}

#[test]
fn drain_life_independent_illegal_target_neither_deals_damage_nor_gains_life() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let body = card_index("8f1dae40-b307-446e-bbd2-86aa35813871");
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 7])
        .hand(0, &[drain_life()])
        .battlefield(1, &[body, island()])
        .hand(1, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let target = on_battlefield(&engine, p1, body).unwrap();
    announce_drain(&mut engine, p0, 5, target);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, p1, unsummon());
    aim(&mut engine, vec![target], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p1, body).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
    assert!(in_graveyard(&engine, p0, drain_life()).is_some());
}
