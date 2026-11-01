//! `cards/instants/mv_1/death_spark.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn death_spark() -> CardIndex {
    card_index("4b23a89c-89ba-41ad-b894-ec02221689eb")
}

fn mons_s_goblin_raiders() -> CardIndex {
    card_index("a37159df-f6d7-4db6-85de-0ea77f425993")
}

/// Player 0, with two Mountains, casts Death Spark at player 1's face (it lands in the graveyard),
/// and then the cards in `above` are put into player 0's graveyard on top of it, first to last, so
/// the last one is the top card. Returns at player 0's first main phase.
fn spark_buried_under(above: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut hand = vec![death_spark()];
    hand.extend_from_slice(above);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, death_spark());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, death_spark()).is_some());

    for card in above {
        let id = in_hand(&engine, p0, *card).expect("the card is in hand");
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
    engine
}

/// Passes to `seat`'s first main phase, stopping early (and failing) if any yes/no question is
/// asked on the way.
fn pass_to_main_unasked(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    pass_until(engine, |e| {
        (e.state().turn.phase == Phase::FirstMain && e.state().turn.active == seat)
            || matches!(e.pending(), Pending::YesNo { .. })
    });
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "no question on the way to {seat:?}'s main phase"
    );
}

/// `Death Spark` ({R}, Instant, `Coverage::Partial`): "deals 1 damage to any target" — a player...
#[test]
fn death_spark_deals_one_damage_to_a_player() {
    let engine = spark_buried_under(&[]);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(engine.state().players[0].life, 20, "only the target");
}

/// ... or a creature, and then nothing else is hit.
#[test]
fn death_spark_deals_one_damage_to_a_creature_and_not_to_the_player() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[death_spark()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, death_spark());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(ogre).unwrap().damage, 1);
    assert_eq!(engine.state().players[1].life, 20);
}

/// "At the beginning of your upkeep, if this card is in your graveyard with a creature card
/// directly above it, you may pay {1}. If you do, return this card to your hand." The creature is
/// the only card above it here. Nothing is asked in the opponent's upkeep.
#[test]
fn death_spark_returns_for_one_mana_when_a_creature_card_is_directly_above_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = spark_buried_under(&[mons_s_goblin_raiders()]);
    let spark = in_graveyard(&engine, p0, death_spark()).unwrap();

    // The opponent's upkeep passes without a question (the harness refuses an unexpected one).
    pass_to_main_unasked(&mut engine, p1);
    assert_eq!(engine.state().object(spark).unwrap().zone, Zone::Graveyard);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        engine.state().turn.active,
        p0,
        "asked in its owner's upkeep"
    );
    assert_eq!(engine.state().turn.step, Step::Upkeep);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(spark).unwrap().zone, Zone::Hand);
}

/// "You may pay {1}": declining leaves it in the graveyard.
#[test]
fn death_spark_stays_in_the_graveyard_when_the_mana_is_declined() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = spark_buried_under(&[mons_s_goblin_raiders()]);
    let spark = in_graveyard(&engine, p0, death_spark()).unwrap();

    pass_to_main_unasked(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(spark).unwrap().zone, Zone::Graveyard);
}

/// "With a creature card directly above it": with no card above it, with only a noncreature above
/// it, and with a creature higher up but a noncreature directly on top of it, nothing is asked.
#[test]
fn death_spark_does_not_return_without_a_creature_card_directly_above_it() {
    let p0 = PlayerId::new(0);
    let layouts: [&[CardIndex]; 3] = [
        &[],
        &[lightning_bolt()],
        // Bottom to top: the Bolt sits directly above Death Spark, the creature above the Bolt.
        &[lightning_bolt(), mons_s_goblin_raiders()],
    ];
    for above in layouts {
        let mut engine = spark_buried_under(above);
        // Through the opponent's turn and into our own main phase; an upkeep question would make
        // the harness stop with an unexpected pending.
        pass_to_main_unasked(&mut engine, PlayerId::new(1));
        pass_to_main_unasked(&mut engine, p0);
        assert!(
            in_graveyard(&engine, p0, death_spark()).is_some(),
            "layout {above:?}: no return"
        );
        assert!(in_hand(&engine, p0, death_spark()).is_none());
    }
}

/// Known gap, pinned: the creature IS directly above Death Spark (the Bolt is above the creature),
/// so the printed text returns it, but the card only recognises a creature as the single card
/// above it. When a condition can say "directly above", this test must move to the returning side.
#[test]
fn death_spark_misses_a_creature_directly_above_it_with_other_cards_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = spark_buried_under(&[mons_s_goblin_raiders(), lightning_bolt()]);
    pass_to_main_unasked(&mut engine, PlayerId::new(1));
    pass_to_main_unasked(&mut engine, p0);
    assert!(in_graveyard(&engine, p0, death_spark()).is_some());
}
