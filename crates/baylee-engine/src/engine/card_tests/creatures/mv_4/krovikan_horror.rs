//! Krovikan Horror — `{3}{B}` 2/2 Creature — Horror Spirit (Alliances)
//!
//! (`Coverage::Partial`: at the beginning of the end step, if it is in your graveyard with
//! exactly one card above it and that card is a creature card, you may return it to your hand;
//! a creature card directly above it with other cards on top of that is not recognised.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn krovikan_horror() -> CardIndex {
    card_index("a4c15223-784f-43a7-8626-30f96ccfdcaf")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Player 0 has the Horror, a Grizzly Bears, a Hill Giant and two Swamps on the battlefield. The
/// Horror is put into the graveyard, then the permanents named in `above` are put on top of it,
/// first to last (the last is the top card). Returns at player 0's first main phase.
fn horror_buried_under(above: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                krovikan_horror(),
                grizzly_bears(),
                hill_giant(),
                swamp(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    for card in std::iter::once(krovikan_horror()).chain(above.iter().copied()) {
        let id = on_battlefield(&engine, p0, card).expect("on the battlefield");
        engine
            .dev_state_mut(p0)
            .unwrap()
            .move_object(
                id,
                ZoneLocation::Graveyard(p0),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
    }
    engine.refresh_offer();
    engine
}

/// Passes to `seat`'s next first main phase, stopping early (and failing) if a yes/no question is
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

/// "{1}, Sacrifice a creature: This creature deals 1 damage to any target." Any creature of mine
/// may be sacrificed (not a land), the Horror stays when another is, and the damage goes to the
/// target alone.
#[test]
fn krovikan_horror_sacrifices_another_creature_for_one_damage() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[krovikan_horror(), grizzly_bears(), swamp(), swamp()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let horror = on_battlefield(&engine, p0, krovikan_horror()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let their_giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let land = on_battlefield(&engine, p0, swamp()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: horror,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_giant],
                players: vec![],
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the sacrifice question, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&bears) && options.contains(&horror));
    assert!(!options.contains(&land), "a land is not a creature");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(on_battlefield(&engine, p0, krovikan_horror()).is_some());
    assert_eq!(engine.state().object(their_giant).unwrap().damage, 1);
    assert_eq!(engine.state().players[1].life, 20, "the creature took it");
}

/// "At the beginning of the end step, if this card is in your graveyard with a creature card
/// directly above it, you may return this card to your hand": a creature card on top of it and
/// nothing else above it returns it, at the end of the turn.
#[test]
fn krovikan_horror_returns_to_hand_at_the_end_step_with_a_creature_directly_above_it() {
    let p0 = PlayerId::new(0);
    let mut engine = horror_buried_under(&[grizzly_bears()]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().turn.step, Step::End);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, krovikan_horror()).is_some());
    assert!(in_graveyard(&engine, p0, krovikan_horror()).is_none());
}

/// "You may": declining leaves it there.
#[test]
fn krovikan_horror_stays_in_the_graveyard_when_the_return_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = horror_buried_under(&[grizzly_bears()]);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, krovikan_horror()).is_some());
    assert!(in_hand(&engine, p0, krovikan_horror()).is_none());
}

/// With no card above it, only a land above it, and a creature higher up but a land directly on
/// top of it, no question is asked (the last is the case the old gate got wrong).
#[test]
fn krovikan_horror_does_not_return_without_a_creature_card_directly_above_it() {
    let p0 = PlayerId::new(0);
    let layouts: [&[CardIndex]; 3] = [&[], &[swamp()], &[swamp(), grizzly_bears()]];
    for above in layouts {
        let mut engine = horror_buried_under(above);
        pass_to_main_unasked(&mut engine, PlayerId::new(1));
        pass_to_main_unasked(&mut engine, p0);
        assert!(
            in_graveyard(&engine, p0, krovikan_horror()).is_some(),
            "layout {above:?}: stays"
        );
    }
}

/// Known gap, pinned: the creature IS directly above the Horror (the land is above the creature),
/// so the printed text returns it, but the card only recognises a creature as the single card
/// above it. When a condition can say "directly above", this test must move to the returning side.
#[test]
fn krovikan_horror_misses_a_creature_directly_above_it_with_other_cards_on_top() {
    let p0 = PlayerId::new(0);
    let mut engine = horror_buried_under(&[grizzly_bears(), swamp()]);
    pass_to_main_unasked(&mut engine, PlayerId::new(1));
    pass_to_main_unasked(&mut engine, p0);
    assert!(in_graveyard(&engine, p0, krovikan_horror()).is_some());
}
