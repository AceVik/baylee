//! `cards/enchantments/auras/mv_2/phantom_wings.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn phantom_wings() -> CardIndex {
    card_index("9da74f0f-143d-4db2-81eb-c7873744a2d0")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Phantom Wings (9da74f0f-143d-4db2-81eb-c7873744a2d0) — "Enchant creature.
/// Enchanted creature has flying." Cast on the Bears: it flies and the Lions
/// beside it do not; the Aura is attached to the Bears. The printed
/// "Sacrifice this Aura: Return enchanted creature to its owner's hand" is
/// not on the card, so no ability of the Aura is offered.
#[test]
fn phantom_wings_gives_the_enchanted_creature_flying_and_no_one_else() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), savannah_lions()],
        &[],
        &[phantom_wings()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let lions = lk::obj(&e, P0, savannah_lions());
    assert!(!keywords(&e, bears).contains(KeywordSet::FLYING));

    let aura = kit::cast_aura(
        &mut e,
        P0,
        phantom_wings(),
        bears,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
    );

    assert_eq!(kit::attached_to(&e, aura), Some(bears));
    assert!(
        keywords(&e, bears).contains(KeywordSet::FLYING),
        "the enchanted creature flies"
    );
    assert!(
        !keywords(&e, lions).contains(KeywordSet::FLYING),
        "the other creature does not"
    );
    for index in 0..3 {
        assert!(
            !lk::offered(&e, phantom_wings(), P0, index),
            "the Aura offers no ability (index {index}): the sacrifice ability is not supported"
        );
    }
}

/// Phantom Wings — "Enchant creature", not "creature you control": it may go on
/// the opponent's Bears, which then flies while the caster's own Bears does not.
#[test]
fn phantom_wings_may_enchant_an_opposing_creature() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[grizzly_bears()],
        &[phantom_wings()],
        &[],
    );
    let mine = lk::obj(&e, P0, grizzly_bears());
    let theirs = lk::obj(&e, P1, grizzly_bears());

    let aura = kit::cast_aura(
        &mut e,
        P0,
        phantom_wings(),
        theirs,
        &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
    );

    assert_eq!(kit::attached_to(&e, aura), Some(theirs));
    assert!(keywords(&e, theirs).contains(KeywordSet::FLYING));
    assert!(
        !keywords(&e, mine).contains(KeywordSet::FLYING),
        "the caster's own Bears stays on the ground"
    );
}
