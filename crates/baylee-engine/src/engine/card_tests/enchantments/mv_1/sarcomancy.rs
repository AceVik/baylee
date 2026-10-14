//! `cards/enchantments/mv_1/sarcomancy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sarcomancy() -> CardIndex {
    card_index("bd1e93da-7524-4bdd-9738-accd76bda7f7")
}

/// "When this enchantment enters, create a 2/2 black Zombie creature token."
/// "At the beginning of your upkeep, if there are no Zombies on the
/// battlefield, this enchantment deals 1 damage to you."
#[test]
fn sarcomancy_makes_a_zombie_and_hurts_only_when_none_is_left() {
    let mut e = table(7331, &[sarcomancy()], &[], &[]);
    cast_with_floating(&mut e, P0, sarcomancy());
    pass_until(&mut e, stack_is_empty);
    let tokens = tokens_of(&e, P0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&e, tokens[0]), (2, 2));
    // With the Zombie out, the next upkeep costs nothing.
    next_main(&mut e, P0);
    assert_eq!(life(&e, P0), 20);
    // Without it, the following one costs 1.
    let zombies = tokens_of(&e, P0);
    bury(&mut e, &zombies);
    next_main(&mut e, P0);
    assert_eq!(life(&e, P0), 19);
}

/// Seat `p`'s next first main phase.
fn next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state().turn.number;
    pass_until(e, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p
            && e.state().turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p)
    });
}
