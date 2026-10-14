//! `cards/enchantments/mv_1/graf_harvest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn graf_harvest() -> CardIndex {
    card_index("d3ba6922-c2f7-45ab-87a3-d4bbd770d1ba")
}

/// "{3}{B}, Exile a creature card from your graveyard: Create a 2/2 black
/// Zombie creature token." and "Zombies you control have menace."
#[test]
fn graf_harvest_exiles_a_creature_card_for_a_zombie_with_menace() {
    let mut e = table(7191, &[], &[graf_harvest(), index::GRIZZLY_BEARS], &[]);
    let bears = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    assert!(
        !keywords(&e, bears).contains(KeywordSet::MENACE),
        "not a Zombie"
    );
    bury(&mut e, &[bears]);
    let card = in_graveyard(&e, P0, index::GRIZZLY_BEARS).unwrap();
    fill_pool(&mut e);
    activate(&mut e, P0, graf_harvest(), 1);
    drive(&mut e, &[], &[], &[card], true);
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P0, index::GRIZZLY_BEARS).is_none(),
        "exiled"
    );
    let tokens = tokens_of(&e, P0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&e, tokens[0]), (2, 2));
    assert!(keywords(&e, tokens[0]).contains(KeywordSet::MENACE));
}

/// Without a creature card in the graveyard the ability is not offered.
#[test]
fn graf_harvest_needs_a_creature_card_to_exile() {
    let e = table(7192, &[], &[graf_harvest()], &[]);
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority expected")
    };
    assert!(legal.abilities.is_empty(), "{:?}", legal.abilities);
}
