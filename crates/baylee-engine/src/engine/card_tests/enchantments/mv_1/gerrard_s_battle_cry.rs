//! `cards/enchantments/mv_1/gerrard_s_battle_cry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gerrard_s_battle_cry() -> CardIndex {
    card_index("12ef651c-5a86-49c4-a512-292e9d135a76")
}

/// Gerrard's Battle Cry (`12ef651c-5a86-49c4-a512-292e9d135a76`) — {W} Enchantment:
/// "{2}{W}: Creatures you control get +1/+1 until end of turn."
#[test]
fn gerrard_s_battle_cry_pumps_your_creatures_only() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                gerrard_s_battle_cry(),
                plains(),
                plains(),
                plains(),
                index::GRIZZLY_BEARS,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();

    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, gerrard_s_battle_cry(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (3, 3));
    assert_eq!(pt(&engine, giant), (3, 3));
}

#[test]
fn gerrards_battle_cry_does_not_pump_opponents_and_expires() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let mut e = lk::game(
        SEED,
        &[gerrard_s_battle_cry(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT],
        &[],
        &[],
    );
    let own = lk::obj(&e, lk::P0, index::GRIZZLY_BEARS);
    let other = lk::obj(&e, lk::P1, index::HILL_GIANT);
    lk::float(&mut e, lk::P0, &[(ManaColor::White, 6)]);
    lk::use_ability(&mut e, lk::P0, gerrard_s_battle_cry(), 0, &[], &[]);
    lk::use_ability(&mut e, lk::P0, gerrard_s_battle_cry(), 0, &[], &[]);
    assert_eq!(pt(&e, own), (4, 4));
    assert_eq!(pt(&e, other), (3, 3));
    reach_their_main_phase(&mut e, lk::P1);
    assert_eq!(pt(&e, own), (2, 2));
}
