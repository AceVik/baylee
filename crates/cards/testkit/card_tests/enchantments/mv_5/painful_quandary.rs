//! `cards/enchantments/mv_5/painful_quandary.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn painful_quandary() -> CardIndex {
    card_index("c37051cc-6683-4dbb-b5ff-5c3a5bdab1df")
}

/// The opponent casts a Gray Ogre and is asked, on the trigger, which card
/// (up to one) to discard; the answer is theirs.
fn their_spell_asks(answer: impl Fn(&[ObjectId]) -> Vec<ObjectId>) -> Engine<RegistryLookup> {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[painful_quandary()])
        .hand(1, &[gray_ogre(), plains()])
        .battlefield(1, &[mountain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, gray_ogre());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(player, p1, "the caster answers");
    assert_eq!((min, max), (0, 1), "discard a card, or not");
    assert!(matches!(prompt, ChoicePrompt::CostDiscard));
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: answer(&options),
            },
        )
        .expect("the answer came out of the question");
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p1, gray_ogre()).is_some()
    });
    engine
}

/// "Whenever an opponent casts a spell, that player loses 5 life unless
/// they discard a card." Declining the discard costs 5 life.
#[test]
fn painful_quandary_costs_five_life_when_the_discard_is_declined() {
    let p1 = PlayerId::new(1);
    let engine = their_spell_asks(|_| vec![]);
    assert_eq!(engine.state().players[1].life, 15);
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .is_empty(),
        "nothing was discarded"
    );
}

/// Discarding a card instead keeps the life total.
#[test]
fn painful_quandary_costs_a_card_when_it_is_discarded() {
    let p1 = PlayerId::new(1);
    let engine = their_spell_asks(|options| vec![options[0]]);
    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p1)).len(),
        1,
        "the discarded card"
    );
}

/// "…an opponent": my own spell asks me nothing.
#[test]
fn painful_quandary_ignores_its_controllers_spells() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[painful_quandary(), mountain(), mountain(), mountain()])
        .hand(0, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, gray_ogre());
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, gray_ogre()).is_some()
    });
    assert_eq!(engine.state().players[0].life, 20);
}
