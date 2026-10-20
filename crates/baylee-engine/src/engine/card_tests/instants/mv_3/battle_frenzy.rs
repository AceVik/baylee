//! `cards/instants/mv_3/battle_frenzy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn battle_frenzy() -> CardIndex {
    card_index("0fca46ec-ec1a-4e6c-bb0e-cdb834b38c92")
}

/// Battle Frenzy (80a6dc47-e63f-444c-989f-7357b42b9d8b) — {2}{R} Instant:
/// - "Green creatures you control get +1/+1 until end of turn."
/// - "Nongreen creatures you control get +1/+0 until end of turn."
#[test]
fn battle_frenzy_pumps_green_creatures_you_control_plus_one_plus_one_and_nongreen_plus_one_plus_zero()
 {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                llanowar_elves(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[llanowar_elves(), gray_ogre()])
        .hand(0, &[battle_frenzy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let my_green = on_battlefield(&engine, P0, llanowar_elves()).expect("my green Elf");
    let my_nongreen = on_battlefield(&engine, P0, gray_ogre()).expect("my nongreen Ogre");
    let their_green = on_battlefield(&engine, P1, llanowar_elves()).expect("their green Elf");
    let their_nongreen = on_battlefield(&engine, P1, gray_ogre()).expect("their nongreen Ogre");

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, P0, battle_frenzy());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, my_green),
        (2, 2),
        "green creature you control gets +1/+1 (1/1 -> 2/2)"
    );
    assert_eq!(
        pt(&engine, my_nongreen),
        (3, 2),
        "nongreen creature you control gets +1/+0 (2/2 -> 3/2)"
    );
    assert_eq!(
        pt(&engine, their_green),
        (1, 1),
        "opponent's green creature is unaffected"
    );
    assert_eq!(
        pt(&engine, their_nongreen),
        (2, 2),
        "opponent's nongreen creature is unaffected"
    );

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, my_green), (1, 1), "wears off at end of turn");
    assert_eq!(pt(&engine, my_nongreen), (2, 2), "wears off at end of turn");
}
