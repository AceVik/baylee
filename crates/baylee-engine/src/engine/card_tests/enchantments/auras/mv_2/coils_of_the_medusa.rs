//! `cards/enchantments/auras/mv_2/coils_of_the_medusa.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn coils_of_the_medusa() -> CardIndex {
    card_index("02a8a4e6-2081-4903-81ad-96e5a949dff8")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Coils of the Medusa (02a8a4e6-2081-4903-81ad-96e5a949dff8) — "Enchant
/// creature. Enchanted creature gets +1/-1." Only the enchanted creature is
/// changed, and the bonus goes with the Aura.
#[test]
fn coils_of_the_medusa_gives_only_the_enchanted_creature_plus_one_minus_one() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), savannah_lions()],
        &[],
        &[coils_of_the_medusa()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    let lions = lk::obj(&e, P0, savannah_lions());
    assert_eq!(pt(&e, bears), (2, 2));

    let aura = kit::cast_aura(
        &mut e,
        P0,
        coils_of_the_medusa(),
        bears,
        &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
    );
    assert_eq!(kit::attached_to(&e, aura), Some(bears));
    assert_eq!(pt(&e, bears), (3, 1), "+1/-1");
    assert_eq!(pt(&e, lions), (2, 1), "the other creature is untouched");
}

/// Coils of the Medusa — "Enchant creature" means any creature, and the -1
/// is real: on the opponent's 2/1 it makes a 3/0, which dies, and the Aura
/// goes with it.
#[test]
fn coils_of_the_medusa_on_an_opposing_one_toughness_creature_kills_it() {
    let mut e = lk::game(
        SEED,
        &[],
        &[savannah_lions()],
        &[coils_of_the_medusa()],
        &[],
    );
    let lions = lk::obj(&e, P1, savannah_lions());
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
    );
    lk::cast_at(&mut e, P0, coils_of_the_medusa(), &[lions], &[]);

    assert!(
        in_graveyard(&e, P1, savannah_lions()).is_some(),
        "toughness 0 (CR 704.5f)"
    );
    assert!(
        in_graveyard(&e, P0, coils_of_the_medusa()).is_some(),
        "the Aura falls off with nothing to enchant"
    );
}
