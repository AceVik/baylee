//! `cards/creatures/mv_2/frankenstein_s_monster.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn fill_pools(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

/// A game with the Monster in hand and `graveyard` already in player 0's
/// graveyard.
fn table(graveyard: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[card_index("100218ce-1572-4bce-bb00-d2df5f68705c")])
        .battlefield(0, graveyard)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    // The same card may appear twice; take each id once.
    let ids: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| e.state().object(*id).is_some_and(|o| o.controller == P0))
        .collect();
    bury(&mut e, &ids);
    e.refresh_offer();
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        graveyard.len()
    );
    e
}

fn cast_for_x(e: &mut Engine<RegistryLookup>, x: u32) {
    fill_pools(e);
    cast_with_floating(e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c"));
    e.apply(P0, PlayerAction::ChooseNumber(x))
        .expect("X is chosen");
}

fn exiled(e: &Engine<RegistryLookup>) -> usize {
    e.state().zones.list(ZoneLocation::Exile(P0)).len()
}

fn exile_from_graveyard(e: &mut Engine<RegistryLookup>, cards: &[CardIndex]) {
    pass_until(e, |e| matches!(e.pending(), Pending::ChooseCards { .. }));
    let objects: Vec<ObjectId> = cards
        .iter()
        .map(|c| in_graveyard(e, P0, *c).expect("the card is in the graveyard"))
        .collect();
    let Pending::ChooseCards { options, .. } = e.pending().clone() else {
        unreachable!("pass_until stopped on the choice")
    };
    for o in &objects {
        assert!(options.contains(o), "a creature card is on offer");
    }
    e.apply(P0, PlayerAction::ChooseObjects { objects })
        .expect("the creature cards are exiled");
}

fn pick_counter(e: &mut Engine<RegistryLookup>, which: u32) {
    assert!(
        matches!(e.pending(), Pending::ChooseNumber { .. }),
        "a counter kind is asked: {:?}",
        e.pending()
    );
    e.apply(P0, PlayerAction::ChooseNumber(which))
        .expect("the kind is chosen");
}

/// Frankenstein's Monster (100218ce-1572-4bce-bb00-d2df5f68705c) — "As this
/// creature enters, exile X creature cards from your graveyard. If you
/// can't, put this creature into its owner's graveyard instead of onto the
/// battlefield. For each creature card exiled this way, this creature enters
/// with a +2/+0, +1/+1, or +0/+2 counter on it."
#[test]
fn frankenstein_s_monster_with_x_zero_enters_as_a_bare_zero_one() {
    let mut e = table(&[]);
    cast_for_x(&mut e, 0);
    pass_until(&mut e, stack_is_empty);
    let m = on_battlefield(&e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c"))
        .expect("it entered");
    assert_eq!(pt(&e, m), (0, 1));
    assert_eq!(exiled(&e), 0);
}

#[test]
fn frankenstein_s_monster_one_exiled_card_gives_the_chosen_counter() {
    // +1/+1 is the middle choice.
    let mut e = table(&[grizzly_bears()]);
    cast_for_x(&mut e, 1);
    exile_from_graveyard(&mut e, &[grizzly_bears()]);
    pick_counter(&mut e, 1);
    pass_until(&mut e, stack_is_empty);
    let m = on_battlefield(&e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c"))
        .expect("it entered");
    assert_eq!(pt(&e, m), (1, 2));
    assert_eq!(exiled(&e), 1);
}

#[test]
fn frankenstein_s_monster_each_exiled_card_chooses_its_own_counter() {
    let mut e = table(&[grizzly_bears(), savannah_lions()]);
    cast_for_x(&mut e, 2);
    exile_from_graveyard(&mut e, &[grizzly_bears(), savannah_lions()]);
    pick_counter(&mut e, 0); // +2/+0
    pick_counter(&mut e, 2); // +0/+2
    pass_until(&mut e, stack_is_empty);
    let m = on_battlefield(&e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c"))
        .expect("it entered");
    assert_eq!(pt(&e, m), (2, 3), "0/1 with +2/+0 and +0/+2");
    assert_eq!(exiled(&e), 2);
}

#[test]
fn frankenstein_s_monster_goes_to_the_graveyard_when_the_graveyard_is_short() {
    let mut e = table(&[grizzly_bears()]);
    cast_for_x(&mut e, 2);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c")).is_none());
    assert!(in_graveyard(&e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c")).is_some());
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "the lone creature card was not exiled"
    );
    assert_eq!(exiled(&e), 0);
}

#[test]
fn frankenstein_s_monster_does_not_count_noncreature_cards() {
    let mut e = table(&[grizzly_bears(), forest()]);
    cast_for_x(&mut e, 2);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c")).is_none(),
        "a land card cannot be the second creature card"
    );
    assert!(in_graveyard(&e, P0, card_index("100218ce-1572-4bce-bb00-d2df5f68705c")).is_some());
    assert_eq!(exiled(&e), 0);
}
