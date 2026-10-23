//! `cards/sorceries/mv_2/superior_numbers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn superior_numbers() -> CardIndex {
    card_index("b98177b5-599f-41b0-aa1e-4769d7005bbd")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// P0 has four creatures and two Forests with Superior Numbers in hand, P1
/// the named creatures; the spell is cast at P1's Hill Giant.
fn cast_at_their_giant(theirs: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                grizzly_bears(),
                llanowar_elves(),
                gray_ogre(),
                hill_giant(),
            ],
        )
        .battlefield(1, theirs)
        .hand(0, &[superior_numbers()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    cast_from_hand(&mut engine, p0, superior_numbers());
    aim_at(&mut engine, p0, giant);
    pass_until(&mut engine, stack_is_empty);
    (engine, giant)
}

/// Superior Numbers (`{G}{G}`, `Coverage::Partial`): damage equal to the
/// number of creatures you control in excess of the number the opponent
/// controls. Four against one is 3 damage, which kills a Hill Giant.
#[test]
fn superior_numbers_deals_the_excess_of_your_creatures_over_theirs() {
    let (engine, _giant) = cast_at_their_giant(&[hill_giant()]);
    assert!(
        on_battlefield(&engine, PlayerId::new(1), hill_giant()).is_none(),
        "four creatures against one is 3 damage, lethal to a 3/3"
    );
}

/// Superior Numbers — only the excess counts: four against three is 1 damage,
/// and four against four is none.
#[test]
fn superior_numbers_counts_only_the_creatures_in_excess_of_theirs() {
    let (engine, giant) = cast_at_their_giant(&[hill_giant(), grizzly_bears(), gray_ogre()]);
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        1,
        "four against three is the excess of one"
    );

    let (engine, giant) =
        cast_at_their_giant(&[hill_giant(), grizzly_bears(), gray_ogre(), llanowar_elves()]);
    assert_eq!(
        engine.state().object(giant).unwrap().damage,
        0,
        "equal numbers is no excess"
    );
}
