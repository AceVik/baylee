//! `cards/enchantments/mv_1/omen_of_the_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn omen_of_the_dead() -> CardIndex {
    card_index("0aa9ee99-69a8-452b-a0d5-001384bdff3c")
}

/// "Flash"; "When this enchantment enters, return target creature card from
/// your graveyard to your hand."; "{2}{B}, Sacrifice this enchantment: Scry 2."
#[test]
fn omen_of_the_dead_returns_a_creature_card_when_it_enters() {
    let mut e = table(7301, &[omen_of_the_dead()], &[index::GRIZZLY_BEARS], &[]);
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    bury(&mut e, &[bears]);
    let gy = in_graveyard(&e, P0, index::GRIZZLY_BEARS).unwrap();
    fill_pool(&mut e);
    cast_with_floating(&mut e, P0, omen_of_the_dead());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    drive(&mut e, &[gy], &[], &[], true);
    pass_until(&mut e, stack_is_empty);
    assert!(in_hand(&e, P0, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&e, P0, omen_of_the_dead()).is_some());

    fill_pool(&mut e);
    activate(&mut e, P0, omen_of_the_dead(), 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::Arrange { .. }) || stack_is_empty(e)
    });
    assert!(in_graveyard(&e, P0, omen_of_the_dead()).is_some());
    assert!(matches!(e.pending(), Pending::Arrange { player, .. } if *player == P0));
}

/// Flash: it can be cast on the opponent's turn.
#[test]
fn omen_of_the_dead_can_be_cast_on_the_opponents_turn() {
    let mut e = table(7302, &[omen_of_the_dead()], &[], &[]);
    pass_until(&mut e, |e| {
        e.state().turn.active == P1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    fill_pool(&mut e);
    cast_with_floating(&mut e, P0, omen_of_the_dead());
    pass_until(&mut e, |e| {
        on_battlefield(e, P0, omen_of_the_dead()).is_some()
    });
    assert!(on_battlefield(&e, P0, omen_of_the_dead()).is_some());
}
