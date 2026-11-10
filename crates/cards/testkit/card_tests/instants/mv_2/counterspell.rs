//! `cards/instants/mv_2/counterspell.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Counterspell: the classic — p0's creature spell never arrives.
#[test]
fn counterspell_counters_a_creature_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(12, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[ondu_cleric()])
        .battlefield(1, &[island(), island()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let cleric = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card: cleric })
        .unwrap();

    // p0 passes; p1 taps both islands and counters the cleric.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1 priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p1, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let cs = engine.state().zones.list(ZoneLocation::Hand(p1))[0];
    engine
        .apply(p1, PlayerAction::CastSpell { card: cs })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending())
    };
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();

    // After both pass, the cleric is in the graveyard, not on the board.
    pass_until(&mut engine, |e| {
        e.state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| {
                e.state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == ondu_cleric()))
            })
    });
    assert!(on_battlefield(&engine, p0, ondu_cleric()).is_none());
}

/// CR 601.2c: a spell whose mandatory target has no legal choice cannot be
/// cast at all, so it must not be offered. Counterspell with an empty stack
/// is the clean case — offering it hands a human a button that only errors,
/// and an agent an action it will pick again on every pass, because failing
/// changes nothing about the state.
#[test]
fn a_spell_with_no_legal_target_is_not_offered() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(51, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.is_empty(),
        "counterspell was offered with nothing on the stack to counter"
    );
}

/// Abrupt Decay: "This spell can't be countered. Destroy target nonland
/// permanent with mana value 3 or less."
///
/// Both printed sentences are read off one board, and each is read against
/// something that *is* allowed, so neither half can pass by being empty.
///
/// The cap is pinned from both sides in one target list: Skyclave
/// Apparition costs exactly three and is offered, Karn costs exactly four
/// and is not, and not one of the four lands on the table is a nonland
/// permanent. A filter one off in either direction, or one that dropped the
/// word "nonland", moves that list.
///
/// The counter half is the Gatherer ruling (2021-03-19): "A spell or ability
/// that counters spells can still target Abrupt Decay. When that spell or
/// ability resolves, Abrupt Decay won't be countered." So p1's Counterspell
/// is offered, points at the Decay, and resolves into the graveyard with the
/// Decay still on the stack under it. This test used to pin the opposite,
/// that the Decay was no target at all, which is how the engine read the
/// keyword until #243.
#[test]
#[allow(clippy::too_many_lines)] // one game, played from the cast to the assertion
fn abrupt_decay_destroys_a_small_permanent_and_the_counterspell_pointed_at_it_does_not_counter_it()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, forest())
        .battlefield(0, &[swamp(), forest()])
        .hand(0, &[abrupt_decay()])
        .battlefield(
            1,
            &[
                island(),
                island(),
                llanowar_elves(),
                skyclave_apparition(),
                karn_the_great_creator(),
            ],
        )
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let victim = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is deployed");
    let apparition =
        on_battlefield(&engine, p1, skyclave_apparition()).expect("the Apparition is deployed");
    let karn = on_battlefield(&engine, p1, karn_the_great_creator()).expect("Karn is deployed");

    tap_all_mana(&mut engine, p0);
    let decay = in_hand(&engine, p0, abrupt_decay()).expect("the Decay is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: decay })
        .expect("a Swamp and a Forest pay {B}{G}");

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("the Decay asks for a target, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "their spell, their target");
    assert_eq!(
        (min, max),
        (1, 1),
        "\"target nonland permanent\" is one target, and not an optional one"
    );
    assert!(
        options.contains(&victim),
        "a one-mana creature is a nonland permanent with mana value 3 or less"
    );
    assert!(
        options.contains(&apparition),
        "the Apparition costs three, and \"3 or less\" includes three"
    );
    assert!(!options.contains(&karn), "Karn costs four, which is more");
    assert_eq!(
        options.len(),
        2,
        "and the four lands on the table are not nonland permanents: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("the Decay points at the Elf");

    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected p1's priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p1);
    let counter = in_hand(&engine, p1, counterspell()).expect("the Counterspell is in hand");
    assert!(
        legal.castable.contains(&counter),
        "two Islands, and a spell up there to point at: the counter is offered"
    );
    engine
        .apply(p1, PlayerAction::CastSpell { card: counter })
        .expect("two Islands pay {U}{U}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the Counterspell asks for a target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![decay],
        "\"This spell can't be countered\" does not take the Decay off the list"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![decay],
            },
        )
        .expect("the Counterspell points at the Decay");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, counterspell()).is_some()
    });
    assert!(
        on_stack(&engine, abrupt_decay()).is_some(),
        "the Counterspell resolved and the Decay is still on the stack under it"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "\"Destroy target nonland permanent\": the Elf is off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "and a destroyed permanent is its owner's card in their graveyard"
    );
    assert!(
        in_graveyard(&engine, p0, abrupt_decay()).is_some(),
        "the Decay resolved, and a resolved instant is a card in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, skyclave_apparition()).is_some(),
        "the Apparition was offered and not chosen, so it is still there"
    );
    assert!(
        on_battlefield(&engine, p1, karn_the_great_creator()).is_some(),
        "and Karn was never on offer at all"
    );
}
