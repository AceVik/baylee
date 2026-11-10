//! An-Havva Constable — `{1}{G}{G}` 2/1+* Creature — Human (Homelands)
//!
//! An-Havva Constable's toughness is equal to 1 plus the number of green
//! creatures on the battlefield.

#[allow(clippy::wildcard_imports)]
use super::*;

fn an_havva_constable() -> CardIndex {
    card_index("7131cdb6-15d0-407e-a7e2-b2a0deef9363")
}

#[test]
fn an_havva_constable_toughness_counts_all_green_creatures_on_battlefield() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[an_havva_constable(), grizzly_bears()])
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                gray_ogre(), // red creature -> not counted
            ],
        )
        .battlefield(1, &[grizzly_bears()]) // opponent's green creature -> counted
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast An-Havva Constable ({1}{G}{G}).
    let constable_hand = in_hand(&engine, p0, an_havva_constable()).unwrap();
    let bears_hand = in_hand(&engine, p0, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: constable_hand,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let constable = on_battlefield(&engine, p0, an_havva_constable()).unwrap();
    // Green creatures on battlefield: Constable itself + p1's Grizzly Bears = 2 -> toughness 1 + 2 = 3.
    assert_eq!(pt(&engine, constable), (2, 3));

    // Cast p0's Grizzly Bears ({1}{G}) -> 3 green creatures on battlefield -> toughness 1 + 3 = 4.
    engine
        .apply(p0, PlayerAction::CastSpell { card: bears_hand })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, constable), (2, 4));
    let _ = p1;
}
