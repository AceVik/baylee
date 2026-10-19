//! `cards/enchantments/mv_5/psychic_allergy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn psychic_allergy() -> CardIndex {
    card_index("58e0cc31-7f7c-4cea-9453-d377a039da36")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn elves() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}

/// p0 casts Psychic Allergy naming `chosen` against an opponent who controls
/// green Bears, green Elves and a red Giant, and the game runs to the
/// opponent's first main phase. Returns the opponent's life.
#[track_caller]
fn opponents_life_after_their_upkeep(chosen: ManaColor) -> i32 {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[psychic_allergy()])
        .battlefield(1, &[grizzly_bears(), elves(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, psychic_allergy());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("the pass stopped on a colour question")
    };
    assert_eq!(player, p0, "its controller chooses as it enters");
    assert!(
        options.contains(&chosen),
        "{chosen:?} is on offer: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(chosen))
        .expect("the colour is on offer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, psychic_allergy()).is_some(),
        "Psychic Allergy resolved"
    );

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the Allergy never hurts me"
    );
    engine.state().players[1].life
}

/// Psychic Allergy (58e0cc31-7f7c-4cea-9453-d377a039da36) — "As this
/// enchantment enters, choose a color. At the beginning of each opponent's
/// upkeep, this enchantment deals X damage to that player, where X is the
/// number of nontoken permanents of the chosen color they control."
///
/// Green: the Bears and the Elves.
#[test]
fn psychic_allergy_deals_one_damage_per_nontoken_permanent_of_the_chosen_color() {
    assert_eq!(opponents_life_after_their_upkeep(ManaColor::Green), 18);
}

/// The chosen colour is what is counted: red finds only the Giant.
#[test]
fn psychic_allergy_counts_only_the_color_it_was_given() {
    assert_eq!(opponents_life_after_their_upkeep(ManaColor::Red), 19);
}

/// A colour the opponent has none of costs them nothing.
#[test]
fn psychic_allergy_naming_an_absent_color_deals_nothing() {
    assert_eq!(opponents_life_after_their_upkeep(ManaColor::White), 20);
}

/// p0 starts with the Allergy and `islands` Islands out, and the game runs to
/// the first thing the Allergy's upkeep trigger asks.
fn my_upkeep(islands: usize) -> Engine<RegistryLookup> {
    let mut mine = vec![psychic_allergy()];
    mine.extend(std::iter::repeat_n(island(), islands));
    let mut engine = Duel::new(SEED, island()).battlefield(0, &mine).start();
    keep_mulligans(&mut engine);
    // A permanent placed on the board still asks its "as this enters"
    // colour; the upkeep half does not care which.
    let Pending::ChooseColor { player, .. } = engine.pending().clone() else {
        panic!(
            "the Allergy asks its colour first, got {:?}",
            engine.pending()
        );
    };
    engine
        .apply(player, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("a colour is chosen");
    engine
}

/// "At the beginning of your upkeep, destroy this enchantment unless you
/// sacrifice two Islands": paying keeps it and costs two of my three Islands.
#[test]
fn psychic_allergy_stays_when_two_islands_are_sacrificed_in_my_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = my_upkeep(3);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped on a yes/no question")
    };
    assert_eq!(player, p0, "my upkeep, my choice");
    engine.apply(p0, PlayerAction::YesNo(true)).expect("I pay");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected the Islands to sacrifice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (2, 2), "exactly two Islands");
    assert_eq!(options.len(), 3, "any of my Islands, and nothing else");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options[..2].to_vec(),
            },
        )
        .expect("two of the offered Islands");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, psychic_allergy()).is_some(),
        "paid for, the Allergy stays"
    );
    assert_eq!(all_on_battlefield(&engine, p0, island()).len(), 1);
}

/// Declined, the Allergy is destroyed and the Islands are kept.
#[test]
fn psychic_allergy_is_destroyed_when_the_sacrifice_is_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = my_upkeep(3);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("I decline");
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, psychic_allergy()).is_none());
    assert!(
        in_graveyard(&engine, p0, psychic_allergy()).is_some(),
        "destroyed"
    );
    assert_eq!(all_on_battlefield(&engine, p0, island()).len(), 3);
}

/// With fewer than two Islands there is nothing to pay with, so the Allergy
/// goes without a question and the lone Island is kept.
#[test]
fn psychic_allergy_is_destroyed_without_asking_when_there_are_not_two_islands() {
    let p0 = PlayerId::new(0);
    let mut engine = my_upkeep(1);
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, psychic_allergy()).is_some()
    });
    assert_eq!(all_on_battlefield(&engine, p0, island()).len(), 1);
}
